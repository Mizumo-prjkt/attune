import 'dart:async';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'playback_controller.dart';
import 'settings_service.dart';

/// Service managing native Android lockscreen / status bar media notifications
/// and hardware overload alerts from Watchguard.
class AttuneNotificationService {
  static final AttuneNotificationService instance = AttuneNotificationService._internal();

  static const MethodChannel _channel = MethodChannel('org.mizproject.attune/notification');

  bool _initialized = false;
  String? _lastTitle;
  String? _lastArtist;
  bool? _lastIsPlaying;
  String? _lastAlbumArtPath;

  AttuneNotificationService._internal() {
    init();
  }

  void init() {
    if (_initialized) return;
    _initialized = true;

    if (!Platform.environment.containsKey('FLUTTER_TEST')) {
      try {
        if (Platform.isAndroid) {
          _channel.setMethodCallHandler(_handleMethodCall);
        }
      } catch (e) {
        debugPrint('[AttuneNotificationService] Initialization error: $e');
      }
    }
  }

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    switch (call.method) {
      case 'onPlaybackAction':
        final action = call.arguments as String?;
        debugPrint('[AttuneNotificationService] Action received from notification: $action');
        switch (action) {
          case 'play':
            PlaybackController.instance.play();
            break;
          case 'pause':
            PlaybackController.instance.pause();
            break;
          case 'togglePlayPause':
            PlaybackController.instance.togglePlayPause();
            break;
          case 'next':
            PlaybackController.instance.skipNext();
            break;
          case 'previous':
            PlaybackController.instance.skipPrevious();
            break;
        }
        break;

      case 'onSwitchCodec':
        debugPrint('[AttuneNotificationService] Switch to System Codec requested via notification');
        await PlaybackController.instance.switchDecoderMode(AudioDecoderMode.systemDefault);
        PlaybackController.instance.dismissOverloadAlert();
        await dismissOverloadAlert();
        break;

      default:
        debugPrint('[AttuneNotificationService] Unhandled method: ${call.method}');
        break;
    }
  }

  /// Displays or updates the persistent media playback notification on Android
  Future<void> showPlaybackNotification({
    required String title,
    required String artist,
    required bool isPlaying,
    String? albumArtPath,
  }) async {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    if (!Platform.isAndroid) return;

    if (_lastTitle == title &&
        _lastArtist == artist &&
        _lastIsPlaying == isPlaying &&
        _lastAlbumArtPath == albumArtPath) {
      return; // Deduplicate redundant notification calls
    }

    _lastTitle = title;
    _lastArtist = artist;
    _lastIsPlaying = isPlaying;
    _lastAlbumArtPath = albumArtPath;

    try {
      await _channel.invokeMethod('showPlaybackNotification', {
        'title': title,
        'artist': artist,
        'isPlaying': isPlaying,
        'albumArtPath': albumArtPath,
      });
    } catch (e) {
      debugPrint('[AttuneNotificationService] Failed to show playback notification: $e');
    }
  }

  /// Clears the media playback notification
  Future<void> hidePlaybackNotification() async {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    if (!Platform.isAndroid) return;

    _lastTitle = null;
    _lastArtist = null;
    _lastIsPlaying = null;
    _lastAlbumArtPath = null;

    try {
      await _channel.invokeMethod('hidePlaybackNotification');
    } catch (e) {
      debugPrint('[AttuneNotificationService] Failed to hide playback notification: $e');
    }
  }

  /// Posts a high-priority system notification alerting about audio buffer underruns,
  /// DSP clipping, or frame pacing drops with an actionable "Switch to System Codec" button.
  Future<void> showOverloadAlert({
    String title = 'System Overloaded: Audio Clipping',
    String message = 'DSP buffer underruns detected. Tap to switch to System Codec.',
    double score = 1.0,
  }) async {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    if (!Platform.isAndroid) return;

    try {
      await _channel.invokeMethod('showOverloadAlert', {
        'title': title,
        'message': message,
        'score': score,
      });
    } catch (e) {
      debugPrint('[AttuneNotificationService] Failed to show overload alert: $e');
    }
  }

  /// Dismisses the overload alert notification
  Future<void> dismissOverloadAlert() async {
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    if (!Platform.isAndroid) return;

    try {
      await _channel.invokeMethod('dismissOverloadAlert');
    } catch (e) {
      debugPrint('[AttuneNotificationService] Failed to dismiss overload alert: $e');
    }
  }
}
