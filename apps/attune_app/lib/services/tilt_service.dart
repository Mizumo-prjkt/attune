import 'dart:async';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'settings_service.dart';

/// Service managing hardware accelerometer tilt on Android and cursor parallax on Desktop.
/// Strictly enforces performance preservation by stopping all hardware sensor streams
/// when the active stage is out of view (e.g. Now Playing sheet open, Settings open, or other tab).
class TiltService extends ChangeNotifier {
  static final TiltService instance = TiltService._internal();
  TiltService._internal() {
    SettingsService.instance.addListener(_onSettingsChanged);
  }

  static const EventChannel _sensorChannel =
      EventChannel('org.mizproject.attune/sensors/accelerometer');

  StreamSubscription? _sensorSub;
  double _tiltX = 0.0;
  double _tiltY = 0.0;
  bool _isInView = false;
  int _activeViewersCount = 0;

  double get tiltX => _tiltX;
  double get tiltY => _tiltY;
  bool get isSensorActive => _sensorSub != null;
  int get activeViewersCount => _activeViewersCount;
  bool get isStageInView => _isInView;

  void _onSettingsChanged() {
    if (!SettingsService.instance.liquidGlass3dMirror) {
      _stopSensors();
      _tiltX = 0.0;
      _tiltY = 0.0;
      notifyListeners();
    } else if (_isInView) {
      _startSensors();
    }
  }

  /// Registers an active glass view that requires tilt updates.
  /// Automatically starts hardware sensors on Android if first viewer.
  void registerViewer() {
    _activeViewersCount++;
    if (_activeViewersCount > 0 && !_isInView) {
      _isInView = true;
      _startSensors();
    }
  }

  /// Unregisters an active glass view.
  /// Automatically halts hardware sensors on Android when count reaches zero.
  void unregisterViewer() {
    _activeViewersCount = (_activeViewersCount - 1).clamp(0, 9999);
    if (_activeViewersCount == 0 && _isInView) {
      _isInView = false;
      _stopSensors();
      _tiltX = 0.0;
      _tiltY = 0.0;
      notifyListeners();
    }
  }

  /// Sets whether the host scaffold stage is currently in view
  /// (e.g. false when Now Playing is open or Settings is presented).
  void setStageInView(bool inView) {
    if (_isInView == inView) return;
    _isInView = inView;
    if (_isInView && _activeViewersCount > 0) {
      _startSensors();
    } else {
      _stopSensors();
      _tiltX = 0.0;
      _tiltY = 0.0;
      notifyListeners();
    }
  }

  void _startSensors() {
    if (kIsWeb || !Platform.isAndroid) return;
    if (!SettingsService.instance.liquidGlass3dMirror) return;
    if (_sensorSub != null) return;

    try {
      _sensorSub = _sensorChannel.receiveBroadcastStream().listen(
        (data) {
          if (data is List && data.length >= 2) {
            final x = (data[0] as num).toDouble().clamp(-1.0, 1.0);
            final y = (data[1] as num).toDouble().clamp(-1.0, 1.0);
            _tiltX = x;
            _tiltY = y;
            notifyListeners();
          }
        },
        onError: (err) {
          debugPrint('[TiltService] Accelerometer stream error: $err');
          _stopSensors();
        },
        cancelOnError: true,
      );
    } catch (e) {
      debugPrint('[TiltService] Failed to bind accelerometer: $e');
    }
  }

  void _stopSensors() {
    _sensorSub?.cancel();
    _sensorSub = null;
  }

  /// Updates tilt on Desktop / Web based on cursor location relative to widget bounds.
  void updatePointer(Offset localOffset, Size size) {
    if (!SettingsService.instance.liquidGlass3dMirror) return;
    if (size.width <= 0 || size.height <= 0) return;

    final normX = ((localOffset.dx / size.width) - 0.5) * 2.0;
    final normY = ((localOffset.dy / size.height) - 0.5) * 2.0;

    _tiltX = normX.clamp(-1.0, 1.0);
    _tiltY = normY.clamp(-1.0, 1.0);
    notifyListeners();
  }

  /// Resets cursor tilt when mouse leaves widget area.
  void resetPointer() {
    if (_tiltX == 0.0 && _tiltY == 0.0) return;
    _tiltX = 0.0;
    _tiltY = 0.0;
    notifyListeners();
  }

  @override
  void dispose() {
    _stopSensors();
    SettingsService.instance.removeListener(_onSettingsChanged);
    super.dispose();
  }
}
