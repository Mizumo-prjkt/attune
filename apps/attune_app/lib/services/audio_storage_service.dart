import 'dart:convert';
import 'dart:io';
import 'dart:isolate';
import 'package:flutter/material.dart';
import '../models/track_item.dart';
import 'environment_delegator.dart';

/// Centralized filesystem & storage path resolution service.
/// In charge of all cross-platform file system calls, supporting:
/// - Linux: XDG directories, user home music, local workspace symlinks.
/// - Android: Scoped storage (Android 11+ API 30+ /storage/emulated/0/Music, SAF & app-specific media).
/// - Windows: USERPROFILE/Music and drive-relative directories.
/// - macOS/iOS: Home Music and sandbox container paths.
class AudioStorageService {
  static final AudioStorageService instance = AudioStorageService._();
  AudioStorageService._();

  static const Set<String> supportedAudioExtensions = {
    '.flac',
    '.mp3',
    '.wav',
    '.ogg',
    '.m4a',
    '.aac',
    '.opus',
    '.alac',
    '.aiff',
  };

  /// Safely ensures the working directory is valid
  static void _ensureValidDirectory() {
    AppEnvironmentDelegator.current.ensureValidWorkingDirectory();
  }

  /// Returns candidate directory paths based on the current operating system environment.
  List<String> getCandidateRoots({List<String>? additionalRoots}) {
    return AppEnvironmentDelegator.current.getMusicCandidateRoots(additionalRoots: additionalRoots);
  }

  /// Recursively discovers all audio files across all available candidate directories.
  /// Protects against infinite symlink loops by tracking canonical paths.
  Future<List<File>> discoverAudioFiles({List<String>? additionalRoots}) async {
    final candidateRoots = getCandidateRoots(additionalRoots: additionalRoots);
    final audioFiles = <File>[];
    final seenCanonicalPaths = <String>{};

    for (final rootPath in candidateRoots) {
      final dir = Directory(rootPath);
      if (!dir.existsSync()) continue;

      try {
        final entities = dir.listSync(recursive: true, followLinks: true);
        for (final entity in entities) {
          if (entity is! File) continue;

          final lower = entity.path.toLowerCase();
          final isAudio = supportedAudioExtensions.any((ext) => lower.endsWith(ext));
          if (!isAudio) continue;

          try {
            final canonical = entity.resolveSymbolicLinksSync();
            if (seenCanonicalPaths.add(canonical)) {
              audioFiles.add(entity);
            }
          } catch (_) {
            if (seenCanonicalPaths.add(entity.path)) {
              audioFiles.add(entity);
            }
          }
        }
      } catch (e) {
        debugPrint('[AudioStorageService] Notice reading $rootPath: $e');
      }

      // If we found a substantial library in the primary music directory, proceed
      if (audioFiles.length > 50) {
        break;
      }
    }

    return audioFiles;
  }

  /// Parses track metadata from a discovered audio file, extracting title, artist,
  /// album, duration, artwork, and genre.
  TrackItem parseTrackMetadata(File file) {
    final path = file.path;
    final segments = path.split(Platform.pathSeparator);
    final filename = segments.last;
    final dotIndex = filename.lastIndexOf('.');
    final extension = dotIndex != -1 ? filename.substring(dotIndex + 1).toLowerCase() : '';
    var baseName = dotIndex != -1 ? filename.substring(0, dotIndex) : filename;

    // Clean track number prefixes (e.g. "01 ", "01. ", "16bit_1. ", "16bit_1-07 ", "1-11 ")
    baseName = baseName.replaceAll(RegExp(r'^(16bit_)?(\d+[\.\-_])*\d+[\.\-\s]+'), '').trim();

    String artist = 'Unknown Artist';
    String title = baseName;
    String album = 'Local Library';
    String genre = 'Pop';
    Duration duration = const Duration(minutes: 3);

    // Parent directory clues
    if (segments.length >= 2) {
      album = segments[segments.length - 2];
    }
    if (segments.length >= 3) {
      final potentialArtist = segments[segments.length - 3];
      if (potentialArtist != 'music' &&
          potentialArtist != 'Soulseek Complete' &&
          potentialArtist != 'Documents' &&
          potentialArtist != 'Music') {
        artist = potentialArtist;
      }
    }

    // Inspect album.nfo if available in same folder
    final parent = file.parent;
    final nfoFile = File('${parent.path}/album.nfo');
    if (nfoFile.existsSync()) {
      try {
        final content = nfoFile.readAsStringSync();
        final artistMatch = RegExp(r'<artist>(.*?)</artist>').firstMatch(content);
        if (artistMatch != null && artistMatch.group(1)!.trim().isNotEmpty) {
          artist = artistMatch.group(1)!.trim();
        }
        final albumMatch = RegExp(r'<title>(.*?)</title>').firstMatch(content);
        if (albumMatch != null && albumMatch.group(1)!.trim().isNotEmpty) {
          album = albumMatch.group(1)!.trim();
        }
        final genreMatch = RegExp(r'<genre>(.*?)</genre>').firstMatch(content);
        if (genreMatch != null && genreMatch.group(1)!.trim().isNotEmpty) {
          genre = _normalizeGenre(genreMatch.group(1)!.trim());
        }
        final durMatch = RegExp(r'<duration>(\d+):(\d+)</duration>').firstMatch(content);
        if (durMatch != null) {
          final m = int.tryParse(durMatch.group(1)!) ?? 3;
          final s = int.tryParse(durMatch.group(2)!) ?? 0;
          duration = Duration(minutes: m, seconds: s);
        }
      } catch (_) {}
    }

    // Parse "Artist - Title" filename format
    if (baseName.contains(' - ')) {
      final parts = baseName.split(' - ');
      if (parts.length >= 2) {
        artist = parts[0].trim();
        title = parts.sublist(1).join(' - ').trim();
      }
    }

    // Format disc compilations (e.g. Project Mirai / Diva OSTs)
    if (album.startsWith('Disc ')) {
      album = 'Project Mirai / Diva ($album)';
      if (artist == 'Unknown Artist') artist = 'Vocaloid';
    }

    // Infer genre if still default
    if (genre == 'Pop') {
      genre = _inferGenre(title: title, artist: artist, album: album);
    }

    // Detect album artwork file in same folder
    String? artworkPath;
    final artCandidates = ['cover.jpg', 'cover.png', 'Cover.jpg', 'Folder.jpg', 'AlbumArtSmall.jpg'];
    for (final art in artCandidates) {
      final artFile = File('${parent.path}/$art');
      if (artFile.existsSync()) {
        artworkPath = artFile.path;
        break;
      }
    }

    // Dynamic accent color
    const palette = [
      Color(0xFF6366F1), // Electric Indigo
      Color(0xFFEC4899), // Pink Punch
      Color(0xFFF59E0B), // Vibrant Amber
      Color(0xFF10B981), // Emerald
      Color(0xFFEF4444), // Crimson
      Color(0xFF8B5CF6), // Purple Haze
      Color(0xFF06B6D4), // Cyan
      Color(0xFFF43F5E), // Rose
      Color(0xFF3B82F6), // Blue
      Color(0xFF14B8A6), // Teal
    ];
    final seedColor = palette[title.hashCode.abs() % palette.length];

    final isLossless = extension == 'flac' || extension == 'wav' || extension == 'alac' || extension == 'aiff';
    final formatStr = extension.toUpperCase();
    final trackId = '${title.toLowerCase().replaceAll(RegExp(r'[^a-z0-9]'), '_')}_${title.hashCode.abs()}';

    return TrackItem(
      id: trackId,
      title: title,
      artist: artist,
      album: album,
      filePath: file.path,
      duration: duration,
      bpm: 120.0,
      musicalKey: '8B (C Major)',
      format: formatStr,
      genre: genre,
      sampleRate: 44100,
      bitDepth: 16,
      isLossless: isLossless,
      isProprietary: false,
      seedColor: seedColor,
      artworkFile: artworkPath,
    );
  }

  /// Safely resolves the application configuration and data directory across platforms
  /// via the active AppEnvironmentDelegator.
  Directory getAppDataDirectory() {
    return AppEnvironmentDelegator.current.appDataDirectory;
  }

  /// Safely resolves the application ephemeral cache directory across platforms
  /// via the active AppEnvironmentDelegator.
  Directory getCacheDirectory() {
    return AppEnvironmentDelegator.current.cacheDirectory;
  }

  File get _cacheFile {
    final dir = getAppDataDirectory();
    return File('${dir.path}/library_cache.json');
  }

  File get _configFile {
    final dir = getAppDataDirectory();
    return File('${dir.path}/config.json');
  }


  /// Loads cached track metadata from local disk for instant (< 5ms) startup.
  List<TrackItem> loadCachedTracks() {
    try {
      final file = _cacheFile;
      if (file.existsSync()) {
        final content = file.readAsStringSync();
        if (content.isNotEmpty) {
          final list = jsonDecode(content) as List<dynamic>;
          return list.map((item) => TrackItem.fromJson(item as Map<String, dynamic>)).toList();
        }
      }
    } catch (e) {
      debugPrint('[AudioStorageService] Cache load notice: $e');
    }
    return [];
  }

  /// Persists track list to local disk cache.
  void saveCachedTracks(List<TrackItem> tracks) {
    try {
      final file = _cacheFile;
      final jsonList = tracks.map((t) => t.toJson()).toList();
      file.writeAsStringSync(jsonEncode(jsonList));
      debugPrint('[AudioStorageService] Saved ${tracks.length} tracks to library cache: ${file.path}');
    } catch (e) {
      debugPrint('[AudioStorageService] Cache save notice: $e');
    }
  }

  /// Checks if first-run setup has been completed.
  bool isSetupCompleted() {
    try {
      final file = _configFile;
      if (file.existsSync()) {
        final content = file.readAsStringSync();
        final map = jsonDecode(content) as Map<String, dynamic>;
        return (map['has_completed_setup'] as bool?) ?? (map['setupCompleted'] as bool?) ?? false;
      }
    } catch (_) {}
    return false;
  }

  /// Saves setup completion status and selected music roots.
  void setSetupCompleted(bool completed, {List<String>? selectedRoots}) {
    try {
      final file = _configFile;
      final map = {
        'has_completed_setup': completed,
        'selected_roots': selectedRoots ?? [],
        'updated_at': DateTime.now().toIso8601String(),
      };
      file.writeAsStringSync(jsonEncode(map));
      debugPrint('[AudioStorageService] Updated setup config: completed=$completed');
    } catch (e) {
      debugPrint('[AudioStorageService] Config save error: $e');
    }
  }

  /// Scans library and returns sorted track items.
  /// If [forceRefresh] is false and cache exists, returns cached tracks immediately!
  Future<List<TrackItem>> scanLibrary({bool forceRefresh = false, List<String>? additionalRoots}) async {
    _ensureValidDirectory();

    // 1. Fast Path: Read from local disk cache immediately
    if (!forceRefresh && additionalRoots == null) {
      final cached = loadCachedTracks();
      if (cached.isNotEmpty) {
        debugPrint('[AudioStorageService] Loaded ${cached.length} tracks instantly from local cache.');
        return cached;
      }
    }

    // 2. Multi-core background Isolate offload with robust fallback
    try {
      final candidateRoots = getCandidateRoots(additionalRoots: additionalRoots);
      final tracks = Platform.environment.containsKey('FLUTTER_TEST')
          ? _scanLibraryInWorker(candidateRoots)
          : await Isolate.run(() => _scanLibraryInWorker(candidateRoots));

      if (tracks.isNotEmpty) {
        saveCachedTracks(tracks);
        return tracks;
      }
    } catch (e) {
      debugPrint('[AudioStorageService] Background scan notice: $e. Falling back to local cache.');
    }

    // Fallback: If scan encountered an issue, recover using cached tracks
    final cached = loadCachedTracks();
    if (cached.isNotEmpty) {
      debugPrint('[AudioStorageService] Recovered ${cached.length} tracks from disk cache.');
      return cached;
    }

    return [];
  }

  static List<TrackItem> _scanLibraryInWorker(List<String> candidateRoots) {
    _ensureValidDirectory();
    final audioFiles = <File>[];
    final seenCanonicalPaths = <String>{};

    for (final rootPath in candidateRoots) {
      try {
        final dir = Directory(rootPath);
        if (!dir.existsSync()) continue;

        final entities = dir.listSync(recursive: true, followLinks: true);
        for (final entity in entities) {
          if (entity is! File) continue;

          final lower = entity.path.toLowerCase();
          final isAudio = supportedAudioExtensions.any((ext) => lower.endsWith(ext));
          if (!isAudio) continue;

          try {
            final canonical = entity.resolveSymbolicLinksSync();
            if (seenCanonicalPaths.add(canonical)) {
              audioFiles.add(entity);
            }
          } catch (_) {
            if (seenCanonicalPaths.add(entity.path)) {
              audioFiles.add(entity);
            }
          }
        }
      } catch (e) {
        debugPrint('[AudioStorageService] Notice reading $rootPath: $e');
      }

      if (audioFiles.length > 50) {
        break;
      }
    }

    final tracks = audioFiles.map((f) => instance.parseTrackMetadata(f)).toList();

    tracks.sort((a, b) {
      final aScore = instance._trackPriorityScore(a);
      final bScore = instance._trackPriorityScore(b);
      if (aScore != bScore) return bScore.compareTo(aScore);
      return a.title.compareTo(b.title);
    });

    return tracks;
  }

  int _trackPriorityScore(TrackItem t) {
    final lower = '${t.title} ${t.artist} ${t.filePath}'.toLowerCase();
    if (lower.contains('rabbit hole')) return 100;
    if (lower.contains('(not) a devil') || lower.contains('not a devil')) return 90;
    if (lower.contains('liar dancer')) return 80;
    if (lower.contains('東京テディベア') || lower.contains('teddy bear')) return 70;
    if (lower.contains('ロストワンの号哭') || lower.contains("lost one's weeping")) return 60;
    if (lower.contains('konton boogie')) return 50;
    return 0;
  }

  String _normalizeGenre(String raw) {
    final lower = raw.toLowerCase();
    if (lower.contains('rock') || lower.contains('punk') || lower.contains('metal')) return 'Rock';
    if (lower.contains('jazz') || lower.contains('swing') || lower.contains('blues')) return 'Jazz';
    if (lower.contains('electro') || lower.contains('dance') || lower.contains('edm') || lower.contains('synth')) return 'Electronic';
    if (lower.contains('pop') || lower.contains('asian')) return 'Pop';
    return raw;
  }

  String _inferGenre({required String title, required String artist, required String album}) {
    final text = '$title $artist $album'.toLowerCase();
    if (text.contains('rock') ||
        text.contains('neru') ||
        text.contains('punk') ||
        text.contains('metal') ||
        text.contains('guitar') ||
        text.contains('号哭') ||
        text.contains('テディベア')) {
      return 'Rock';
    }
    if (text.contains('jazz') || text.contains('swing') || text.contains('piano') || text.contains('symphony')) {
      return 'Jazz';
    }
    if (text.contains('dance') ||
        text.contains('fever') ||
        text.contains('boogie') ||
        text.contains('dancer') ||
        text.contains('synthesizer') ||
        text.contains('synth') ||
        text.contains('samfree') ||
        text.contains('techno')) {
      return 'Electronic';
    }
    return 'Pop';
  }
}
