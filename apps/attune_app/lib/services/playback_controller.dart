import 'dart:async';
import 'dart:io';
import 'dart:math';
import 'package:flutter/material.dart';
import '../models/track_item.dart';
import 'audio_storage_service.dart';
import 'engine_bridge.dart';

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
  final List<TrackItem> _libraryTracks = [];
  final List<TrackItem> _queue = [];
  int _currentQueueIndex = 0;
  int _upNextQueueIndex = 0;
  bool _isScanning = false;
  ShuffleMode _shuffleMode = ShuffleMode.off;

  // Recent Playback History to prevent track repetition and bias in smart shuffle
  final List<String> _recentTrackIds = [];
  static const int _maxRecentHistory = 15;

  void _recordRecentTrack(String? id) {
    if (id == null || id.isEmpty) return;
    _recentTrackIds.remove(id);
    _recentTrackIds.add(id);
    if (_recentTrackIds.length > _maxRecentHistory) {
      _recentTrackIds.removeAt(0);
    }
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

  static double _calculateHarmonicScore(String keyA, String keyB) {
    final camA = _normalizeToCamelot(keyA);
    final camB = _normalizeToCamelot(keyB);
    if (camA == null || camB == null) return 0.15;

    final numA = int.tryParse(camA.substring(0, camA.length - 1));
    final modeA = camA.substring(camA.length - 1);
    final numB = int.tryParse(camB.substring(0, camB.length - 1));
    final modeB = camB.substring(camB.length - 1);

    if (numA == null || numB == null) return 0.15;

    final diff = (numA - numB).abs();
    final circleDist = min(diff, 12 - diff);

    if (circleDist == 0 && modeA == modeB) {
      return 0.40; // Exact harmonic key match
    } else if (circleDist == 0 && modeA != modeB) {
      return 0.35; // Relative major/minor
    } else if (circleDist == 1 && modeA == modeB) {
      return 0.30; // Adjacent Camelot harmonic blend
    } else if (circleDist == 2 && modeA == modeB) {
      return 0.20; // 2-step energy boost
    } else if (circleDist == 1 && modeA != modeB) {
      return 0.25; // Diagonal harmonic step
    } else {
      return 0.05; // Distant harmonic relationship
    }
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

  Timer? _pollingTimer;

  PlaybackController._init() {
    _initEngine();
    scanLocalAudioDirectories();
  }

  // Getters
  List<TrackItem> get libraryTracks => List.unmodifiable(_libraryTracks);
  List<TrackItem> get queue => List.unmodifiable(_queue);
  int get currentQueueIndex => _currentQueueIndex;
  int get upNextQueueIndex => _upNextQueueIndex;
  bool get isScanning => _isScanning;
  ShuffleMode get shuffleMode => _shuffleMode;
  Map<String, dynamic>? get pendingRecipe => _pendingRecipe;

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

  void cycleShuffleMode() {
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

  /// Scans local directories for genuine audio files (.flac, .mp3, .wav, .ogg, etc.)
  /// utilizing AudioStorageService across Windows, Linux, and Android scoped storage.
  Future<void> scanLocalAudioDirectories({String? customPath, bool forceRefresh = false}) async {
    if (_isScanning) return;
    _isScanning = true;
    notifyListeners();

    try {
      final scannedTracks = await AudioStorageService.instance.scanLibrary(
        forceRefresh: forceRefresh,
        additionalRoots: customPath != null ? [customPath] : null,
      );

      if (scannedTracks.isEmpty) {
        debugPrint('[PlaybackController] No audio files discovered by AudioStorageService.');
        _isScanning = false;
        notifyListeners();
        return;
      }

      _libraryTracks.clear();
      _libraryTracks.addAll(scannedTracks);

      _queue.clear();
      _queue.addAll(scannedTracks);
      _currentQueueIndex = 0;
      _upNextQueueIndex = _pickNextQueueIndex();

      if (_queue.isNotEmpty) {
        _durationSec = _queue.first.duration.inSeconds.toDouble();
        _loadCurrentTrackToEngine();
      }

      debugPrint('[PlaybackController] Ingested ${_libraryTracks.length} genuine tracks via AudioStorageService.');
    } catch (e) {
      debugPrint('[PlaybackController] Error scanning audio files: $e');
    } finally {
      _isScanning = false;
      notifyListeners();
    }
  }

  Future<void> profileTrackAsync(TrackItem track) async {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    try {
      final tap = await _bridge.profileTrack(track.filePath, tier: 1);
      if (tap != null) {
        final summary = tap['global_summary'] as Map<String, dynamic>?;
        final trackIdObj = tap['track_id'] as Map<String, dynamic>?;

        final nominalBpm = (summary?['nominal_bpm'] as num?)?.toDouble() ?? track.bpm;
        final primaryKey = summary?['primary_key'] as String? ?? track.musicalKey;
        final durMs = (trackIdObj?['duration_ms'] as num?)?.toInt();
        final actualDuration = durMs != null && durMs > 0 ? Duration(milliseconds: durMs) : track.duration;

        final updated = track.copyWith(
          bpm: nominalBpm,
          musicalKey: primaryKey,
          duration: actualDuration,
          tapProfile: tap,
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
        notifyListeners();
      }
    } catch (e) {
      debugPrint('[PlaybackController] Background profiling notice for ${track.title}: $e');
    }
  }

  void _initEngine() {
    _bridge.playerStart();
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
    final state = _bridge.playerGetState();

    if (state != null && state['initialized'] == true) {
      _isPlaying = state['is_playing'] as bool? ?? false;
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

      final trans = state['transition'] as Map<String, dynamic>?;
      if (trans != null && trans['in_progress'] == true) {
        _isTransitioning = true;
        _transitionProgress = (trans['progress'] as num? ?? 0.0).toDouble();
        _activeArchetype =
            (trans['archetype'] as String? ?? 'Riff Handoff').replaceAll('ARCHETYPE_', '').replaceAll('_', ' ');
      } else {
        if (_isTransitioning) {
          // Transition finished in native mixer: advance queue and update active deck
          _isTransitioning = false;
          _transitionProgress = 0.0;
          _pendingRecipe = null;
          _advanceQueue();
        } else if (_isPlaying && _durationSec > 5.0) {
          final activeDeckJson = activeDeck == 'DeckA' ? deckA : deckB;
          final deckState = activeDeckJson?['state'] as String?;
          final isAudibleFinished = activeDeckJson?['is_audible_finished'] == true;
          final audibleEndMs = ((activeDeckJson?['audible_end_ms'] as num?)?.toDouble() ?? 0.0);
          final isNearAudibleEnd = audibleEndMs > 0 && (_positionSec * 1000 >= audibleEndMs - 250);
          final isTrackEnded = deckState == 'Finished' || isAudibleFinished || isNearAudibleEnd || _positionSec >= _durationSec - 0.3;

          if (_shuffleMode == ShuffleMode.smart) {
            // Smart Algorithmic Transition: trigger at genuine recipe start time
            if (_pendingRecipe != null && !_isTransitionPlanning) {
              final timing = _pendingRecipe!['timing'] as Map<String, dynamic>?;
              final startMs = (timing?['track_a_transition_start_ms'] as num?)?.toDouble();
              if (startMs != null && _positionSec * 1000 >= startMs) {
                _executeAutoTransition(_pendingRecipe!);
              } else if (isTrackEnded) {
                // If track reaches audible silence before recipe startMs, trigger transition immediately!
                _executeAutoTransition(_pendingRecipe!);
              }
            } else if (isTrackEnded) {
              skipNext();
            }
          } else {
            // Normal (Off) or Standard Random Shuffle: play until music finishes, cutting trailing dead air
            if (isTrackEnded) {
              skipNext();
            }
          }
        }
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
  }

  void _executeAutoTransition(Map<String, dynamic> recipe) {
    if (_isTransitioning) return;
    _isTransitioning = true;
    _activeArchetype =
        (recipe['archetype'] as String? ?? 'Riff Handoff').replaceAll('ARCHETYPE_', '').replaceAll('_', ' ');
    _bridge.playerTriggerTransition(recipe);
    notifyListeners();
  }

  void togglePlay() {
    if (_isPlaying) {
      pause();
    } else {
      play();
    }
  }

  void play() {
    _isPlaying = true;
    _bridge.playerPlay();
    notifyListeners();
  }

  void pause() {
    _isPlaying = false;
    _bridge.playerPause();
    notifyListeners();
  }

  void seek(double seconds) {
    _positionSec = seconds.clamp(0.0, _durationSec);
    _bridge.playerSeek(-1, (_positionSec * 1000).toInt());
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

  void skipNext() {
    if (_queue.isEmpty) return;
    _recordRecentTrack(currentTrack?.id);

    _currentQueueIndex = _upNextQueueIndex;
    _upNextQueueIndex = _pickNextQueueIndex();

    _positionSec = 0.0;
    _transitionProgress = 0.0;
    _isTransitioning = false;
    _pendingRecipe = null;
    _loadCurrentTrackToEngine(force: true);
    if (_isPlaying) {
      _bridge.playerPlay();
    }
    notifyListeners();
  }

  int _findBestSmartMatchIndex() {
    if (_queue.length <= 1) return _currentQueueIndex;
    final current = currentTrack;
    if (current == null) return (_currentQueueIndex + 1) % _queue.length;

    final List<({int index, double score})> scored = [];

    for (int i = 0; i < _queue.length; i++) {
      if (i == _currentQueueIndex) continue;
      final candidate = _queue[i];

      // 1. Recency Penalty (Heavy penalty for recently played songs to eliminate repetitive bias)
      double recencyPenalty = 0.0;
      final historyIdx = _recentTrackIds.lastIndexOf(candidate.id);
      if (historyIdx != -1) {
        final recencyRank = _recentTrackIds.length - 1 - historyIdx;
        if (recencyRank == 0) {
          recencyPenalty = -6.0; // The song that just played
        } else if (recencyRank <= 2) {
          recencyPenalty = -4.0;
        } else if (recencyRank <= 5) {
          recencyPenalty = -2.5;
        } else if (recencyRank <= 10) {
          recencyPenalty = -1.0;
        } else {
          recencyPenalty = -0.5;
        }
      }

      // 2. Tempo compatibility
      double tempoScore = 0.0;
      final isCurrentDefault = current.bpm <= 0 || (current.bpm == 120.0 && current.tapProfile == null);
      final isCandidateDefault = candidate.bpm <= 0 || (candidate.bpm == 120.0 && candidate.tapProfile == null);
      if (isCurrentDefault || isCandidateDefault) {
        tempoScore = 0.35; // Neutral baseline for unprofiled tracks
      } else {
        final bpmDelta = (current.bpm - candidate.bpm).abs();
        if (bpmDelta <= 5) {
          tempoScore = 1.0;
        } else if (bpmDelta <= 12) {
          tempoScore = 0.8;
        } else if (bpmDelta <= 20) {
          tempoScore = 0.55;
        } else {
          final ratio = candidate.bpm / current.bpm;
          if ((ratio - 1.0).abs() < 0.12 || (ratio - 0.5).abs() < 0.08 || (ratio - 2.0).abs() < 0.08) {
            tempoScore = 0.75;
          } else {
            tempoScore = (0.5 - (bpmDelta / 200.0)).clamp(0.0, 0.45);
          }
        }
      }

      // 3. Harmonic Key Alignment (Camelot Wheel / Circle of Fifths)
      final harmonicScore = _calculateHarmonicScore(current.musicalKey, candidate.musicalKey);

      // 4. Genre alignment bonus
      final genreBonus = (candidate.genre.isNotEmpty &&
              current.genre.isNotEmpty &&
              candidate.genre.toLowerCase() == current.genre.toLowerCase())
          ? 0.35
          : 0.0;

      // 5. Artist variety preference
      final varietyScore = (candidate.artist != current.artist) ? 0.20 : 0.0;

      // 6. Aggression / Dynamic energy similarity (if TAP profiles exist)
      double aggressionScore = 0.0;
      if (current.tapProfile != null && candidate.tapProfile != null) {
        final aAgg = (current.tapProfile!['global_summary']?['aggression_index'] as num?)?.toDouble() ?? 0.5;
        final bAgg = (candidate.tapProfile!['global_summary']?['aggression_index'] as num?)?.toDouble() ?? 0.5;
        aggressionScore = (1.0 - (aAgg - bAgg).abs()) * 0.25;
      }

      final totalScore = tempoScore + harmonicScore + genreBonus + varietyScore + aggressionScore + recencyPenalty;
      scored.add((index: i, score: totalScore));
    }

    if (scored.isEmpty) return (_currentQueueIndex + 1) % _queue.length;

    // Top-K Weighted Probabilistic Selection (Prevents index 0 deterministic tie-breaking)
    scored.sort((a, b) => b.score.compareTo(a.score));
    final topScore = scored.first.score;

    final pool = scored.take(6).where((c) => c.score >= topScore - 0.40).toList();
    if (pool.isEmpty) return scored.first.index;

    final minScore = pool.map((c) => c.score).reduce(min);
    final weights = pool.map((c) => max(0.1, c.score - minScore + 0.5)).toList();
    final totalWeight = weights.reduce((a, b) => a + b);
    double randVal = Random().nextDouble() * totalWeight;

    for (int k = 0; k < pool.length; k++) {
      randVal -= weights[k];
      if (randVal <= 0) {
        return pool[k].index;
      }
    }
    return pool.first.index;
  }

  void skipPrevious() {
    if (_positionSec > 3.0) {
      seek(0.0);
    } else if (_currentQueueIndex > 0) {
      _recordRecentTrack(currentTrack?.id);
      _currentQueueIndex--;
      _upNextQueueIndex = _pickNextQueueIndex();
      _positionSec = 0.0;
      _isTransitioning = false;
      _transitionProgress = 0.0;
      _pendingRecipe = null;
      _loadCurrentTrackToEngine(force: true);
      if (_isPlaying) {
        _bridge.playerPlay();
      }
    }
    notifyListeners();
  }

  void playTrack(TrackItem track) {
    _recordRecentTrack(currentTrack?.id);
    final existingIdx = _queue.indexWhere((t) => t.id == track.id);
    if (existingIdx != -1) {
      _currentQueueIndex = existingIdx;
    } else {
      _queue.insert(_currentQueueIndex + 1, track);
      _currentQueueIndex++;
    }
    _upNextQueueIndex = _pickNextQueueIndex();
    _positionSec = 0.0;
    _isTransitioning = false;
    _transitionProgress = 0.0;
    _pendingRecipe = null;
    _loadCurrentTrackToEngine(force: true);
    play();
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

      // For manual transition trigger, anchor start to current position
      if (recipe != null) {
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
        }
      }

      recipe ??= _buildFallbackRecipe(current, next);

      _isTransitioning = true;
      _activeArchetype =
          (recipe['archetype'] as String? ?? 'Riff Handoff').replaceAll('ARCHETYPE_', '').replaceAll('_', ' ');
      _bridge.playerTriggerTransition(recipe);
      notifyListeners();
    } finally {
      _isTransitionPlanning = false;
    }
  }

  Map<String, dynamic> _buildFallbackRecipe(TrackItem current, TrackItem next) {
    return {
      'schema_version': 1,
      'track_a_hash': current.id,
      'track_b_hash': next.id,
      'archetype': 'ARCHETYPE_2_RIFF_HANDOFF',
      'confidence_score': 0.95,
      'timing': {
        'track_a_transition_start_ms': (_positionSec * 1000).toInt(),
        'track_a_exit_end_ms': ((_positionSec + 1.4) * 1000).toInt(),
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
    if (oldIndex < 0 || oldIndex >= _queue.length || newIndex < 0 || newIndex >= _queue.length) {
      return;
    }
    final item = _queue.removeAt(oldIndex);
    _queue.insert(newIndex, item);
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
    notifyListeners();
  }

  void _loadCurrentTrackToEngine({bool force = false}) {
    final track = currentTrack;
    if (track != null) {
      _durationSec = track.duration.inSeconds.toDouble();
      if (force || _loadedDeckAPath != track.filePath) {
        _positionSec = 0.0;
        _loadedDeckAPath = track.filePath;
        _bridge.playerLoadTrack(0, track.filePath);
      }
    }
    final next = upNextTrack;
    if (next != null) {
      _bridge.playerLoadTrack(1, next.filePath);
    }
    _preparePendingRecipe();
  }

  Future<void> _preparePendingRecipe() async {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    final current = currentTrack;
    final next = upNextTrack;
    if (_shuffleMode != ShuffleMode.smart || current == null || next == null) {
      _pendingRecipe = null;
      return;
    }

    // On-Demand JIT Mode: Analyze head/tail cue zones for unprofiled tracks (<100 ms)
    if (current.tapProfile == null) {
      try {
        final profileA = await _bridge.profileTrack(current.filePath, tier: 0);
        if (profileA != null) {
          final bpm = (profileA['global_summary']?['bpm_estimate'] as num?)?.toDouble() ?? current.bpm;
          final key = profileA['global_summary']?['key_signature'] as String? ?? current.musicalKey;
          final updated = current.copyWith(tapProfile: profileA, bpm: bpm, musicalKey: key);
          if (_currentQueueIndex < _queue.length && _queue[_currentQueueIndex].id == current.id) {
            _queue[_currentQueueIndex] = updated;
          }
          AudioStorageService.instance.saveCachedTracks(_queue);
        }
      } catch (e) {
        debugPrint('[PlaybackController] JIT profile A error: $e');
      }
    }

    if (next.tapProfile == null) {
      try {
        final profileB = await _bridge.profileTrack(next.filePath, tier: 0);
        if (profileB != null) {
          final bpm = (profileB['global_summary']?['bpm_estimate'] as num?)?.toDouble() ?? next.bpm;
          final key = profileB['global_summary']?['key_signature'] as String? ?? next.musicalKey;
          final updated = next.copyWith(tapProfile: profileB, bpm: bpm, musicalKey: key);
          if (_upNextQueueIndex < _queue.length && _queue[_upNextQueueIndex].id == next.id) {
            _queue[_upNextQueueIndex] = updated;
          }
          AudioStorageService.instance.saveCachedTracks(_queue);
        }
      } catch (e) {
        debugPrint('[PlaybackController] JIT profile B error: $e');
      }
    }

    final curWithProfile = currentTrack;
    final nextWithProfile = upNextTrack;
    if (curWithProfile?.tapProfile != null && nextWithProfile?.tapProfile != null) {
      try {
        final recipe = await _bridge.planTransition(curWithProfile!.tapProfile!, nextWithProfile!.tapProfile!);
        if (recipe != null) {
          _pendingRecipe = recipe;
          final archetype = recipe['archetype'] as String? ?? 'ARCHETYPE_2_RIFF_HANDOFF';
          _activeArchetype = archetype.replaceAll('ARCHETYPE_', '').replaceAll('_', ' ');
          notifyListeners();
        }
      } catch (e) {
        debugPrint('[PlaybackController] Error preparing transition recipe: $e');
      }
    }
  }

  @override
  void dispose() {
    _pollingTimer?.cancel();
    _bridge.playerStop();
    super.dispose();
  }
}
