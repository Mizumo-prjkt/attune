import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/services/settings_service.dart';
import 'package:attune_app/services/system_audio_player_service.dart';
import 'package:attune_app/services/watchguard_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('WatchguardService Tests', () {
    setUp(() {
      WatchguardService.instance.resetMetrics();
    });

    test('Initial state is stable with zero clipping and zero underruns', () {
      final status = WatchguardService.instance.currentStatus;
      expect(status.clippingCount, 0);
      expect(status.underrunCount, 0);
      expect(status.isOverloaded, false);
      expect(WatchguardService.instance.isOverloaded, false);
    });

    test('Recording multiple clipping and underrun events triggers overload status', () {
      WatchguardService.instance.recordAudioEvent(
        peakAmplitude: 1.05,
        isUnderrun: false,
        isClipping: true,
      );
      WatchguardService.instance.recordAudioEvent(
        peakAmplitude: 1.02,
        isUnderrun: true,
        isClipping: true,
      );

      final status = WatchguardService.instance.currentStatus;
      expect(status.clippingCount, 2);
      expect(status.underrunCount, 1);
      expect(status.isOverloaded, true);
      expect(status.warningMessage, isNotNull);
      expect(status.recommendSystemCodec, true);
    });

    test('Recording UI frames calculates average FPS and tracks spikes', () {
      WatchguardService.instance.recordUiFrame(16.6);
      WatchguardService.instance.recordUiFrame(16.7);
      WatchguardService.instance.recordUiFrame(33.4);

      final status = WatchguardService.instance.currentStatus;
      expect(status.averageFps, greaterThan(20.0));
      expect(status.peakFrameDurationMs, greaterThanOrEqualTo(33.0));
    });

    test('resetMetrics restores optimal stable state', () {
      WatchguardService.instance.recordAudioEvent(
        peakAmplitude: 1.0,
        isUnderrun: true,
        isClipping: true,
      );
      WatchguardService.instance.recordAudioEvent(
        peakAmplitude: 1.0,
        isUnderrun: true,
        isClipping: true,
      );
      expect(WatchguardService.instance.isOverloaded, true);

      WatchguardService.instance.resetMetrics();
      expect(WatchguardService.instance.isOverloaded, false);
      expect(WatchguardService.instance.currentStatus.clippingCount, 0);
      expect(WatchguardService.instance.currentStatus.underrunCount, 0);
    });
  });

  group('SystemAudioPlayerService Mock Engine Tests', () {
    test('Load, play, pause, seek lifecycle on mock engine', () async {
      final service = SystemAudioPlayerService.instance;
      final ok = await service.load('/mock/test/track.mp3');
      expect(ok, true);
      expect(service.currentPath, '/mock/test/track.mp3');
      expect(service.durationSec, greaterThan(0.0));

      final played = await service.play();
      expect(played, true);
      expect(service.isPlaying, true);

      final sought = await service.seek(const Duration(seconds: 15));
      expect(sought, true);
      expect(service.positionSec, 15.0);

      final paused = await service.pause();
      expect(paused, true);
      expect(service.isPlaying, false);

      final stopped = await service.stop();
      expect(stopped, true);
      expect(service.isPlaying, false);
      expect(service.positionSec, 0.0);
    });
  });

  group('PlaybackController Decoder & Smart Shuffle Enforcement Tests', () {
    setUp(() {
      SettingsService.instance.setDecoderMode(AudioDecoderMode.attuneRust);
      PlaybackController.instance.setShuffleMode(ShuffleMode.off);
      WatchguardService.instance.resetMetrics();
    });

    test('Smart Shuffle is supported on Attune Rust engine', () {
      expect(PlaybackController.instance.isSmartShuffleSupported, true);
      PlaybackController.instance.setShuffleMode(ShuffleMode.smart);
      expect(PlaybackController.instance.shuffleMode, ShuffleMode.smart);
    });

    test('Switching to System Default strictly disables and blocks Smart Shuffle', () async {
      PlaybackController.instance.setShuffleMode(ShuffleMode.smart);
      expect(PlaybackController.instance.shuffleMode, ShuffleMode.smart);

      // Switch to System Default Codec
      await PlaybackController.instance.switchDecoderMode(AudioDecoderMode.systemDefault);
      expect(PlaybackController.instance.isSmartShuffleSupported, false);
      // Smart Shuffle should be automatically downshifted to standard shuffle
      expect(PlaybackController.instance.shuffleMode, ShuffleMode.standard);

      // Attempting to set Smart Shuffle directly must be blocked and clamped to standard
      PlaybackController.instance.setShuffleMode(ShuffleMode.smart);
      expect(PlaybackController.instance.shuffleMode, ShuffleMode.standard);

      // Cycling shuffle mode in System Default cycles between Off and Standard only
      PlaybackController.instance.cycleShuffleMode(); // from standard -> off
      expect(PlaybackController.instance.shuffleMode, ShuffleMode.off);

      PlaybackController.instance.cycleShuffleMode(); // from off -> standard
      expect(PlaybackController.instance.shuffleMode, ShuffleMode.standard);

      PlaybackController.instance.cycleShuffleMode(); // from standard -> off (skips smart)
      expect(PlaybackController.instance.shuffleMode, ShuffleMode.off);
    });

    test('System overload state triggers alert and can be dismissed', () {
      expect(PlaybackController.instance.isSystemOverloaded, false);

      // Generate overload in Watchguard
      WatchguardService.instance.recordAudioEvent(
        peakAmplitude: 1.0,
        isUnderrun: true,
        isClipping: true,
      );
      WatchguardService.instance.recordAudioEvent(
        peakAmplitude: 1.0,
        isUnderrun: true,
        isClipping: true,
      );

      expect(WatchguardService.instance.isOverloaded, true);
      expect(PlaybackController.instance.isSystemOverloaded, true);

      // User dismisses overload alert
      PlaybackController.instance.dismissOverloadAlert();
      expect(PlaybackController.instance.isSystemOverloaded, false);
    });
  });
}
