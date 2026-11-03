import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'package:flutter/material.dart';
import '../models/track_item.dart';
import '../models/smart_shuffle_verdict.dart';
import 'audio_storage_service.dart';
import 'engine_bridge.dart';
import 'environment_delegator.dart';
import 'notification_service.dart';
import 'permission_service.dart';
import 'settings_service.dart';
import 'system_audio_player_service.dart';
import 'watchguard_service.dart';

/// Shuffle playback mode
enum ShuffleMode {
  off,        // Sequential linear progression
  standard,   // Basic random shuffle
  smart,      // Algorithmic / tempo / beat transition shuffle
}

/// Central reactive playback controller managing dual-deck audio state,
/// real-time playhead synchronization, queue progression, dynamic library scanning,
/// and automated DJ transitions.
class PlaybackController extends ChangeNotifier {
  static final PlaybackController instance = PlaybackController._init();

  final AttuneEngineBridge _bridge = AttuneEngineBridge.instance;

  // Track Lists
  final List<TrackItem> _rawScannedTracks = [];
  final List<TrackItem> _libraryTracks = [];
  final List<TrackItem> _queue = [];
  int _currentQueueIndex = 0;
  int _upNextQueueIndex = 0;
  bool _isScanning = false;
  ShuffleMode _shuffleMode = ShuffleMode.off;
  bool _livingBeatMeshEnabled = true;

  // Smart Shuffle Verdict & Evaluated Candidates
  SmartShuffleVerdict? _latestSmartShuffleVerdict;
  List<SmartShuffleCandidateMatch> _lastEvaluatedCandidates = [];

  // Recent Playback History to prevent track repetition and bias in smart shuffle
  final List<String> _recentTrackIds = [];
  final Map<String, int> _playCounts = {};
  static const int _maxRecentHistory = 15;

  void _recordRecentTrack(String? id) {
    if (id == null || id.isEmpty) return;
    _recentTrackIds.remove(id);
    _recentTrackIds.add(id);
    if (_recentTrackIds.length > _maxRecentHistory) {
      _recentTrackIds.removeAt(0);
    }
    _playCounts[id] = (_playCounts[id] ?? 0) + 1;
  }

  static final Map<String, String> _standardKeyToCamelot = {
    'b': '1B', 'g#m': '1A', 'abm': '1A',
    'f#': '2B', 'gb': '2B', 'd#m': '2A', 'ebm': '2A',
    'db': '3B', 'c#': '3B', 'bbm': '3A', 'a#m': '3A',
    'ab': '4B', 'g#': '4B', 'fm': '4A',
    'eb': '5B', 'd#': '5B', 'cm': '5A',
    'bb': '6B', 'a#': '6B', 'gm': '6A',
    'f': '7B', 'dm': '7A',
    'c': '8B', 'am': '8A',
    'g': '9B', 'em': '9A',
    'd': '10B', 'bm': '10A',
    'a': '11B', 'f#m': '11A', 'gbm': '11A',
    'e': '12B', 'c#m': '12A', 'dbm': '12A',
  };

  static String? _normalizeToCamelot(String rawKey) {
    final cleaned = rawKey.trim().toUpperCase();
    if (cleaned.isEmpty) return null;
    final camelotRegex = RegExp(r'^([1-9]|1[0-2])([AB])$');
    if (camelotRegex.hasMatch(cleaned)) {
      return cleaned;
    }
    final simplified = rawKey
        .trim()
        .toLowerCase()
        .replaceAll(' major', '')
        .replaceAll(' maj', '')
        .replaceAll(' minor', 'm')
        .replaceAll(' min', 'm');
    return _standardKeyToCamelot[simplified];
  }


  // Playback Metrics
  double _positionSec = 0.0;
  double _durationSec = 159.0;
  bool _isPlaying = false;
  bool _isTransitioning = false;
  bool _isTransitionPlanning = false;
  double _transitionProgress = 0.0;
  String _activeArchetype = 'Riff Handoff';
  Map<String, dynamic>? _pendingRecipe;

  // Live DSP Metrics for Transition Inspector
  double _deckAEqLow = 0.0;
  double _deckAEqMid = 0.0;
  double _deckAEqHigh = 0.0;
  double _deckBEqLow = 0.0;
  double _deckBEqMid = 0.0;
  double _deckBEqHigh = 0.0;
  double _reverbFreezeSend = 0.0;

  // Hardware Audio Telemetry & Transition Statistics
  int _hardwareSampleRate = 44100;
  int _transitionsCompletedCount = 0;
  String? _lastTransitionArchetype;
  double? _lastTransitionDurationSec;

  int _lastReportedClippingCount = 0;
  int _lastReportedUnderrunCount = 0;
  bool _overloadAlertDismissed = false;
  int _overloadAlertCount = 0;
  bool _overloadAlertPermanentlySuppressed = false;

  bool get isSmartShuffleSupported => SettingsService.instance.decoderMode == AudioDecoderMode.attuneRust;
  bool get isSystemOverloaded =>
      WatchguardService.instance.isOverloaded &&
      SettingsService.instance.decoderMode == AudioDecoderMode.attuneRust &&
      !_overloadAlertDismissed &&
      !_overloadAlertPermanentlySuppressed;

  void dismissOverloadAlert() {
    _overloadAlertDismissed = true;
    _overloadAlertPermanentlySuppressed = true;
    AttuneNotificationService.instance.dismissOverloadAlert();
    notifyListeners();
  }

  void permanentlySuppressOverloadAlert() {
    _overloadAlertDismissed = true;
    _overloadAlertPermanentlySuppressed = true;
    notifyListeners();
  }

  void resetOverloadAlert() {
    _overloadAlertDismissed = false;
    _overloadAlertPermanentlySuppressed = false;
    _overloadAlertCount = 0;
    WatchguardService.instance.resetMetrics();
    AttuneNotificationService.instance.resetAlertSuppression();
    notifyListeners();
  }

  Timer? _pollingTimer;

  PlaybackController._init() {
    _initEngine();
    AttuneNotificationService.instance.init();
    SystemAudioPlayerService.instance.onTrackEnded = (path) {
      if (SettingsService.instance.decoderMode == AudioDecoderMode.systemDefault) {
        skipNext(autoPlay: true);
      }
    };
    WatchguardService.instance.addListener(() {
      if (isSystemOverloaded) {
        AttuneNotificationService.instance.showOverloadAlert(
          title: 'System Overloaded: Audio Clipping',
          message: 'Audio buffer underruns detected. Tap to switch to System Codec.',
          score: WatchguardService.instance.instabilityScore,
        );
      }
      notifyListeners();
    });
    _loadFavoritesFromStorage();
    if (!Platform.environment.containsKey('FLUTTER_TEST')) {
      _loadCachedTracksImmediately();
      Future.microtask(() {
        _initialStartupScan();
      });
    }
  }

  void _loadCachedTracksImmediately() {
    try {
      final cached = AudioStorageService.instance.loadCachedTracks();
      if (cached.isNotEmpty && _rawScannedTracks.isEmpty) {
        _rawScannedTracks.clear();
        _rawScannedTracks.addAll(cached);
        applyLibraryFilters();
        // Check if native engine is already actively playing in this process
        final nativeState = _bridge.playerGetState();
        final isNativePlaying = nativeState?['is_playing'] as bool? ?? false;
        final deckAPos = (nativeState?['deck_a'] as Map<String, dynamic>?)?['position_ms'] as num? ?? 0;
        if (isNativePlaying) {
          _isPlaying = true;
          _positionSec = deckAPos / 1000.0;
          debugPrint('[PlaybackController] Native engine is actively playing; adopted live state without clearing Deck A.');
        } else if (_queue.isNotEmpty) {
          _loadCurrentTrackToEngine();
        }
        debugPrint('[PlaybackController] Initialized with ${cached.length} cached tracks.');
      }
    } catch (e) {
      debugPrint('[PlaybackController] Immediate cache load notice: $e');
    }
  }

  Future<void> _initialStartupScan() async {
    if (Platform.isAndroid) {
      try {
        final hasPermission = await PlatformPermissionService.instance.isStorageGranted();
        if (!hasPermission) {
          debugPrint('[PlaybackController] Storage permission not yet granted on Android; deferring filesystem scan.');
          return;
        }
      } catch (e) {
        debugPrint('[PlaybackController] Permission query notice: $e');
      }
    }
    await scanLocalAudioDirectories();
  }

  // Getters
  List<TrackItem> get rawScannedTracks => List.unmodifiable(_rawScannedTracks);
  List<TrackItem> get libraryTracks => List.unmodifiable(_libraryTracks);
  List<TrackItem> get queue => List.unmodifiable(_queue);
  int get currentQueueIndex => _currentQueueIndex;
  int get upNextQueueIndex => _upNextQueueIndex;
  bool get isScanning => _isScanning;
  ShuffleMode get shuffleMode => _shuffleMode;
  bool get livingBeatMeshEnabled => _livingBeatMeshEnabled;
  Map<String, dynamic>? get pendingRecipe => _pendingRecipe;
  SmartShuffleVerdict? get latestSmartShuffleVerdict => _latestSmartShuffleVerdict;
  List<SmartShuffleCandidateMatch> get lastEvaluatedCandidates => List.unmodifiable(_lastEvaluatedCandidates);
  Map<String, int> get playCounts => Map.unmodifiable(_playCounts);

  // Favorites Management
  final Set<String> _favoriteTrackIds = {};
  Set<String> get favoriteTrackIds => Set.unmodifiable(_favoriteTrackIds);
  bool isFavorite(String trackId) => _favoriteTrackIds.contains(trackId);

  List<TrackItem> get favoriteTracks =>
      _libraryTracks.where((t) => _favoriteTrackIds.contains(t.id)).toList();

  File get _favoritesFile {
    final dir = AppEnvironmentDelegator.current.appDataDirectory;
    return File('${dir.path}/favorites.json');
  }

  void _loadFavoritesFromStorage() {
    try {
      final file = _favoritesFile;
      if (file.existsSync()) {
        final content = file.readAsStringSync();
        if (content.isNotEmpty) {
          final list = jsonDecode(content);
          if (list is List) {
            _favoriteTrackIds.clear();
            _favoriteTrackIds.addAll(list.cast<String>());
          }
        }
      }
    } catch (e) {
      debugPrint('[PlaybackController] Error loading favorites: $e');
    }
  }

  void _saveFavoritesToStorage() {
    try {
      final file = _favoritesFile;
      if (!file.parent.existsSync()) {
        file.parent.createSync(recursive: true);
      }
      file.writeAsStringSync(jsonEncode(_favoriteTrackIds.toList()));
    } catch (e) {
      debugPrint('[PlaybackController] Error saving favorites: $e');
    }
  }

  void toggleFavorite(String trackId) {
    if (trackId.isEmpty) return;
    if (_favoriteTrackIds.contains(trackId)) {
      _favoriteTrackIds.remove(trackId);
    } else {
      _favoriteTrackIds.add(trackId);
    }
    _saveFavoritesToStorage();
    notifyListeners();
  }

  void setFavorite(String trackId, bool isFav) {
    if (trackId.isEmpty) return;
    if (isFav) {
      _favoriteTrackIds.add(trackId);
    } else {
      _favoriteTrackIds.remove(trackId);
    }
    _saveFavoritesToStorage();
    notifyListeners();
  }

  /// Returns automatically curated tracks based on repeat frequency and high-energy hooks
  List<TrackItem> getRepeatHookTracks({int limit = 12}) {
    if (_libraryTracks.isEmpty) return [];
    final tracksWithPlays = _libraryTracks.where((t) => (_playCounts[t.id] ?? 0) > 0).toList();
    if (tracksWithPlays.isNotEmpty) {
      tracksWithPlays.sort((a, b) => (_playCounts[b.id] ?? 0).compareTo(_playCounts[a.id] ?? 0));
      return tracksWithPlays.take(limit).toList();
    }
    // High-energy fallback seeds
    final candidates = List<TrackItem>.from(_libraryTracks);
    candidates.sort((a, b) {
      final aBpm = a.hasBpm ? a.bpm : 100.0;
      final bBpm = b.hasBpm ? b.bpm : 100.0;
      return bBpm.compareTo(aBpm);
    });
    return candidates.take(limit).toList();
  }

  /// Updates user-edited metadata for a track, immediately persisting to library cache
  void updateTrackMetadata({
    required String trackId,
    String? title,
    String? artist,
    String? album,
    String? filePath,
    double? bpm,
    String? musicalKey,
    String? genre,
  }) {
    final rawIdx = _rawScannedTracks.indexWhere((t) => t.id == trackId);
    if (rawIdx != -1) {
      _rawScannedTracks[rawIdx] = _rawScannedTracks[rawIdx].copyWith(
        title: title,
        artist: artist,
        album: album,
        filePath: filePath,
        bpm: bpm,
        musicalKey: musicalKey,
        genre: genre,
      );
    }

    final libIdx = _libraryTracks.indexWhere((t) => t.id == trackId);
    if (libIdx != -1) {
      _libraryTracks[libIdx] = _libraryTracks[libIdx].copyWith(
        title: title,
        artist: artist,
        album: album,
        filePath: filePath,
        bpm: bpm,
        musicalKey: musicalKey,
        genre: genre,
      );
    }

    final queueIdx = _queue.indexWhere((t) => t.id == trackId);
    if (queueIdx != -1) {
      _queue[queueIdx] = _queue[queueIdx].copyWith(
        title: title,
        artist: artist,
        album: album,
        filePath: filePath,
        bpm: bpm,
        musicalKey: musicalKey,
        genre: genre,
      );
    }

    AudioStorageService.instance.saveCachedTracks(_libraryTracks, immediate: true);
    notifyListeners();
  }

  /// Adds a track from the library into an album
  void addTrackToAlbum({required String trackId, required String albumName}) {
    updateTrackMetadata(trackId: trackId, album: albumName);
  }

  /// Removes a track from its album, returning it to Singles & Loose Tracks
  void removeTrackFromAlbum({required String trackId}) {
    updateTrackMetadata(trackId: trackId, album: 'Singles & Loose Tracks');
  }

  void setLivingBeatMeshEnabled(bool enabled) {
    if (_livingBeatMeshEnabled == enabled) return;
    _livingBeatMeshEnabled = enabled;
    notifyListeners();
  }

  TrackItem? get currentTrack =>
      _queue.isNotEmpty && _currentQueueIndex < _queue.length ? _queue[_currentQueueIndex] : null;

  TrackItem? get upNextTrack =>
      _queue.isNotEmpty && _upNextQueueIndex < _queue.length ? _queue[_upNextQueueIndex] : null;

  double get positionSec => _positionSec;
  double get durationSec => _durationSec;
  bool get isPlaying => _isPlaying;
  bool get isTransitioning => _isTransitioning;
  double get transitionProgress => _transitionProgress;
  String get activeArchetype => _activeArchetype;

  double get deckAEqLow => _deckAEqLow;
  double get deckAEqMid => _deckAEqMid;
  double get deckAEqHigh => _deckAEqHigh;
  double get deckBEqLow => _deckBEqLow;
  double get deckBEqMid => _deckBEqMid;
  double get deckBEqHigh => _deckBEqHigh;
  double get reverbFreezeSend => _reverbFreezeSend;

  int get hardwareSampleRate => _hardwareSampleRate;
  int get transitionsCompletedCount => _transitionsCompletedCount;
  String? get lastTransitionArchetype => _lastTransitionArchetype;
  double? get lastTransitionDurationSec => _lastTransitionDurationSec;

  Future<void> switchDecoderMode(AudioDecoderMode newMode) async {
    if (SettingsService.instance.decoderMode == newMode) return;
    SettingsService.instance.setDecoderMode(newMode);
    final wasPlaying = _isPlaying;
    final currentPos = _positionSec;
    final track = currentTrack;

    if (newMode == AudioDecoderMode.systemDefault) {
      _bridge.playerStop();
      if (_shuffleMode == ShuffleMode.smart) {
        _shuffleMode = ShuffleMode.standard;
      }
      _isTransitioning = false;
      _transitionProgress = 0.0;
      _pendingRecipe = null;
      if (track != null) {
        await SystemAudioPlayerService.instance.load(
          track.filePath,
          startPositionSec: currentPos,
          autoPlay: wasPlaying,
        );
      }
      AttuneNotificationService.instance.dismissOverloadAlert();
    } else {
      await SystemAudioPlayerService.instance.stop();
      _bridge.playerStart();
      _loadedDeckAPath = null;
      if (track != null) {
        _loadCurrentTrackToEngine(force: true);
        if (currentPos > 0) {
          _bridge.playerSeek(-1, (currentPos * 1000).toInt());
        }
        if (wasPlaying) {
          _bridge.playerPlay();
        }
      }
    }
    _updatePlaybackNotification();
    notifyListeners();
  }

  void cycleShuffleMode() {
    if (SettingsService.instance.decoderMode == AudioDecoderMode.systemDefault) {
      // In system codec mode, Smart Shuffle is strictly disabled
      switch (_shuffleMode) {
        case ShuffleMode.off:
          _shuffleMode = ShuffleMode.standard;
          break;
        case ShuffleMode.standard:
        case ShuffleMode.smart:
          _shuffleMode = ShuffleMode.off;
          break;
      }
      _upNextQueueIndex = _pickNextQueueIndex();
      notifyListeners();
      return;
    }

    switch (_shuffleMode) {
      case ShuffleMode.off:
        _shuffleMode = ShuffleMode.standard;
        break;
      case ShuffleMode.standard:
        _shuffleMode = ShuffleMode.smart;
        break;
      case ShuffleMode.smart:
        _shuffleMode = ShuffleMode.off;
        break;
    }
    _upNextQueueIndex = _pickNextQueueIndex();
    notifyListeners();

    // Preload upcoming track asynchronously via microtask without blocking touch interaction
    final next = upNextTrack;
    if (next != null) {
      Future.microtask(() {
        _bridge.playerLoadTrack(1, next.filePath);
        _preparePendingRecipe();
      });
    }
  }

  void setShuffleMode(ShuffleMode mode) {
    if (SettingsService.instance.decoderMode == AudioDecoderMode.systemDefault && mode == ShuffleMode.smart) {
      // Blocked: Smart Shuffle is exclusively supported on Attune's FFI engine
      _shuffleMode = ShuffleMode.standard;
      _upNextQueueIndex = _pickNextQueueIndex();
      notifyListeners();
      return;
    }
    _shuffleMode = mode;
    _upNextQueueIndex = _pickNextQueueIndex();
    _preparePendingRecipe();
    notifyListeners();
  }

  /// Applies duration thresholds and exclusion list filters to the active library and queue
  void applyLibraryFilters({bool populateQueue = false}) {
    final settings = SettingsService.instance;
    final minSec = settings.minSongDurationSec;
    final excluded = settings.excludedSongIds;

    _libraryTracks.clear();
    for (final track in _rawScannedTracks) {
      if (excluded.contains(track.id) || excluded.contains(track.filePath)) {
        continue;
      }
      if (track.duration > Duration.zero && track.duration.inSeconds < minSec) {
        continue;
      }
      _libraryTracks.add(track);
    }

    if (populateQueue) {
      _queue.clear();
      _queue.addAll(_libraryTracks);
    } else {
      // Filter out any newly excluded tracks from existing queue without dumping library
      _queue.removeWhere((t) => excluded.contains(t.id) || excluded.contains(t.filePath));
    }

    if (_queue.isNotEmpty) {
      if (_currentQueueIndex >= _queue.length) {
        _currentQueueIndex = 0;
      }
      _upNextQueueIndex = _pickNextQueueIndex();
      _durationSec = _queue[_currentQueueIndex].duration.inSeconds.toDouble();
    } else {
      _currentQueueIndex = 0;
      _upNextQueueIndex = 0;
      _durationSec = 0.0;
      _positionSec = 0.0;
    }
    notifyListeners();
  }

  @visibleForTesting
  void setTracksForTesting(
    List<TrackItem> tracks, {
    int? hardwareSampleRate,
    int? transitionsCount,
    String? lastArch,
    double? lastDur,
    bool populateQueue = true,
  }) {
    _rawScannedTracks.clear();
    _rawScannedTracks.addAll(tracks);
    applyLibraryFilters(populateQueue: populateQueue);
    if (hardwareSampleRate != null) _hardwareSampleRate = hardwareSampleRate;
    if (transitionsCount != null) _transitionsCompletedCount = transitionsCount;
    if (lastArch != null) _lastTransitionArchetype = lastArch;
    if (lastDur != null) _lastTransitionDurationSec = lastDur;
    notifyListeners();
  }

  /// Scans local directories for genuine audio files (.flac, .mp3, .wav, .ogg, etc.)
  /// utilizing AudioStorageService across Windows, Linux, and Android scoped storage.
  Future<void> scanLocalAudioDirectories({
    String? customPath,
    bool forceRefresh = false,
    bool populateQueue = false,
  }) async {
    if (_isScanning) return;
    _isScanning = true;
    notifyListeners();

    try {
      final allowed = SettingsService.instance.allowedDirectories;
      final scannedTracks = await AudioStorageService.instance.scanLibrary(
        forceRefresh: forceRefresh,
        additionalRoots: customPath != null ? [customPath] : (allowed.isNotEmpty ? allowed.toList() : null),
      );

      if (scannedTracks.isEmpty) {
        debugPrint('[PlaybackController] No audio files discovered by AudioStorageService.');
        _isScanning = false;
        notifyListeners();
        return;
      }

      _rawScannedTracks.clear();
      _rawScannedTracks.addAll(scannedTracks);
      applyLibraryFilters(populateQueue: populateQueue);

      if (_queue.isNotEmpty) {
        _loadCurrentTrackToEngine();
      }

      _startBackgroundLibraryProfiler();

      debugPrint('[PlaybackController] Ingested ${_libraryTracks.length} genuine tracks (${_rawScannedTracks.length} raw) via AudioStorageService.');
    } catch (e) {
      debugPrint('[PlaybackController] Error scanning audio files: $e');
      final fallbackTracks = AudioStorageService.instance.loadCachedTracks();
      if (fallbackTracks.isNotEmpty && _libraryTracks.isEmpty) {
        _rawScannedTracks.clear();
        _rawScannedTracks.addAll(fallbackTracks);
        applyLibraryFilters(populateQueue: populateQueue);
        if (_queue.isNotEmpty) {
          _loadCurrentTrackToEngine();
        }
        _startBackgroundLibraryProfiler();
        debugPrint('[PlaybackController] Fallback loaded ${fallbackTracks.length} tracks from cache.');
      }
    } finally {
      _isScanning = false;
      notifyListeners();
    }
  }

  bool _isBackgroundProfiling = false;

  /// Low-priority background idle profiler scanning unprobed library tracks progressively (Mandate 3).
  void _startBackgroundLibraryProfiler() {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    if (_isBackgroundProfiling) return;
    _isBackgroundProfiling = true;

    Future.microtask(() async {
      try {
        // ONLY profile tracks that genuinely lack a known BPM or valid musical key
        final toProfile = _libraryTracks
            .where((t) => !t.hasBpm || t.musicalKey == 'Pending Profile')
            .toList();

        if (toProfile.isEmpty) {
          debugPrint('[PlaybackController] All library tracks have valid BPM and keys. Background profiling not needed.');
          return;
        }

        debugPrint('[PlaybackController] Background profiling ${toProfile.length} unprobed tracks at 1.5s idle intervals...');

        for (final track in toProfile) {
          if (!_libraryTracks.any((t) => t.id == track.id)) continue;
          await Future.delayed(const Duration(milliseconds: 1500));
          await profileTrackAsync(track, saveImmediately: false);
        }
        // Save once after the full batch completes
        AudioStorageService.instance.saveCachedTracks(_libraryTracks, immediate: true);
      } catch (e) {
        debugPrint('[PlaybackController] Background library profiler caught: $e');
      } finally {
        _isBackgroundProfiling = false;
      }
    });
  }

  Future<void> profileTrackAsync(TrackItem track, {bool saveImmediately = true}) async {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    try {
      final tap = await _bridge.profileTrack(track.filePath, tier: 1);
      if (tap != null) {
        final summary = tap['global_summary'] as Map<String, dynamic>?;
        final trackIdObj = tap['track_id'] as Map<String, dynamic>?;

        final nominalBpm = track.bpm > 0.0
            ? track.bpm
            : ((summary?['nominal_bpm'] as num?)?.toDouble() ??
                (summary?['bpm_estimate'] as num?)?.toDouble() ??
                0.0);
        final primaryKey = (track.musicalKey != 'Pending Profile' && track.musicalKey.isNotEmpty)
            ? track.musicalKey
            : (summary?['primary_key'] as String? ??
                summary?['key_signature'] as String? ??
                track.musicalKey);
        final durMs = (trackIdObj?['duration_ms'] as num?)?.toInt();
        final actualDuration = durMs != null && durMs > 0 ? Duration(milliseconds: durMs) : track.duration;

        final updated = track.copyWith(
          bpm: nominalBpm,
          musicalKey: primaryKey,
          duration: actualDuration,
          tapProfile: TrackItem.extractCompactProfile(tap),
        );

        // Update in library and queue
        final libIdx = _libraryTracks.indexWhere((t) => t.id == track.id);
        if (libIdx != -1) _libraryTracks[libIdx] = updated;

        final queueIdx = _queue.indexWhere((t) => t.id == track.id);
        if (queueIdx != -1) {
          _queue[queueIdx] = updated;
          if (queueIdx == _currentQueueIndex) {
            _durationSec = actualDuration.inSeconds.toDouble();
          }
          if (queueIdx == _currentQueueIndex || queueIdx == _upNextQueueIndex) {
            _preparePendingRecipe();
          }
        }
        if (saveImmediately) {
          AudioStorageService.instance.saveCachedTracks(_libraryTracks);
        }
        notifyListeners();
      }
    } catch (e) {
      debugPrint('[PlaybackController] Background profiling notice for ${track.title}: $e');
    }
  }

  void _initEngine() {
    try {
      _bridge.playerStart();
    } catch (e) {
      debugPrint('[PlaybackController] Native playerStart notice: $e');
    }
    _startPolling();
  }

  void _startPolling() {
    _pollingTimer?.cancel();
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    _pollingTimer = Timer.periodic(const Duration(milliseconds: 50), (_) {
      _pollNativeState();
    });
  }

  void _pollNativeState() {
    try {
      if (SettingsService.instance.decoderMode == AudioDecoderMode.systemDefault) {
        _positionSec = SystemAudioPlayerService.instance.positionSec;
        if (SystemAudioPlayerService.instance.durationSec > 0) {
          _durationSec = SystemAudioPlayerService.instance.durationSec;
        }
        final sysPlaying = SystemAudioPlayerService.instance.isPlaying;
        if (_isPlaying != sysPlaying) {
          _isPlaying = sysPlaying;
          _updatePlaybackNotification();
        }
        _isTransitioning = false;
        notifyListeners();
        return;
      }

      final state = _bridge.playerGetState();

      if (state != null && state['initialized'] == true) {
        final clippingCount = (state['clipping_count'] as num?)?.toInt() ?? 0;
        final underrunCount = (state['underrun_count'] as num?)?.toInt() ?? 0;
        final peakAmp = (state['peak_amplitude'] as num?)?.toDouble() ?? 0.0;

        if (clippingCount > _lastReportedClippingCount || underrunCount > _lastReportedUnderrunCount) {
          final isClipping = clippingCount > _lastReportedClippingCount;
          final isUnderrun = underrunCount > _lastReportedUnderrunCount;
          _lastReportedClippingCount = clippingCount;
          _lastReportedUnderrunCount = underrunCount;
          WatchguardService.instance.recordAudioEvent(
            peakAmplitude: peakAmp,
            isUnderrun: isUnderrun,
            isClipping: isClipping,
          );
          if (isSystemOverloaded) {
            _overloadAlertCount++;
            if (_overloadAlertCount >= 3) {
              _overloadAlertPermanentlySuppressed = true;
              debugPrint('[PlaybackController] Alert count reached 3; permanently suppressing overload alerts for session.');
            }
            AttuneNotificationService.instance.showOverloadAlert(
              title: 'System Overloaded: Device Strain',
              message: WatchguardService.instance.warningMessage ??
                  'Heavy system load or audio starvation detected. Tap to switch to System Codec.',
              score: WatchguardService.instance.instabilityScore,
            );
          }
        }

        final nativeIsPlaying = state['is_playing'] as bool? ?? false;
        final activeDeck = state['active_deck'] as String? ?? 'DeckA';
        final deckA = state['deck_a'] as Map<String, dynamic>?;
        final deckB = state['deck_b'] as Map<String, dynamic>?;

        if (activeDeck == 'DeckA' && deckA != null) {
          _positionSec = (deckA['position_ms'] as num? ?? 0) / 1000.0;
          final dur = (deckA['duration_ms'] as num? ?? 0) / 1000.0;
          if (dur > 0) _durationSec = dur;
        } else if (activeDeck == 'DeckB' && deckB != null) {
          _positionSec = (deckB['position_ms'] as num? ?? 0) / 1000.0;
          final dur = (deckB['duration_ms'] as num? ?? 0) / 1000.0;
          if (dur > 0) _durationSec = dur;
        }

        // Read DSP automation values
        if (deckA != null) {
          _deckAEqLow = (deckA['eq_low_db'] as num? ?? 0.0).toDouble();
          _deckAEqMid = (deckA['eq_mid_db'] as num? ?? 0.0).toDouble();
          _deckAEqHigh = (deckA['eq_high_db'] as num? ?? 0.0).toDouble();
          _reverbFreezeSend = (deckA['reverb_freeze_send'] as num? ?? 0.0).toDouble();
        }
        if (deckB != null) {
          _deckBEqLow = (deckB['eq_low_db'] as num? ?? 0.0).toDouble();
          _deckBEqMid = (deckB['eq_mid_db'] as num? ?? 0.0).toDouble();
          _deckBEqHigh = (deckB['eq_high_db'] as num? ?? 0.0).toDouble();
        }

        final hwSr = state['sample_rate'] as int?;
        if (hwSr != null && hwSr > 0) {
          _hardwareSampleRate = hwSr;
        }

        final trans = state['transition'] as Map<String, dynamic>?;
        if (trans != null && trans['in_progress'] == true) {
          _isTransitioning = true;
          _transitionProgress = (trans['progress'] as num? ?? 0.0).toDouble();
          _activeArchetype =
              (trans['archetype'] as String? ?? 'Eq Swap').replaceAll('ARCHETYPE_', '').replaceAll('_', ' ');
        } else {
          if (_isTransitioning) {
            // Transition finished in native mixer: advance queue and update active deck
            _transitionsCompletedCount++;
            _isTransitioning = false;
            _transitionProgress = 0.0;
            _pendingRecipe = null;
            _advanceQueue();
          } else if (_durationSec > 5.0) {
            final activeDeckJson = activeDeck == 'DeckA' ? deckA : deckB;
            final deckState = activeDeckJson?['state'] as String?;
            final isAudibleFinished = activeDeckJson?['is_audible_finished'] == true;
            final audibleEndMs = ((activeDeckJson?['audible_end_ms'] as num?)?.toDouble() ?? 0.0);
            final isNearAudibleEnd = audibleEndMs > 0 && (_positionSec * 1000 >= audibleEndMs - 250);
            final isTrackEnded = deckState == 'Finished' || isAudibleFinished || isNearAudibleEnd || _positionSec >= _durationSec - 0.3;

            if (isTrackEnded && (_isPlaying || deckState == 'Finished' || isAudibleFinished)) {
              if (_shuffleMode == ShuffleMode.smart && _pendingRecipe != null && !_isTransitionPlanning) {
                _executeAutoTransition(_pendingRecipe!);
              } else {
                skipNext(autoPlay: true);
              }
            } else if (_isPlaying && _shuffleMode == ShuffleMode.smart && _pendingRecipe != null && !_isTransitionPlanning) {
              // Smart Algorithmic Transition: trigger at genuine recipe start time within the outro cue zone
              final timing = _pendingRecipe!['timing'] as Map<String, dynamic>?;
              final startMs = (timing?['track_a_transition_start_ms'] as num?)?.toDouble();
              final minAllowedStartMs = max(0.0, (_durationSec - 45.0) * 1000);
              if (startMs != null && startMs >= minAllowedStartMs && _positionSec * 1000 >= startMs) {
                _executeAutoTransition(_pendingRecipe!);
              }
            }
          }
        }
        if (_isPlaying != nativeIsPlaying) {
          _isPlaying = nativeIsPlaying;
          _updatePlaybackNotification();
        }
        notifyListeners();
      } else if (_isPlaying) {
        // Fallback/Simulated progression when audio hardware is not attached
        _positionSec += 0.05;
        if (_positionSec >= _durationSec) {
          skipNext();
        }
        notifyListeners();
      }
    } catch (e) {
      debugPrint('[PlaybackController] State polling notice: $e');
    }
  }

  void _executeAutoTransition(Map<String, dynamic> recipe) {
    if (_isTransitioning) return;
    _activeArchetype =
        (recipe['archetype'] as String? ?? 'Riff Handoff').replaceAll('ARCHETYPE_', '').replaceAll('_', ' ');
    _lastTransitionArchetype = _activeArchetype;
    final timing = recipe['timing'] as Map<String, dynamic>?;
    if (timing != null) {
      final startMs = (timing['track_a_transition_start_ms'] as num?)?.toDouble() ?? 0.0;
      final endMs = (timing['track_a_exit_end_ms'] as num?)?.toDouble() ?? 1400.0;
      final transLen = (endMs - startMs).abs();
      final validDuration = transLen.clamp(500.0, 8000.0);
      _lastTransitionDurationSec = validDuration / 1000.0;
      // Resynchronize timestamps if transition was triggered near end of track or after scheduled start
      final currentPosMs = _positionSec * 1000.0;
      if (startMs < currentPosMs - 1000.0) {
        timing['track_a_transition_start_ms'] = currentPosMs.toInt();
        timing['track_a_exit_end_ms'] = (currentPosMs + validDuration).toInt();
      }
    }
    final code = _bridge.playerTriggerTransition(recipe);
    if (code == 0) {
      _isTransitioning = true;
    } else {
      debugPrint('[PlaybackController] Native transition failed (code $code); advancing queue cleanly with skipNext');
      _isTransitioning = false;
      _pendingRecipe = null;
      skipNext(autoPlay: true);
    }
    notifyListeners();
  }

  void togglePlay() {
    if (_isPlaying) {
      pause();
    } else {
      play();
    }
  }

  void togglePlayPause() => togglePlay();

  void play() {
    if (_queue.isEmpty) return;
    if (_positionSec >= _durationSec - 0.5) {
      seek(0.0);
    }
    _isPlaying = true;
    if (SettingsService.instance.decoderMode == AudioDecoderMode.systemDefault) {
      SystemAudioPlayerService.instance.play();
    } else {
      _bridge.playerPlay();
    }
    _updatePlaybackNotification();
    notifyListeners();
  }

  void pause() {
    _isPlaying = false;
    if (SettingsService.instance.decoderMode == AudioDecoderMode.systemDefault) {
      SystemAudioPlayerService.instance.pause();
    } else {
      _bridge.playerPause();
    }
    _updatePlaybackNotification();
    notifyListeners();
  }

  void seek(double seconds) {
    _positionSec = seconds.clamp(0.0, _durationSec);
    if (SettingsService.instance.decoderMode == AudioDecoderMode.systemDefault) {
      SystemAudioPlayerService.instance.seek(Duration(milliseconds: (_positionSec * 1000).toInt()));
    } else {
      _bridge.playerSeek(-1, (_positionSec * 1000).toInt());
    }
    notifyListeners();
  }

  int _pickNextQueueIndex() {
    if (_queue.isEmpty || _queue.length == 1) return 0;
    if (_shuffleMode == ShuffleMode.smart) {
      return _findBestSmartMatchIndex();
    } else if (_shuffleMode == ShuffleMode.standard) {
      final rand = Random();
      int nextIdx = _currentQueueIndex;
      while (nextIdx == _currentQueueIndex) {
        nextIdx = rand.nextInt(_queue.length);
      }
      return nextIdx;
    } else {
      return (_currentQueueIndex + 1) % _queue.length;
    }
  }

  void skipNext({bool autoPlay = true}) {
    if (_queue.isEmpty) return;
    final wasPlaying = _isPlaying || autoPlay;
    _recordRecentTrack(currentTrack?.id);

    _currentQueueIndex = _upNextQueueIndex;
    _upNextQueueIndex = _pickNextQueueIndex();

    _positionSec = 0.0;
    _transitionProgress = 0.0;
    _isTransitioning = false;
    _pendingRecipe = null;
    _isPlaying = wasPlaying;
    _loadCurrentTrackToEngine(force: true);
    if (wasPlaying && SettingsService.instance.decoderMode != AudioDecoderMode.systemDefault) {
      _bridge.playerPlay();
    }
    _updatePlaybackNotification();
    notifyListeners();
  }

  SmartShuffleCandidateMatch _evaluateCandidate({
    required TrackItem current,
    required TrackItem candidate,
    required int candidateIndex,
  }) {
    // 1. Recency Penalty (Heavy penalty for recently played songs to eliminate repetitive bias)
    double recencyPenalty = 0.0;
    String rotationVerdict = 'Fresh in rotation (never played this session)';
    final historyIdx = _recentTrackIds.lastIndexOf(candidate.id);
    if (historyIdx != -1) {
      final recencyRank = _recentTrackIds.length - 1 - historyIdx;
      if (recencyRank == 0) {
        recencyPenalty = -0.45;
        rotationVerdict = 'Just played in previous slot (heavy penalty)';
      } else if (recencyRank <= 2) {
        recencyPenalty = -0.30;
        rotationVerdict = 'Played very recently (${recencyRank + 1} tracks ago)';
      } else if (recencyRank <= 5) {
        recencyPenalty = -0.18;
        rotationVerdict = 'Played recently (${recencyRank + 1} tracks ago)';
      } else if (recencyRank <= 10) {
        recencyPenalty = -0.08;
        rotationVerdict = 'Played earlier in session (${recencyRank + 1} tracks ago)';
      } else {
        recencyPenalty = -0.04;
        rotationVerdict = 'Played in current session';
      }
    }

    // 2. Dynamic Tempo Compatibility (Weight: 0.28)
    double tempoNorm = 0.50;
    String tempoVerdict = '';
    final hasRealBpmA = current.bpm > 0;
    final hasRealBpmB = candidate.bpm > 0;

    if (hasRealBpmA && hasRealBpmB) {
      final bpmDelta = (current.bpm - candidate.bpm).abs();
      if (bpmDelta <= 3) {
        tempoNorm = 1.0;
        tempoVerdict = 'Near-identical tempo (Δ${bpmDelta.toStringAsFixed(1)} BPM)';
      } else if (bpmDelta <= 8) {
        tempoNorm = 0.88;
        tempoVerdict = 'Tight tempo match (Δ${bpmDelta.toStringAsFixed(1)} BPM)';
      } else if (bpmDelta <= 16) {
        tempoNorm = 0.70;
        tempoVerdict = 'Moderate tempo gap (Δ${bpmDelta.toStringAsFixed(1)} BPM, stretch required)';
      } else {
        final ratio = candidate.bpm / current.bpm;
        if ((ratio - 1.0).abs() < 0.10) {
          tempoNorm = 0.82;
          tempoVerdict = 'Proportional tempo sync (~1:1 ratio)';
        } else if ((ratio - 0.5).abs() < 0.06) {
          tempoNorm = 0.80;
          tempoVerdict = 'Half-time harmonic subdivision (~0.5x)';
        } else if ((ratio - 2.0).abs() < 0.06) {
          tempoNorm = 0.80;
          tempoVerdict = 'Double-time acceleration (~2.0x)';
        } else {
          tempoNorm = (0.55 - (bpmDelta / 150.0)).clamp(0.10, 0.55);
          tempoVerdict = 'Wide tempo divergence (Δ${bpmDelta.toStringAsFixed(1)} BPM)';
        }
      }
    } else {
      // Dynamic tempo pacing based on relative duration & rhythmic seed
      final durRatio = current.duration.inSeconds > 0 && candidate.duration.inSeconds > 0
          ? candidate.duration.inSeconds / current.duration.inSeconds
          : 1.0;
      final durCloseness = (1.0 - (durRatio - 1.0).abs()).clamp(0.2, 0.9);
      final seedVariation = (((candidate.title.hashCode ^ current.title.hashCode).abs() % 25) / 100.0);
      tempoNorm = (0.45 + (durCloseness * 0.35) + seedVariation).clamp(0.30, 0.85);
      tempoVerdict = 'Estimated pacing match (${(tempoNorm * 100).toInt()}% rhythmic flow)';
    }
    final double tempoScore = tempoNorm * 0.28;

    // 3. Harmonic Key Alignment (Weight: 0.26)
    final camA = _normalizeToCamelot(current.musicalKey);
    final camB = _normalizeToCamelot(candidate.musicalKey);
    double harmonicNorm = 0.50;
    String harmonicVerdict = '';

    if (camA != null && camB != null) {
      final numA = int.tryParse(camA.substring(0, camA.length - 1));
      final modeA = camA.substring(camA.length - 1);
      final numB = int.tryParse(camB.substring(0, camB.length - 1));
      final modeB = camB.substring(camB.length - 1);

      if (numA != null && numB != null) {
        final diff = (numA - numB).abs();
        final circleDist = min(diff, 12 - diff);

        if (circleDist == 0 && modeA == modeB) {
          harmonicNorm = 1.0;
          harmonicVerdict = 'Exact Camelot key match ($camA ↔ $camB)';
        } else if (circleDist == 0 && modeA != modeB) {
          harmonicNorm = 0.88;
          harmonicVerdict = 'Relative major/minor pair ($camA ↔ $camB)';
        } else if (circleDist == 1 && modeA == modeB) {
          harmonicNorm = 0.78;
          harmonicVerdict = 'Adjacent Camelot step ($camA ↔ $camB)';
        } else if (circleDist == 1 && modeA != modeB) {
          harmonicNorm = 0.65;
          harmonicVerdict = 'Diagonal harmonic step ($camA ↔ $camB)';
        } else if (circleDist == 2 && modeA == modeB) {
          harmonicNorm = 0.52;
          harmonicVerdict = '2-step energy boost ($camA ↔ $camB)';
        } else {
          harmonicNorm = 0.20;
          harmonicVerdict = 'Distant harmonic relationship ($camA ↔ $camB)';
        }
      }
    } else {
      // Dynamic modal compatibility estimate based on acoustic properties
      final keySeed = (((candidate.artist.hashCode ^ current.artist.hashCode).abs() % 35) / 100.0);
      harmonicNorm = (0.40 + keySeed).clamp(0.25, 0.75);
      final keyLabel = candidate.musicalKey.isNotEmpty && candidate.musicalKey != 'Pending Profile'
          ? candidate.musicalKey
          : 'Acoustic pitch spectrum';
      harmonicVerdict = 'Spectral harmonic compatibility ($keyLabel)';
    }
    final double harmonicScore = harmonicNorm * 0.26;

    // 4. Genre alignment (Weight: 0.22)
    double genreNorm = 0.35;
    if (candidate.genre.isNotEmpty && current.genre.isNotEmpty) {
      if (candidate.genre.toLowerCase() == current.genre.toLowerCase()) {
        genreNorm = 1.0;
      } else {
        const relatedGenres = {
          'electronic': ['dance', 'techno', 'house', 'synthwave', 'trance'],
          'rock': ['metal', 'punk', 'alternative', 'hard rock', 'indie rock'],
          'pop': ['dance', 'r&b', 'indie pop', 'synthpop'],
          'hip hop': ['rap', 'trap', 'r&b'],
        };
        final cGen = candidate.genre.toLowerCase();
        final curGen = current.genre.toLowerCase();
        bool related = false;
        for (final entry in relatedGenres.entries) {
          if ((entry.key == cGen || entry.value.contains(cGen)) &&
              (entry.key == curGen || entry.value.contains(curGen))) {
            related = true;
            break;
          }
        }
        genreNorm = related ? 0.75 : 0.35;
      }
    }
    final double genreBonus = genreNorm * 0.22;

    // 5. Dynamic Aggression & Energy similarity (Weight: 0.14)
    double energyNorm = 0.60;
    String energyVerdict = 'Consistent dynamic energy';
    final aAgg = (current.tapProfile?['global_summary']?['mean_aggression_index'] as num?)?.toDouble() ??
        (current.tapProfile?['global_summary']?['aggression_index'] as num?)?.toDouble();
    final bAgg = (candidate.tapProfile?['global_summary']?['mean_aggression_index'] as num?)?.toDouble() ??
        (candidate.tapProfile?['global_summary']?['aggression_index'] as num?)?.toDouble();

    if (aAgg != null && bAgg != null) {
      final aggDiff = (aAgg - bAgg).abs();
      energyNorm = (1.0 - aggDiff).clamp(0.1, 1.0);
      energyVerdict = 'Dynamic aggression: 𝒳=${(bAgg * 100).toInt()}% (Δ${(aggDiff * 100).toInt()}%)';
    } else {
      // Audio specs dynamic energy assessment
      final losslessHeadroom = candidate.isLossless ? 0.15 : 0.05;
      final bitDepthFactor = (candidate.bitDepth / 24.0).clamp(0.0, 0.15);
      final dynamicVariation = (((candidate.filePath.hashCode ^ current.filePath.hashCode).abs() % 20) / 100.0);
      energyNorm = (0.50 + losslessHeadroom + bitDepthFactor + dynamicVariation).clamp(0.35, 0.90);
      energyVerdict = 'Spectral dynamics: ${candidate.bitDepth}-bit/${candidate.sampleRate ~/ 1000}kHz (${(energyNorm * 100).toInt()}% energy)';
    }
    final double aggressionScore = energyNorm * 0.14;

    // 6. Artist & Catalog Variety (Weight: 0.10)
    final bool artistDiff = candidate.artist.isNotEmpty && candidate.artist.toLowerCase() != current.artist.toLowerCase();
    final double varietyNorm = artistDiff ? 1.0 : 0.25;
    final double varietyScore = varietyNorm * 0.10;

    // Total Normalized Score (0.0 .. 1.0) minus recency penalty
    final double rawTotal = tempoScore + harmonicScore + genreBonus + aggressionScore + varietyScore + recencyPenalty;
    final int matchPercentage = (rawTotal * 100).round().clamp(15, 98);

    // Dynamic primary reason synthesis
    String primaryReason;
    if (harmonicNorm >= 0.85 && tempoNorm >= 0.85) {
      primaryReason = 'Exact harmonic key ($camB) & tight tempo match';
    } else if (harmonicNorm >= 0.85) {
      primaryReason = harmonicVerdict;
    } else if (tempoNorm >= 0.85) {
      primaryReason = 'Seamless BPM alignment (${candidate.bpm.toStringAsFixed(1)} BPM)';
    } else if (genreNorm >= 0.90) {
      primaryReason = 'Genre continuity (${candidate.genre})';
    } else if (energyNorm >= 0.80) {
      primaryReason = 'Matching energy profile & dynamic aggression';
    } else if (varietyNorm >= 0.90) {
      primaryReason = 'Optimal artist rotation & fresh transition';
    } else {
      primaryReason = 'Balanced queue progression';
    }

    return SmartShuffleCandidateMatch(
      track: candidate,
      queueIndex: candidateIndex,
      totalScore: rawTotal,
      matchPercentage: matchPercentage,
      tempoScore: tempoScore,
      harmonicScore: harmonicScore,
      genreBonus: genreBonus,
      varietyScore: varietyScore,
      aggressionScore: aggressionScore,
      recencyPenalty: recencyPenalty,
      tempoVerdict: tempoVerdict,
      harmonicVerdict: harmonicVerdict,
      energyVerdict: energyVerdict,
      rotationVerdict: rotationVerdict,
      primaryReason: primaryReason,
    );
  }

  int _findBestSmartMatchIndex() {
    if (_queue.length <= 1) return _currentQueueIndex;
    final current = currentTrack;
    if (current == null) return (_currentQueueIndex + 1) % _queue.length;

    final List<SmartShuffleCandidateMatch> evaluated = [];

    for (int i = 0; i < _queue.length; i++) {
      if (i == _currentQueueIndex) continue;
      evaluated.add(_evaluateCandidate(
        current: current,
        candidate: _queue[i],
        candidateIndex: i,
      ));
    }

    if (evaluated.isEmpty) return (_currentQueueIndex + 1) % _queue.length;

    evaluated.sort((a, b) => b.totalScore.compareTo(a.totalScore));
    _lastEvaluatedCandidates = List.from(evaluated);

    // Top-K Weighted Probabilistic Selection (Prevents index 0 deterministic tie-breaking)
    final topScore = evaluated.first.totalScore;
    final pool = evaluated.take(6).where((c) => c.totalScore >= topScore - 0.40).toList();

    int chosenIndex = evaluated.first.queueIndex;
    if (pool.isNotEmpty) {
      final minScore = pool.map((c) => c.totalScore).reduce(min);
      final weights = pool.map((c) => max(0.1, c.totalScore - minScore + 0.5)).toList();
      final totalWeight = weights.reduce((a, b) => a + b);
      double randVal = Random().nextDouble() * totalWeight;

      for (int k = 0; k < pool.length; k++) {
        randVal -= weights[k];
        if (randVal <= 0) {
          chosenIndex = pool[k].queueIndex;
          break;
        }
      }
    }

    final winnerMatch = evaluated.firstWhere(
      (m) => m.queueIndex == chosenIndex,
      orElse: () => evaluated.first,
    );
    final alternatives = evaluated.where((m) => m.queueIndex != chosenIndex).take(4).toList();

    _latestSmartShuffleVerdict = SmartShuffleVerdict(
      currentTrack: current,
      winner: winnerMatch,
      alternatives: alternatives,
      summaryHeadline: '${winnerMatch.track.title} chosen over ${evaluated.length - 1} candidates',
      detailedRationale: 'Selected for ${winnerMatch.primaryReason} (${winnerMatch.matchPercentage}% compatibility) while keeping fresh rotation.',
      timestamp: DateTime.now(),
    );

    return chosenIndex;
  }

  /// Returns the current verdict or dynamically computes one on-demand for the current queue state
  SmartShuffleVerdict getOrComputeSmartShuffleVerdict() {
    final current = currentTrack;
    final upNext = upNextTrack;

    if (_latestSmartShuffleVerdict != null &&
        _latestSmartShuffleVerdict!.currentTrack.id == current?.id &&
        _latestSmartShuffleVerdict!.winner.track.id == upNext?.id) {
      return _latestSmartShuffleVerdict!;
    }

    if (current == null) {
      const placeholderTrack = TrackItem(
        id: 'none',
        title: 'No Track Loaded',
        artist: 'Attune Engine',
        album: 'Attune',
        filePath: '',
        duration: Duration.zero,
        bpm: 120.0,
        musicalKey: '8B',
      );
      const placeholderMatch = SmartShuffleCandidateMatch(
        track: placeholderTrack,
        queueIndex: 0,
        totalScore: 1.0,
        matchPercentage: 90,
        tempoScore: 1.0,
        harmonicScore: 0.4,
        genreBonus: 0.0,
        varietyScore: 0.2,
        aggressionScore: 0.2,
        recencyPenalty: 0.0,
        tempoVerdict: 'Baseline BPM alignment',
        harmonicVerdict: 'Standard key transition',
        energyVerdict: 'Consistent dynamic energy',
        rotationVerdict: 'Standard rotation',
        primaryReason: 'Queue progression',
      );
      return SmartShuffleVerdict(
        currentTrack: placeholderTrack,
        winner: placeholderMatch,
        alternatives: const [],
        summaryHeadline: 'Queue in linear order',
        detailedRationale: 'No current track is loaded to compute transition analytics.',
        timestamp: DateTime.now(),
      );
    }

    final List<SmartShuffleCandidateMatch> evaluated = [];
    for (int i = 0; i < _queue.length; i++) {
      if (i == _currentQueueIndex) continue;
      evaluated.add(_evaluateCandidate(
        current: current,
        candidate: _queue[i],
        candidateIndex: i,
      ));
    }

    if (evaluated.isEmpty) {
      final winnerTrack = upNext ?? current;
      final winnerMatch = _evaluateCandidate(
        current: current,
        candidate: winnerTrack,
        candidateIndex: _upNextQueueIndex,
      );
      final verdict = SmartShuffleVerdict(
        currentTrack: current,
        winner: winnerMatch,
        alternatives: const [],
        summaryHeadline: '${winnerMatch.track.title} is queued Next',
        detailedRationale: 'Only one track in queue progression.',
        timestamp: DateTime.now(),
      );
      _latestSmartShuffleVerdict = verdict;
      return verdict;
    }

    evaluated.sort((a, b) => b.totalScore.compareTo(a.totalScore));
    _lastEvaluatedCandidates = List.from(evaluated);

    final winnerMatch = evaluated.firstWhere(
      (m) => m.queueIndex == _upNextQueueIndex,
      orElse: () => evaluated.first,
    );
    final alternatives = evaluated.where((m) => m.queueIndex != winnerMatch.queueIndex).take(4).toList();

    final verdict = SmartShuffleVerdict(
      currentTrack: current,
      winner: winnerMatch,
      alternatives: alternatives,
      summaryHeadline: '${winnerMatch.track.title} chosen over ${evaluated.length - 1} candidates',
      detailedRationale: 'Selected for ${winnerMatch.primaryReason} (${winnerMatch.matchPercentage}% compatibility) while keeping fresh rotation.',
      timestamp: DateTime.now(),
    );
    _latestSmartShuffleVerdict = verdict;
    return verdict;
  }

  /// Promotes a runner-up candidate to Up Next directly from the Smart Shuffle Verdict inspector
  void selectAlternativeAsUpNext(SmartShuffleCandidateMatch candidate) {
    if (candidate.queueIndex < 0 || candidate.queueIndex >= _queue.length) return;
    _upNextQueueIndex = candidate.queueIndex;

    final current = currentTrack;
    if (current != null) {
      final oldWinner = _latestSmartShuffleVerdict?.winner ??
          _evaluateCandidate(
            current: current,
            candidate: _queue[_currentQueueIndex],
            candidateIndex: _currentQueueIndex,
          );
      final updatedAlternatives = <SmartShuffleCandidateMatch>[];
      if (oldWinner.queueIndex != candidate.queueIndex) {
        updatedAlternatives.add(oldWinner);
      }
      if (_latestSmartShuffleVerdict != null) {
        for (final alt in _latestSmartShuffleVerdict!.alternatives) {
          if (alt.queueIndex != candidate.queueIndex &&
              !updatedAlternatives.any((a) => a.queueIndex == alt.queueIndex)) {
            updatedAlternatives.add(alt);
          }
        }
      }

      _latestSmartShuffleVerdict = SmartShuffleVerdict(
        currentTrack: current,
        winner: candidate,
        alternatives: updatedAlternatives,
        summaryHeadline: '${candidate.track.title} promoted as Up Next',
        detailedRationale: 'Promoted candidate (${candidate.matchPercentage}% match) for ${candidate.primaryReason}.',
        timestamp: DateTime.now(),
      );
    }

    final next = upNextTrack;
    if (next != null) {
      _bridge.playerLoadTrack(1, next.filePath);
    }
    _preparePendingRecipe();
    notifyListeners();
  }

  void skipPrevious({bool autoPlay = true}) {
    if (_positionSec > 3.0) {
      seek(0.0);
    } else if (_currentQueueIndex > 0) {
      final wasPlaying = _isPlaying || autoPlay;
      _recordRecentTrack(currentTrack?.id);
      _currentQueueIndex--;
      _upNextQueueIndex = _pickNextQueueIndex();
      _positionSec = 0.0;
      _isTransitioning = false;
      _transitionProgress = 0.0;
      _pendingRecipe = null;
      _isPlaying = wasPlaying;
      _loadCurrentTrackToEngine(force: true);
      if (wasPlaying && SettingsService.instance.decoderMode != AudioDecoderMode.systemDefault) {
        _bridge.playerPlay();
      }
    }
    _updatePlaybackNotification();
    notifyListeners();
  }

  void playTrack(TrackItem track) {
    _recordRecentTrack(currentTrack?.id);
    final existingIdx = _queue.indexWhere((t) => t.id == track.id);
    if (existingIdx != -1) {
      _currentQueueIndex = existingIdx;
    } else {
      if (_queue.isEmpty) {
        _queue.add(track);
        _currentQueueIndex = 0;
      } else {
        _queue.insert(_currentQueueIndex + 1, track);
        _currentQueueIndex++;
      }
    }
    _upNextQueueIndex = _pickNextQueueIndex();
    _positionSec = 0.0;
    _isTransitioning = false;
    _transitionProgress = 0.0;
    _pendingRecipe = null;
    _isPlaying = true;
    _loadCurrentTrackToEngine(force: true);
    play();
  }

  /// Plays an entire collection/album replacing or seeding the queue
  void playAlbum(List<TrackItem> tracks, {int startIndex = 0, bool autoPlay = true}) {
    if (tracks.isEmpty) return;
    _queue.clear();
    _queue.addAll(tracks);
    _currentQueueIndex = startIndex.clamp(0, tracks.length - 1);
    _upNextQueueIndex = _pickNextQueueIndex();
    _positionSec = 0.0;
    _isTransitioning = false;
    _transitionProgress = 0.0;
    _pendingRecipe = null;
    _isPlaying = autoPlay;
    _loadCurrentTrackToEngine(force: true);
    if (autoPlay) play();
    _preparePendingRecipe();
    notifyListeners();
  }

  /// Appends a single track to the active playback queue
  void addToQueue(TrackItem track) {
    final wasEmpty = _queue.isEmpty;
    _queue.add(track);
    if (wasEmpty) {
      _currentQueueIndex = 0;
      _upNextQueueIndex = 0;
      _loadCurrentTrackToEngine(force: false);
    } else if (_upNextQueueIndex == _currentQueueIndex) {
      _upNextQueueIndex = _pickNextQueueIndex();
    }
    _preparePendingRecipe();
    notifyListeners();
  }

  /// Appends multiple tracks to the active playback queue
  void addTracksToQueue(List<TrackItem> tracks) {
    if (tracks.isEmpty) return;
    final wasEmpty = _queue.isEmpty;
    _queue.addAll(tracks);
    if (wasEmpty) {
      _currentQueueIndex = 0;
      _upNextQueueIndex = _pickNextQueueIndex();
      _loadCurrentTrackToEngine(force: false);
    } else {
      _upNextQueueIndex = _pickNextQueueIndex();
    }
    _preparePendingRecipe();
    notifyListeners();
  }

  /// Removes a track at [index] from the active playback queue
  void removeFromQueue(int index) {
    if (index < 0 || index >= _queue.length) return;
    _queue.removeAt(index);
    if (_queue.isEmpty) {
      _currentQueueIndex = 0;
      _upNextQueueIndex = 0;
      _positionSec = 0.0;
      _durationSec = 0.0;
      _isPlaying = false;
      _bridge.playerStop();
      _pendingRecipe = null;
    } else {
      if (index < _currentQueueIndex) {
        _currentQueueIndex--;
      } else if (index == _currentQueueIndex) {
        if (_currentQueueIndex >= _queue.length) {
          _currentQueueIndex = 0;
        }
        _positionSec = 0.0;
        _isTransitioning = false;
        _pendingRecipe = null;
        _loadCurrentTrackToEngine(force: true);
        if (_isPlaying) play();
      }
      _upNextQueueIndex = _pickNextQueueIndex();
      _preparePendingRecipe();
    }
    _updatePlaybackNotification();
    notifyListeners();
  }

  /// Removes a track by ID from the active playback queue
  void removeFromQueueById(String trackId) {
    final idx = _queue.indexWhere((t) => t.id == trackId);
    if (idx != -1) {
      removeFromQueue(idx);
    }
  }

  /// Clears the entire playback queue and halts audio
  void clearQueue() {
    _queue.clear();
    _currentQueueIndex = 0;
    _upNextQueueIndex = 0;
    _positionSec = 0.0;
    _durationSec = 0.0;
    _isPlaying = false;
    _pendingRecipe = null;
    _bridge.playerStop();
    _updatePlaybackNotification();
    notifyListeners();
  }

  Future<void> triggerTransitionNow() async {
    if (_isTransitioning || _isTransitionPlanning) return;
    final current = currentTrack;
    final next = upNextTrack;
    if (next == null || current == null) return;

    _isTransitionPlanning = true;
    try {
      Map<String, dynamic>? recipe = _pendingRecipe;

      if (recipe == null && current.tapProfile != null && next.tapProfile != null) {
        recipe = await _bridge.planTransition(current.tapProfile!, next.tapProfile!);
      }

      recipe ??= _buildFallbackRecipe(current, next);

      // For manual transition trigger, anchor start to current position
      final timing = recipe['timing'] as Map<String, dynamic>?;
      if (timing != null) {
        final transLenMs = ((timing['track_a_exit_end_ms'] as num? ?? 1400) -
                (timing['track_a_transition_start_ms'] as num? ?? 0))
            .abs()
            .toInt()
            .clamp(500, 5000);
        final currentMs = (_positionSec * 1000).toInt();
        timing['track_a_transition_start_ms'] = currentMs;
        timing['track_a_exit_end_ms'] = currentMs + transLenMs;
        _lastTransitionDurationSec = transLenMs / 1000.0;
      }

      _isTransitioning = true;
      _activeArchetype =
          (recipe['archetype'] as String? ?? 'Riff Handoff').replaceAll('ARCHETYPE_', '').replaceAll('_', ' ');
      _lastTransitionArchetype = _activeArchetype;
      _bridge.playerTriggerTransition(recipe);
      notifyListeners();
    } finally {
      _isTransitionPlanning = false;
    }
  }

  @visibleForTesting
  Map<String, dynamic> buildFallbackRecipe(TrackItem current, TrackItem next) => _buildFallbackRecipe(current, next);

  Map<String, dynamic> _buildFallbackRecipe(TrackItem current, TrackItem next) {
    final trackDurMs = current.duration.inMilliseconds > 0
        ? current.duration.inMilliseconds
        : (_durationSec * 1000).toInt();
    final fallbackStartMs = max(0, trackDurMs - 1400);
    final fallbackEndMs = trackDurMs > 0 ? trackDurMs : 1400;

    return {
      'schema_version': 1,
      'track_a_hash': current.id,
      'track_b_hash': next.id,
      'archetype': 'ARCHETYPE_2_RIFF_HANDOFF',
      'confidence_score': 0.95,
      'timing': {
        'track_a_transition_start_ms': fallbackStartMs,
        'track_a_exit_end_ms': fallbackEndMs,
        'track_b_entry_start_ms': 1000,
        'track_b_native_lock_ms': 2400,
      },
      'optional_stem_bridge_uri': null,
      'dsp_automation_keyframes': [
        {
          'offset_ms': 0,
          'deck_a': {
            'gain_db': 0.0,
            'eq_sub_db': 0.0,
            'eq_mid_db': 0.0,
            'eq_high_db': 0.0,
            'stretch_ratio': 1.0,
            'pitch_cents': 0,
            'reverb_freeze_send': 0.0,
          },
          'deck_b': {
            'gain_db': -6.0,
            'eq_sub_db': -12.0,
            'eq_mid_db': -3.0,
            'eq_high_db': -3.0,
            'stretch_ratio': 1.0,
            'pitch_cents': 0,
            'reverb_freeze_send': 0.0,
          },
        },
        {
          'offset_ms': 1400,
          'deck_a': {
            'gain_db': -60.0,
            'eq_sub_db': -24.0,
            'eq_mid_db': -24.0,
            'eq_high_db': -24.0,
            'stretch_ratio': 1.0,
            'pitch_cents': 0,
            'reverb_freeze_send': 0.45,
          },
          'deck_b': {
            'gain_db': 0.0,
            'eq_sub_db': 0.0,
            'eq_mid_db': 0.0,
            'eq_high_db': 0.0,
            'stretch_ratio': 1.0,
            'pitch_cents': 0,
            'reverb_freeze_send': 0.0,
          },
        },
      ],
    };
  }

  void reorderQueue(int oldIndex, int newIndex) {
    if (_shuffleMode == ShuffleMode.smart) return; // Smart Shuffle dynamically chooses upcoming tracks
    if (oldIndex < 0 || oldIndex >= _queue.length || newIndex < 0 || newIndex >= _queue.length) {
      return;
    }
    final currentId = currentTrack?.id;
    final item = _queue.removeAt(oldIndex);
    _queue.insert(newIndex, item);
    if (currentId != null) {
      final newCurrentIdx = _queue.indexWhere((t) => t.id == currentId);
      if (newCurrentIdx != -1) {
        _currentQueueIndex = newCurrentIdx;
      }
    }
    _upNextQueueIndex = _pickNextQueueIndex();
    final next = upNextTrack;
    if (next != null) {
      _bridge.playerLoadTrack(1, next.filePath);
    }
    _preparePendingRecipe();
    notifyListeners();
  }

  String? _loadedDeckAPath;

  void _advanceQueue() {
    if (_queue.isEmpty) return;
    _recordRecentTrack(currentTrack?.id);
    _currentQueueIndex = _upNextQueueIndex;
    _upNextQueueIndex = _pickNextQueueIndex();
    final current = currentTrack;
    if (current != null) {
      _loadedDeckAPath = current.filePath;
      _durationSec = current.duration.inSeconds.toDouble();
      _positionSec = 0.0;
    }
    // Asynchronously preload upcoming track into Deck B (deck 1)
    final next = upNextTrack;
    if (next != null) {
      _bridge.playerLoadTrack(1, next.filePath);
    }
    _preparePendingRecipe();
    _updatePlaybackNotification();
    notifyListeners();
  }

  void _loadCurrentTrackToEngine({bool force = false}) {
    final track = currentTrack;
    if (track == null) return;
    _durationSec = track.duration.inSeconds.toDouble();

    if (SettingsService.instance.decoderMode == AudioDecoderMode.systemDefault) {
      _positionSec = 0.0;
      _loadedDeckAPath = track.filePath;
      SystemAudioPlayerService.instance.load(
        track.filePath,
        startPositionSec: 0.0,
        autoPlay: _isPlaying,
      );
      _updatePlaybackNotification();
      return;
    }

    if (force || _loadedDeckAPath != track.filePath) {
      _positionSec = 0.0;
      _loadedDeckAPath = track.filePath;
      _bridge.playerLoadTrack(0, track.filePath);
    }
    final next = upNextTrack;
    if (next != null) {
      _bridge.playerLoadTrack(1, next.filePath);
    }
    _preparePendingRecipe();
    _updatePlaybackNotification();
  }

  void _updatePlaybackNotification() {
    final track = currentTrack;
    if (track == null) {
      AttuneNotificationService.instance.hidePlaybackNotification();
      return;
    }
    AttuneNotificationService.instance.showPlaybackNotification(
      title: track.title,
      artist: track.artist,
      isPlaying: _isPlaying,
      albumArtPath: track.artworkFile,
    );
  }

  Future<void> _preparePendingRecipe() async {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    final current = currentTrack;
    final next = upNextTrack;
    if (_shuffleMode != ShuffleMode.smart || current == null || next == null) {
      _pendingRecipe = null;
      return;
    }

    Map<String, dynamic>? fullProfileA = current.tapProfile;
    Map<String, dynamic>? fullProfileB = next.tapProfile;

    // On-Demand JIT Mode: Analyze head/tail cue zones for unprofiled tracks (<100 ms)
    if (fullProfileA == null || fullProfileA['cue_zones'] == null) {
      try {
        final profileA = await _bridge.profileTrack(current.filePath, tier: 0);
        if (profileA != null) {
          fullProfileA = profileA;
          final bpm = current.bpm > 0.0
              ? current.bpm
              : ((profileA['global_summary']?['nominal_bpm'] as num?)?.toDouble() ??
                  (profileA['global_summary']?['bpm_estimate'] as num?)?.toDouble() ??
                  0.0);
          final key = (current.musicalKey != 'Pending Profile' && current.musicalKey.isNotEmpty)
              ? current.musicalKey
              : (profileA['global_summary']?['primary_key'] as String? ??
                  profileA['global_summary']?['key_signature'] as String? ??
                  current.musicalKey);
          final compactA = TrackItem.extractCompactProfile(profileA);
          final updated = current.copyWith(tapProfile: compactA, bpm: bpm, musicalKey: key);
          final libIdx = _libraryTracks.indexWhere((t) => t.id == current.id);
          if (libIdx != -1) {
            _libraryTracks[libIdx] = updated;
          }
          if (_currentQueueIndex < _queue.length && _queue[_currentQueueIndex].id == current.id) {
            _queue[_currentQueueIndex] = updated;
          }
        }
      } catch (e) {
        debugPrint('[PlaybackController] JIT profile A error: $e');
      }
    }

    if (fullProfileB == null || fullProfileB['cue_zones'] == null) {
      try {
        final profileB = await _bridge.profileTrack(next.filePath, tier: 0);
        if (profileB != null) {
          fullProfileB = profileB;
          final bpm = next.bpm > 0.0
              ? next.bpm
              : ((profileB['global_summary']?['nominal_bpm'] as num?)?.toDouble() ??
                  (profileB['global_summary']?['bpm_estimate'] as num?)?.toDouble() ??
                  0.0);
          final key = (next.musicalKey != 'Pending Profile' && next.musicalKey.isNotEmpty)
              ? next.musicalKey
              : (profileB['global_summary']?['primary_key'] as String? ??
                  profileB['global_summary']?['key_signature'] as String? ??
                  next.musicalKey);
          final compactB = TrackItem.extractCompactProfile(profileB);
          final updated = next.copyWith(tapProfile: compactB, bpm: bpm, musicalKey: key);
          final libIdx = _libraryTracks.indexWhere((t) => t.id == next.id);
          if (libIdx != -1) {
            _libraryTracks[libIdx] = updated;
          }
          if (_upNextQueueIndex < _queue.length && _queue[_upNextQueueIndex].id == next.id) {
            _queue[_upNextQueueIndex] = updated;
          }
        }
      } catch (e) {
        debugPrint('[PlaybackController] JIT profile B error: $e');
      }
    }

    if (fullProfileA != null && fullProfileB != null) {
      try {
        final recipe = await _bridge.planTransition(fullProfileA, fullProfileB);
        if (recipe != null) {
          _pendingRecipe = recipe;
          final archetype = recipe['archetype'] as String? ?? 'ARCHETYPE_2_RIFF_HANDOFF';
          _activeArchetype = archetype.replaceAll('ARCHETYPE_', '').replaceAll('_', ' ');
          notifyListeners();
          return;
        }
      } catch (e) {
        debugPrint('[PlaybackController] Error preparing transition recipe: $e');
      }
    }

    // Guarantee fallback recipe so smart shuffle transitions NEVER stall
    final curWithProfile = currentTrack;
    final nextWithProfile = upNextTrack;
    if (curWithProfile != null && nextWithProfile != null) {
      _pendingRecipe = _buildFallbackRecipe(curWithProfile, nextWithProfile);
      _activeArchetype = 'Equal-Power Crossfade';
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _pollingTimer?.cancel();
    _bridge.playerStop();
    super.dispose();
  }
}
