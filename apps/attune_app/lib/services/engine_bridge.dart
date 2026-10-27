import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';

// Native function typedefs
typedef AttuneInitC = Void Function();
typedef AttuneInitDart = void Function();

typedef AttuneVersionC = Pointer<Utf8> Function();
typedef AttuneVersionDart = Pointer<Utf8> Function();

typedef AttuneFreeStringC = Void Function(Pointer<Utf8>);
typedef AttuneFreeStringDart = void Function(Pointer<Utf8>);

typedef AttuneProfileFileC = Pointer<Utf8> Function(Pointer<Utf8>, Int32);
typedef AttuneProfileFileDart = Pointer<Utf8> Function(Pointer<Utf8>, int);

typedef AttunePlanTransitionC = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef AttunePlanTransitionDart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

typedef AttuneCacheStatsC = Pointer<Utf8> Function(Pointer<Utf8>);
typedef AttuneCacheStatsDart = Pointer<Utf8> Function(Pointer<Utf8>);

typedef AttunePlayerStartC = Int32 Function();
typedef AttunePlayerStartDart = int Function();

typedef AttunePlayerLoadTrackC = Int32 Function(Int32, Pointer<Utf8>);
typedef AttunePlayerLoadTrackDart = int Function(int, Pointer<Utf8>);

typedef AttunePlayerPlayC = Int32 Function();
typedef AttunePlayerPlayDart = int Function();

typedef AttunePlayerPauseC = Int32 Function();
typedef AttunePlayerPauseDart = int Function();

typedef AttunePlayerSeekC = Int32 Function(Int32, Uint64);
typedef AttunePlayerSeekDart = int Function(int, int);

typedef AttunePlayerTriggerTransitionC = Int32 Function(Pointer<Utf8>);
typedef AttunePlayerTriggerTransitionDart = int Function(Pointer<Utf8>);

typedef AttunePlayerGetStateC = Pointer<Utf8> Function();
typedef AttunePlayerGetStateDart = Pointer<Utf8> Function();

typedef AttunePlayerStopC = Int32 Function();
typedef AttunePlayerStopDart = int Function();

/// High-performance Dart FFI bridge to the native Rust `attune-core` & `attune-ffi` engine.
/// Exposes 5-layer audio profiling, transition recipe planning, SQLite cache analytics,
/// and real-time dual-deck hardware playback control.
class AttuneEngineBridge {
  static AttuneEngineBridge? _instance;
  static AttuneEngineBridge get instance => _instance ??= AttuneEngineBridge._init();

  DynamicLibrary? _lib;
  bool _isInitialized = false;

  AttuneInitDart? _attuneInit;
  AttuneVersionDart? _attuneVersion;
  AttuneFreeStringDart? _attuneFreeString;
  AttuneProfileFileDart? _attuneProfileFile;
  AttunePlanTransitionDart? _attunePlanTransition;
  AttuneCacheStatsDart? _attuneCacheStats;

  AttunePlayerStartDart? _attunePlayerStart;
  AttunePlayerLoadTrackDart? _attunePlayerLoadTrack;
  AttunePlayerPlayDart? _attunePlayerPlay;
  AttunePlayerPauseDart? _attunePlayerPause;
  AttunePlayerSeekDart? _attunePlayerSeek;
  AttunePlayerTriggerTransitionDart? _attunePlayerTriggerTransition;
  AttunePlayerGetStateDart? _attunePlayerGetState;
  AttunePlayerStopDart? _attunePlayerStop;

  AttuneEngineBridge._init() {
    _loadLibrary();
  }

  bool get isAvailable => _lib != null && _isInitialized;

  void _loadLibrary() {
    try {
      if (Platform.isLinux) {
        // Search workspace build targets first during development
        final candidatePaths = [
          '../../target/debug/libattune_engine.so',
          '../../target/release/libattune_engine.so',
          '/home/miku/Documents/attune/target/debug/libattune_engine.so',
          '/home/miku/Documents/attune/target/release/libattune_engine.so',
          'libattune_engine.so',
        ];
        for (final path in candidatePaths) {
          if (File(path).existsSync()) {
            _lib = DynamicLibrary.open(path);
            break;
          }
        }
        _lib ??= DynamicLibrary.open('libattune_engine.so');
      } else if (Platform.isAndroid) {
        _lib = DynamicLibrary.open('libattune_engine.so');
      } else if (Platform.isMacOS) {
        _lib = DynamicLibrary.open('libattune_engine.dylib');
      } else if (Platform.isWindows) {
        _lib = DynamicLibrary.open('attune_engine.dll');
      }

      if (_lib != null) {
        _attuneInit = _lib!.lookupFunction<AttuneInitC, AttuneInitDart>('attune_init');
        _attuneVersion = _lib!.lookupFunction<AttuneVersionC, AttuneVersionDart>('attune_version');
        _attuneFreeString = _lib!.lookupFunction<AttuneFreeStringC, AttuneFreeStringDart>('attune_free_string');
        _attuneProfileFile = _lib!.lookupFunction<AttuneProfileFileC, AttuneProfileFileDart>('attune_profile_file');
        _attunePlanTransition =
            _lib!.lookupFunction<AttunePlanTransitionC, AttunePlanTransitionDart>('attune_plan_transition');
        _attuneCacheStats = _lib!.lookupFunction<AttuneCacheStatsC, AttuneCacheStatsDart>('attune_cache_stats');

        _attunePlayerStart = _lib!.lookupFunction<AttunePlayerStartC, AttunePlayerStartDart>('attune_player_start');
        _attunePlayerLoadTrack =
            _lib!.lookupFunction<AttunePlayerLoadTrackC, AttunePlayerLoadTrackDart>('attune_player_load_track');
        _attunePlayerPlay = _lib!.lookupFunction<AttunePlayerPlayC, AttunePlayerPlayDart>('attune_player_play');
        _attunePlayerPause = _lib!.lookupFunction<AttunePlayerPauseC, AttunePlayerPauseDart>('attune_player_pause');
        _attunePlayerSeek = _lib!.lookupFunction<AttunePlayerSeekC, AttunePlayerSeekDart>('attune_player_seek');
        _attunePlayerTriggerTransition = _lib!.lookupFunction<AttunePlayerTriggerTransitionC,
            AttunePlayerTriggerTransitionDart>('attune_player_trigger_transition');
        _attunePlayerGetState =
            _lib!.lookupFunction<AttunePlayerGetStateC, AttunePlayerGetStateDart>('attune_player_get_state');
        _attunePlayerStop = _lib!.lookupFunction<AttunePlayerStopC, AttunePlayerStopDart>('attune_player_stop');

        _attuneInit?.call();
        _isInitialized = true;
        debugPrint('[AttuneEngineBridge] Successfully loaded native engine v$version');
      }
    } catch (e) {
      debugPrint('[AttuneEngineBridge] Native engine not loaded (running in fallback/mock mode): $e');
    }
  }

  /// Engine version query
  String get version {
    if (!isAvailable || _attuneVersion == null || _attuneFreeString == null) {
      return '0.1.0-simulated';
    }
    final ptr = _attuneVersion!();
    if (ptr.address == 0) return '0.1.0-simulated';
    final str = ptr.toDartString();
    _attuneFreeString!(ptr);
    return str;
  }

  /// Profiles an audio file and returns its parsed TrackAnalysisProfile (TAP)
  Future<Map<String, dynamic>?> profileTrack(String filePath, {int tier = 1}) async {
    if (!isAvailable || _attuneProfileFile == null || _attuneFreeString == null) {
      return null;
    }
    final pathPtr = filePath.toNativeUtf8();
    try {
      final resPtr = _attuneProfileFile!(pathPtr, tier);
      if (resPtr.address == 0) return null;
      final jsonStr = resPtr.toDartString();
      _attuneFreeString!(resPtr);
      return jsonDecode(jsonStr) as Map<String, dynamic>;
    } finally {
      calloc.free(pathPtr);
    }
  }

  /// Plans a seamless transition recipe between two TrackAnalysisProfiles
  Future<Map<String, dynamic>?> planTransition(
    Map<String, dynamic> tapA,
    Map<String, dynamic> tapB,
  ) async {
    if (!isAvailable || _attunePlanTransition == null || _attuneFreeString == null) {
      return null;
    }
    final jsonA = jsonEncode(tapA).toNativeUtf8();
    final jsonB = jsonEncode(tapB).toNativeUtf8();
    try {
      final resPtr = _attunePlanTransition!(jsonA, jsonB);
      if (resPtr.address == 0) return null;
      final jsonStr = resPtr.toDartString();
      _attuneFreeString!(resPtr);
      return jsonDecode(jsonStr) as Map<String, dynamic>;
    } finally {
      calloc.free(jsonA);
      calloc.free(jsonB);
    }
  }

  /// Queries SQLite cache statistics
  Future<Map<String, dynamic>?> getCacheStats(String dbPath) async {
    if (!isAvailable || _attuneCacheStats == null || _attuneFreeString == null) {
      return null;
    }
    final pathPtr = dbPath.toNativeUtf8();
    try {
      final resPtr = _attuneCacheStats!(pathPtr);
      if (resPtr.address == 0) return null;
      final jsonStr = resPtr.toDartString();
      _attuneFreeString!(resPtr);
      return jsonDecode(jsonStr) as Map<String, dynamic>;
    } finally {
      calloc.free(pathPtr);
    }
  }

  // Real-Time Playback Engine Controls

  int playerStart() {
    if (!isAvailable || _attunePlayerStart == null) return -1;
    return _attunePlayerStart!();
  }

  int playerLoadTrack(int deckId, String filePath) {
    if (!isAvailable || _attunePlayerLoadTrack == null) return -1;
    final pathPtr = filePath.toNativeUtf8();
    try {
      return _attunePlayerLoadTrack!(deckId, pathPtr);
    } finally {
      calloc.free(pathPtr);
    }
  }

  int playerPlay() {
    if (!isAvailable || _attunePlayerPlay == null) return -1;
    return _attunePlayerPlay!();
  }

  int playerPause() {
    if (!isAvailable || _attunePlayerPause == null) return -1;
    return _attunePlayerPause!();
  }

  int playerSeek(int deckId, int positionMs) {
    if (!isAvailable || _attunePlayerSeek == null) return -1;
    return _attunePlayerSeek!(deckId, positionMs);
  }

  int playerTriggerTransition(Map<String, dynamic> recipe) {
    if (!isAvailable || _attunePlayerTriggerTransition == null) return -1;
    final jsonPtr = jsonEncode(recipe).toNativeUtf8();
    try {
      return _attunePlayerTriggerTransition!(jsonPtr);
    } finally {
      calloc.free(jsonPtr);
    }
  }

  Map<String, dynamic>? playerGetState() {
    if (!isAvailable || _attunePlayerGetState == null || _attuneFreeString == null) {
      return null;
    }
    final ptr = _attunePlayerGetState!();
    if (ptr.address == 0) return null;
    final str = ptr.toDartString();
    _attuneFreeString!(ptr);
    try {
      return jsonDecode(str) as Map<String, dynamic>;
    } catch (_) {
      return null;
    }
  }

  int playerStop() {
    if (!isAvailable || _attunePlayerStop == null) return -1;
    return _attunePlayerStop!();
  }
}
