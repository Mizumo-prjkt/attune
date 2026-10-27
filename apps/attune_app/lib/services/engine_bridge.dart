import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';
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

typedef AttuneMasterFxSetEnabledC = Int32 Function(Int32);
typedef AttuneMasterFxSetEnabledDart = int Function(int);

typedef AttuneMasterFxSetPreampDbC = Int32 Function(Float);
typedef AttuneMasterFxSetPreampDbDart = int Function(double);

typedef AttuneMasterFxSetBandGainC = Int32 Function(Int32, Float);
typedef AttuneMasterFxSetBandGainDart = int Function(int, double);

typedef AttuneMasterFxSetAllBandsC = Int32 Function(Pointer<Float>, Int32);
typedef AttuneMasterFxSetAllBandsDart = int Function(Pointer<Float>, int);

typedef AttuneMasterFxSetBassC = Int32 Function(Int32, Float, Float);
typedef AttuneMasterFxSetBassDart = int Function(int, double, double);

typedef AttuneMasterFxSetClarityC = Int32 Function(Int32, Float);
typedef AttuneMasterFxSetClarityDart = int Function(int, double);

typedef AttuneMasterFxSetStereoWidthC = Int32 Function(Float);
typedef AttuneMasterFxSetStereoWidthDart = int Function(double);

typedef AttuneMasterFxSetParamsJsonC = Int32 Function(Pointer<Utf8>);
typedef AttuneMasterFxSetParamsJsonDart = int Function(Pointer<Utf8>);

typedef AttuneMasterFxGetParamsJsonC = Pointer<Utf8> Function();
typedef AttuneMasterFxGetParamsJsonDart = Pointer<Utf8> Function();

typedef AttuneProbeExternalDspC = Pointer<Utf8> Function();
typedef AttuneProbeExternalDspDart = Pointer<Utf8> Function();

/// High-performance Dart FFI bridge to the native Rust `attune-core` & `attune-ffi` engine.
/// Exposes 5-layer audio profiling, transition recipe planning, SQLite cache analytics,
/// and real-time dual-deck hardware playback control.
class AttuneEngineBridge {
  static AttuneEngineBridge? _instance;
  static AttuneEngineBridge get instance => _instance ??= AttuneEngineBridge._init();

  DynamicLibrary? _lib;
  String? _resolvedLibraryPath;
  String? get resolvedLibraryPath => _resolvedLibraryPath;
  bool _isInitialized = false;

  AttuneInitDart? _attuneInit;
  AttuneVersionDart? _attuneVersion;
  AttuneFreeStringDart? _attuneFreeString;
  AttuneCacheStatsDart? _attuneCacheStats;

  AttunePlayerStartDart? _attunePlayerStart;
  AttunePlayerLoadTrackDart? _attunePlayerLoadTrack;
  AttunePlayerPlayDart? _attunePlayerPlay;
  AttunePlayerPauseDart? _attunePlayerPause;
  AttunePlayerSeekDart? _attunePlayerSeek;
  AttunePlayerTriggerTransitionDart? _attunePlayerTriggerTransition;
  AttunePlayerGetStateDart? _attunePlayerGetState;
  AttunePlayerStopDart? _attunePlayerStop;

  AttuneMasterFxSetEnabledDart? _attuneMasterFxSetEnabled;
  AttuneMasterFxSetPreampDbDart? _attuneMasterFxSetPreampDb;
  AttuneMasterFxSetBandGainDart? _attuneMasterFxSetBandGain;
  AttuneMasterFxSetAllBandsDart? _attuneMasterFxSetAllBands;
  AttuneMasterFxSetBassDart? _attuneMasterFxSetBass;
  AttuneMasterFxSetClarityDart? _attuneMasterFxSetClarity;
  AttuneMasterFxSetStereoWidthDart? _attuneMasterFxSetStereoWidth;
  AttuneMasterFxSetParamsJsonDart? _attuneMasterFxSetParamsJson;
  AttuneMasterFxGetParamsJsonDart? _attuneMasterFxGetParamsJson;
  AttuneProbeExternalDspDart? _attuneProbeExternalDsp;

  AttuneEngineBridge._init() {
    _loadLibrary();
  }

  bool get isAvailable => _lib != null && _isInitialized;

  void _loadLibrary() {
    try {
      if (Platform.isLinux) {
        // Search workspace build targets first during development (favoring release)
        final candidatePaths = [
          '../../target/release/libattune_engine.so',
          '../../target/debug/libattune_engine.so',
          '/home/miku/Documents/attune/target/release/libattune_engine.so',
          '/home/miku/Documents/attune/target/debug/libattune_engine.so',
          'lib/libattune_engine.so',
          'libattune_engine.so',
        ];
        for (final path in candidatePaths) {
          if (File(path).existsSync()) {
            _resolvedLibraryPath = path;
            _lib = DynamicLibrary.open(path);
            break;
          }
        }
        if (_lib == null) {
          _resolvedLibraryPath = 'libattune_engine.so';
          _lib = DynamicLibrary.open('libattune_engine.so');
        }
      } else if (Platform.isAndroid) {
        _resolvedLibraryPath = 'libattune_engine.so';
        _lib = DynamicLibrary.open('libattune_engine.so');
      } else if (Platform.isMacOS) {
        _resolvedLibraryPath = 'libattune_engine.dylib';
        _lib = DynamicLibrary.open('libattune_engine.dylib');
      } else if (Platform.isWindows) {
        _resolvedLibraryPath = 'attune_engine.dll';
        _lib = DynamicLibrary.open('attune_engine.dll');
      }

      if (_lib != null) {
        _attuneInit = _lib!.lookupFunction<AttuneInitC, AttuneInitDart>('attune_init');
        _attuneVersion = _lib!.lookupFunction<AttuneVersionC, AttuneVersionDart>('attune_version');
        _attuneFreeString = _lib!.lookupFunction<AttuneFreeStringC, AttuneFreeStringDart>('attune_free_string');
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

        _attuneMasterFxSetEnabled =
            _lib!.lookupFunction<AttuneMasterFxSetEnabledC, AttuneMasterFxSetEnabledDart>('attune_master_fx_set_enabled');
        _attuneMasterFxSetPreampDb =
            _lib!.lookupFunction<AttuneMasterFxSetPreampDbC, AttuneMasterFxSetPreampDbDart>('attune_master_fx_set_preamp_db');
        _attuneMasterFxSetBandGain =
            _lib!.lookupFunction<AttuneMasterFxSetBandGainC, AttuneMasterFxSetBandGainDart>('attune_master_fx_set_band_gain');
        _attuneMasterFxSetAllBands =
            _lib!.lookupFunction<AttuneMasterFxSetAllBandsC, AttuneMasterFxSetAllBandsDart>('attune_master_fx_set_all_bands');
        _attuneMasterFxSetBass =
            _lib!.lookupFunction<AttuneMasterFxSetBassC, AttuneMasterFxSetBassDart>('attune_master_fx_set_bass');
        _attuneMasterFxSetClarity =
            _lib!.lookupFunction<AttuneMasterFxSetClarityC, AttuneMasterFxSetClarityDart>('attune_master_fx_set_clarity');
        _attuneMasterFxSetStereoWidth =
            _lib!.lookupFunction<AttuneMasterFxSetStereoWidthC, AttuneMasterFxSetStereoWidthDart>('attune_master_fx_set_stereo_width');
        _attuneMasterFxSetParamsJson =
            _lib!.lookupFunction<AttuneMasterFxSetParamsJsonC, AttuneMasterFxSetParamsJsonDart>('attune_master_fx_set_params_json');
        _attuneMasterFxGetParamsJson =
            _lib!.lookupFunction<AttuneMasterFxGetParamsJsonC, AttuneMasterFxGetParamsJsonDart>('attune_master_fx_get_params_json');
        _attuneProbeExternalDsp =
            _lib!.lookupFunction<AttuneProbeExternalDspC, AttuneProbeExternalDspDart>('attune_probe_external_dsp');

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

  /// Profiles an audio file and returns its parsed TrackAnalysisProfile (TAP) asynchronously on a background worker
  Future<Map<String, dynamic>?> profileTrack(String filePath, {int tier = 1}) async {
    if (!isAvailable) return null;
    final libPath = _resolvedLibraryPath;
    if (libPath == null) return null;

    try {
      return await Isolate.run(() {
        final lib = DynamicLibrary.open(libPath);
        final profileFn = lib.lookupFunction<AttuneProfileFileC, AttuneProfileFileDart>('attune_profile_file');
        final freeFn = lib.lookupFunction<AttuneFreeStringC, AttuneFreeStringDart>('attune_free_string');

        final pathPtr = filePath.toNativeUtf8();
        try {
          final resPtr = profileFn(pathPtr, tier);
          if (resPtr.address == 0) return null;
          final jsonStr = resPtr.toDartString();
          freeFn(resPtr);
          return jsonDecode(jsonStr) as Map<String, dynamic>;
        } finally {
          calloc.free(pathPtr);
        }
      });
    } catch (e) {
      debugPrint('[AttuneEngineBridge] Background profileTrack notice: $e');
      return null;
    }
  }

  /// Plans a seamless transition recipe between two TrackAnalysisProfiles asynchronously on a background worker
  Future<Map<String, dynamic>?> planTransition(
    Map<String, dynamic> tapA,
    Map<String, dynamic> tapB,
  ) async {
    if (!isAvailable) return null;
    final libPath = _resolvedLibraryPath;
    if (libPath == null) return null;

    final jsonA = jsonEncode(tapA);
    final jsonB = jsonEncode(tapB);

    try {
      return await Isolate.run(() {
        final lib = DynamicLibrary.open(libPath);
        final planFn = lib.lookupFunction<AttunePlanTransitionC, AttunePlanTransitionDart>('attune_plan_transition');
        final freeFn = lib.lookupFunction<AttuneFreeStringC, AttuneFreeStringDart>('attune_free_string');

        final aPtr = jsonA.toNativeUtf8();
        final bPtr = jsonB.toNativeUtf8();
        try {
          final resPtr = planFn(aPtr, bPtr);
          if (resPtr.address == 0) return null;
          final jsonStr = resPtr.toDartString();
          freeFn(resPtr);
          return jsonDecode(jsonStr) as Map<String, dynamic>;
        } finally {
          calloc.free(aPtr);
          calloc.free(bPtr);
        }
      });
    } catch (e) {
      debugPrint('[AttuneEngineBridge] Background planTransition notice: $e');
      return null;
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

  // Attune Studio Master FX Rack Controls

  int masterFxSetEnabled(bool enabled) {
    if (!isAvailable || _attuneMasterFxSetEnabled == null) return -1;
    return _attuneMasterFxSetEnabled!(enabled ? 1 : 0);
  }

  int masterFxSetPreampDb(double preampDb) {
    if (!isAvailable || _attuneMasterFxSetPreampDb == null) return -1;
    return _attuneMasterFxSetPreampDb!(preampDb);
  }

  int masterFxSetBandGain(int bandIdx, double gainDb) {
    if (!isAvailable || _attuneMasterFxSetBandGain == null) return -1;
    return _attuneMasterFxSetBandGain!(bandIdx, gainDb);
  }

  int masterFxSetAllBands(List<double> gainsDb) {
    if (!isAvailable || _attuneMasterFxSetAllBands == null) return -1;
    final ptr = calloc<Float>(gainsDb.length);
    try {
      for (var i = 0; i < gainsDb.length; i++) {
        ptr[i] = gainsDb[i];
      }
      return _attuneMasterFxSetAllBands!(ptr, gainsDb.length);
    } finally {
      calloc.free(ptr);
    }
  }

  int masterFxSetBass(bool enabled, double gainDb, double freqHz) {
    if (!isAvailable || _attuneMasterFxSetBass == null) return -1;
    return _attuneMasterFxSetBass!(enabled ? 1 : 0, gainDb, freqHz);
  }

  int masterFxSetClarity(bool enabled, double wet) {
    if (!isAvailable || _attuneMasterFxSetClarity == null) return -1;
    return _attuneMasterFxSetClarity!(enabled ? 1 : 0, wet);
  }

  int masterFxSetStereoWidth(double width) {
    if (!isAvailable || _attuneMasterFxSetStereoWidth == null) return -1;
    return _attuneMasterFxSetStereoWidth!(width);
  }

  int masterFxSetParamsJson(Map<String, dynamic> params) {
    if (!isAvailable || _attuneMasterFxSetParamsJson == null) return -1;
    final jsonPtr = jsonEncode(params).toNativeUtf8();
    try {
      return _attuneMasterFxSetParamsJson!(jsonPtr);
    } finally {
      calloc.free(jsonPtr);
    }
  }

  Map<String, dynamic>? masterFxGetParams() {
    if (!isAvailable || _attuneMasterFxGetParamsJson == null || _attuneFreeString == null) {
      return null;
    }
    final ptr = _attuneMasterFxGetParamsJson!();
    if (ptr.address == 0) return null;
    final str = ptr.toDartString();
    _attuneFreeString!(ptr);
    try {
      return jsonDecode(str) as Map<String, dynamic>;
    } catch (_) {
      return null;
    }
  }

  Map<String, dynamic>? probeExternalDsp() {
    if (!isAvailable || _attuneProbeExternalDsp == null || _attuneFreeString == null) {
      return null;
    }
    final ptr = _attuneProbeExternalDsp!();
    if (ptr.address == 0) return null;
    final str = ptr.toDartString();
    _attuneFreeString!(ptr);
    try {
      return jsonDecode(str) as Map<String, dynamic>;
    } catch (_) {
      return null;
    }
  }
}

