import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';

// Native function typedefs for libattune_watchguard.so
typedef WatchguardInitC = Int32 Function();
typedef WatchguardInitDart = int Function();

typedef WatchguardVersionC = Pointer<Utf8> Function();
typedef WatchguardVersionDart = Pointer<Utf8> Function();

typedef WatchguardRecordAudioEventC = Void Function(Float, Int32, Int32);
typedef WatchguardRecordAudioEventDart = void Function(double, int, int);

typedef WatchguardRecordUiFrameC = Void Function(Float);
typedef WatchguardRecordUiFrameDart = void Function(double);

typedef WatchguardGetStatusJsonC = Pointer<Utf8> Function();
typedef WatchguardGetStatusJsonDart = Pointer<Utf8> Function();

typedef WatchguardResetMetricsC = Int32 Function();
typedef WatchguardResetMetricsDart = int Function();

typedef WatchguardFreeStringC = Void Function(Pointer<Utf8>);
typedef WatchguardFreeStringDart = void Function(Pointer<Utf8>);

/// Structured snapshot of device stability, audio clipping, and performance telemetry
class WatchguardStabilityStatus {
  final bool isOverloaded;
  final double instabilityScore;
  final int clippingCount;
  final int underrunCount;
  final int recentClippingEvents;
  final int recentUnderrunEvents;
  final double averageFps;
  final double droppedFrameRatio;
  final double peakFrameDurationMs;
  final String? warningMessage;
  final bool recommendSystemCodec;
  final int timestampMs;

  const WatchguardStabilityStatus({
    this.isOverloaded = false,
    this.instabilityScore = 0.0,
    this.clippingCount = 0,
    this.underrunCount = 0,
    this.recentClippingEvents = 0,
    this.recentUnderrunEvents = 0,
    this.averageFps = 60.0,
    this.droppedFrameRatio = 0.0,
    this.peakFrameDurationMs = 16.6,
    this.warningMessage,
    this.recommendSystemCodec = false,
    this.timestampMs = 0,
  });

  factory WatchguardStabilityStatus.fromJson(Map<String, dynamic> json) {
    return WatchguardStabilityStatus(
      isOverloaded: json['is_overloaded'] as bool? ?? false,
      instabilityScore: (json['instability_score'] as num?)?.toDouble() ?? 0.0,
      clippingCount: (json['clipping_count'] as num?)?.toInt() ?? 0,
      underrunCount: (json['underrun_count'] as num?)?.toInt() ?? 0,
      recentClippingEvents: (json['recent_clipping_events'] as num?)?.toInt() ?? 0,
      recentUnderrunEvents: (json['recent_underrun_events'] as num?)?.toInt() ?? 0,
      averageFps: (json['average_fps'] as num?)?.toDouble() ?? 60.0,
      droppedFrameRatio: (json['dropped_frame_ratio'] as num?)?.toDouble() ?? 0.0,
      peakFrameDurationMs: (json['peak_frame_duration_ms'] as num?)?.toDouble() ?? 16.6,
      warningMessage: json['warning_message'] as String?,
      recommendSystemCodec: json['recommend_system_codec'] as bool? ?? false,
      timestampMs: (json['timestamp_ms'] as num?)?.toInt() ?? 0,
    );
  }
}

/// Real-time Watchguard service monitoring audio clipping, buffer starvation,
/// UI frame timing drops, and device instability.
class WatchguardService extends ChangeNotifier {
  static final WatchguardService instance = WatchguardService._init();

  DynamicLibrary? _lib;
  bool _isNativeLoaded = false;
  bool get isNativeLoaded => _isNativeLoaded;

  WatchguardInitDart? _initFn;
  WatchguardVersionDart? _versionFn;
  WatchguardRecordAudioEventDart? _recordAudioEvent;
  WatchguardRecordUiFrameDart? _recordUiFrame;
  WatchguardGetStatusJsonDart? _getStatusJson;
  WatchguardResetMetricsDart? _resetMetrics;
  WatchguardFreeStringDart? _freeString;

  // Pure Dart fallback state for hermetic unit testing
  int _mockClippingTotal = 0;
  int _mockUnderrunTotal = 0;
  final List<double> _mockFrameDurations = [];

  WatchguardStabilityStatus _currentStatus = const WatchguardStabilityStatus();
  WatchguardStabilityStatus get currentStatus => _currentStatus;

  bool get isOverloaded => _currentStatus.isOverloaded;
  double get averageFps => _currentStatus.averageFps;
  String? get warningMessage => _currentStatus.warningMessage;
  bool get recommendSystemCodec => _currentStatus.recommendSystemCodec;

  String get version {
    if (_isNativeLoaded && _versionFn != null) {
      try {
        final ptr = _versionFn!();
        if (ptr != nullptr) {
          return ptr.toDartString();
        }
      } catch (_) {}
    }
    return '0.1.0';
  }

  Timer? _pollingTimer;
  bool _frameTimingsRegistered = false;

  WatchguardService._init() {
    _loadNativeLibrary();
    _registerFrameTimingsCallback();
    _startPolling();
  }

  void _loadNativeLibrary() {
    if (kIsWeb) return;
    try {
      if (Platform.isAndroid) {
        _lib = DynamicLibrary.open('libattune_watchguard.so');
      } else if (Platform.isLinux) {
        final candidates = [
          'libattune_watchguard.so',
          'lib/libattune_watchguard.so',
          '../../target/debug/libattune_watchguard.so',
          '../../target/release/libattune_watchguard.so',
          'target/debug/libattune_watchguard.so',
          'target/release/libattune_watchguard.so',
        ];
        for (final cand in candidates) {
          if (File(cand).existsSync()) {
            _lib = DynamicLibrary.open(cand);
            break;
          }
        }
        _lib ??= DynamicLibrary.open('libattune_watchguard.so');
      } else if (Platform.isWindows) {
        _lib = DynamicLibrary.open('attune_watchguard.dll');
      } else if (Platform.isMacOS) {
        _lib = DynamicLibrary.open('libattune_watchguard.dylib');
      }

      if (_lib != null) {
        final initFn = _lib!.lookupFunction<WatchguardInitC, WatchguardInitDart>('watchguard_init');
        initFn();

        _versionFn = _lib!.lookupFunction<WatchguardVersionC, WatchguardVersionDart>('watchguard_version');
        _recordAudioEvent = _lib!.lookupFunction<WatchguardRecordAudioEventC, WatchguardRecordAudioEventDart>('watchguard_record_audio_event');
        _recordUiFrame = _lib!.lookupFunction<WatchguardRecordUiFrameC, WatchguardRecordUiFrameDart>('watchguard_record_ui_frame');
        _getStatusJson = _lib!.lookupFunction<WatchguardGetStatusJsonC, WatchguardGetStatusJsonDart>('watchguard_get_status_json');
        _resetMetrics = _lib!.lookupFunction<WatchguardResetMetricsC, WatchguardResetMetricsDart>('watchguard_reset_metrics');
        _freeString = _lib!.lookupFunction<WatchguardFreeStringC, WatchguardFreeStringDart>('watchguard_free_string');
        _isNativeLoaded = true;
        debugPrint('[WatchguardService] Native watchguard library loaded successfully');
      }
    } catch (e) {
      debugPrint('[WatchguardService] Native watchguard not available (running in pure-Dart fallback): $e');
      _isNativeLoaded = false;
    }
  }

  void _registerFrameTimingsCallback() {
    if (_frameTimingsRegistered) return;
    try {
      SchedulerBinding.instance.addTimingsCallback((List<FrameTiming> timings) {
        for (final timing in timings) {
          final frameMs = timing.totalSpan.inMicroseconds / 1000.0;
          recordUiFrame(frameMs);
        }
      });
      _frameTimingsRegistered = true;
    } catch (_) {}
  }

  void recordUiFrame(double frameMs) {
    if (frameMs <= 0.0) return;
    if (_isNativeLoaded && _recordUiFrame != null) {
      try {
        _recordUiFrame!(frameMs);
        return;
      } catch (_) {}
    }

    // Fallback simulation
    if (_mockFrameDurations.length >= 60) {
      _mockFrameDurations.removeAt(0);
    }
    _mockFrameDurations.add(frameMs);
    _refreshStatus();
  }

  void recordAudioEvent({
    required double peakAmplitude,
    required bool isUnderrun,
    required bool isClipping,
  }) {
    if (_isNativeLoaded && _recordAudioEvent != null) {
      try {
        _recordAudioEvent!(peakAmplitude, isUnderrun ? 1 : 0, isClipping ? 1 : 0);
        _refreshStatus();
        return;
      } catch (_) {}
    }

    // Fallback simulation
    if (isClipping || peakAmplitude >= 0.995) {
      _mockClippingTotal++;
    }
    if (isUnderrun) {
      _mockUnderrunTotal++;
    }
    _refreshStatus();
  }

  void _startPolling() {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    _pollingTimer?.cancel();
    _pollingTimer = Timer.periodic(const Duration(seconds: 2), (_) {
      _refreshStatus();
    });
  }

  void _refreshStatus() {
    if (_isNativeLoaded && _getStatusJson != null && _freeString != null) {
      try {
        final ptr = _getStatusJson!();
        if (ptr != nullptr) {
          final jsonStr = ptr.toDartString();
          _freeString!(ptr);
          final map = jsonDecode(jsonStr) as Map<String, dynamic>;
          _currentStatus = WatchguardStabilityStatus.fromJson(map);
          notifyListeners();
          return;
        }
      } catch (e) {
        debugPrint('[WatchguardService] Error querying native status: $e');
      }
    }

    // Pure Dart evaluation for mock/test mode
    double totalMs = 0.0;
    double peakMs = 16.6;
    int dropped = 0;
    for (final d in _mockFrameDurations) {
      totalMs += d;
      if (d > peakMs) peakMs = d;
      if (d > 22.0) dropped++;
    }
    final avgMs = _mockFrameDurations.isNotEmpty ? totalMs / _mockFrameDurations.length : 16.6;
    final avgFps = avgMs > 0 ? (1000.0 / avgMs).clamp(1.0, 120.0) : 60.0;
    final droppedRatio = _mockFrameDurations.isNotEmpty ? dropped / _mockFrameDurations.length : 0.0;

    final isOverloaded = _mockClippingTotal >= 2 || _mockUnderrunTotal >= 2 || (droppedRatio > 0.40 && peakMs > 70.0);
    String? warning;
    if (_mockClippingTotal > 0 && _mockUnderrunTotal > 0) {
      warning = 'System Overloaded: Audio clipping & buffer starvation detected. Suggest switching to System Codec.';
    } else if (_mockClippingTotal > 0) {
      warning = 'System Overloaded: Audio clipping detected in native DSP engine.';
    } else if (_mockUnderrunTotal > 0) {
      warning = 'Audio Buffer Pressure: Buffer underruns detected on device hardware.';
    } else if (isOverloaded) {
      warning = 'Device Strain Detected: Frame drops and high system load. Recommend System Codec.';
    }

    _currentStatus = WatchguardStabilityStatus(
      isOverloaded: isOverloaded,
      instabilityScore: isOverloaded ? 0.65 : 0.0,
      clippingCount: _mockClippingTotal,
      underrunCount: _mockUnderrunTotal,
      recentClippingEvents: _mockClippingTotal,
      recentUnderrunEvents: _mockUnderrunTotal,
      averageFps: avgFps,
      droppedFrameRatio: droppedRatio,
      peakFrameDurationMs: peakMs,
      warningMessage: warning,
      recommendSystemCodec: isOverloaded,
      timestampMs: DateTime.now().millisecondsSinceEpoch,
    );
    notifyListeners();
  }

  void resetMetrics() {
    _mockClippingTotal = 0;
    _mockUnderrunTotal = 0;
    _mockFrameDurations.clear();
    if (_isNativeLoaded && _resetMetrics != null) {
      try {
        _resetMetrics!();
      } catch (_) {}
    }
    _refreshStatus();
  }

  @override
  void dispose() {
    _pollingTimer?.cancel();
    super.dispose();
  }
}
