import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/settings_service.dart';
import 'package:attune_app/services/engine_bridge.dart';
import 'package:attune_app/stages/settings_stage.dart';

void main() {
  group('SettingsService Unit Tests', () {
    test('Audio focus levels have correct display names and descriptive text', () {
      final service = SettingsService.instance;

      expect(AudioFocusLevel.standard.displayName, 'Standard (Default)');
      expect(AudioFocusLevel.sensitive.displayName, 'Sensitive');
      expect(AudioFocusLevel.ultraSensitive.displayName, 'Ultra Sensitive');
      expect(AudioFocusLevel.ahFuckAlerts.displayName, 'Ah f#ck alerts');
      expect(AudioFocusLevel.ahFuckEverything.displayName, 'Ah F#ck everything');

      expect(AudioFocusLevel.standard.description, contains('Pause music during calls'));
      expect(AudioFocusLevel.sensitive.description, contains('any sound poke immediately pauses'));
      expect(AudioFocusLevel.ultraSensitive.description, contains('shaking your device'));
      expect(AudioFocusLevel.ahFuckAlerts.description, contains('will not yield audio focus'));
      expect(AudioFocusLevel.ahFuckEverything.description, contains('also ignores incoming phone calls'));

      service.setAudioFocusLevel(AudioFocusLevel.ultraSensitive);
      expect(service.audioFocusLevel, AudioFocusLevel.ultraSensitive);

      service.setAudioFocusLevel(AudioFocusLevel.standard);
      expect(service.audioFocusLevel, AudioFocusLevel.standard);
    });

    test('Audio decoder modes toggle correctly', () {
      final service = SettingsService.instance;

      expect(AudioDecoderMode.attuneRust.displayName, contains("Attune's Rust Codec"));
      expect(
        AudioDecoderMode.androidDefault.displayName,
        Platform.isAndroid ? 'Android Default' : 'System Default',
      );

      service.setDecoderMode(AudioDecoderMode.androidDefault);
      expect(service.decoderMode, AudioDecoderMode.androidDefault);

      service.setDecoderMode(AudioDecoderMode.attuneRust);
      expect(service.decoderMode, AudioDecoderMode.attuneRust);
    });

    test('Equalizer bands adjust and reset to flat', () {
      final service = SettingsService.instance;

      service.setEqualizerMode(EqualizerMode.attuneEq);
      expect(service.equalizerMode, EqualizerMode.attuneEq);

      service.setEqBands(lowDb: 4.5, midDb: -2.0, highDb: 6.0);
      expect(service.eqLowDb, 4.5);
      expect(service.eqMidDb, -2.0);
      expect(service.eqHighDb, 6.0);

      service.resetEqualizer();
      expect(service.eqLowDb, 0.0);
      expect(service.eqMidDb, 0.0);
      expect(service.eqHighDb, 0.0);
    });

    test('Files limit timer clamps and updates correctly', () {
      final service = SettingsService.instance;

      service.setMinSongDurationSec(75);
      expect(service.minSongDurationSec, 75);

      // Clamp test
      service.setMinSongDurationSec(2);
      expect(service.minSongDurationSec, 5);

      service.setMinSongDurationSec(500);
      expect(service.minSongDurationSec, 300);

      service.setMinSongDurationSec(60);
      expect(service.minSongDurationSec, 60);
    });

    test('Exclusion and custom directories manage sets properly', () {
      final service = SettingsService.instance;

      service.toggleSongExcluded('test_track_1');
      expect(service.excludedSongIds.contains('test_track_1'), isTrue);

      service.removeExcludedSong('test_track_1');
      expect(service.excludedSongIds.contains('test_track_1'), isFalse);

      service.addCustomDirectory('/test/custom/music');
      expect(service.allowedDirectories.contains('/test/custom/music'), isTrue);
      service.toggleDirectoryAllowed('/test/custom/music', false);
      expect(service.allowedDirectories.contains('/test/custom/music'), isFalse);
    });
  });

  group('SettingsAndAudioStage Widget Tests', () {
    testWidgets('Renders all 6 section headers and eliminates useless pill badges', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SettingsAndAudioStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      // 6 Section headers must be present
      expect(find.text('GENERAL'), findsOneWidget);
      expect(find.text('AUDIO'), findsOneWidget);
      expect(find.text('LYRICS'), findsOneWidget);
      expect(find.text('SKIN'), findsOneWidget);
      expect(find.text('FILES'), findsOneWidget);
      expect(find.text('DEVELOPER OPTIONS'), findsOneWidget);

      // Header must display 'Settings'
      expect(find.text('Settings'), findsOneWidget);

      // Strict requirement: NO useless pills! The header version pill badge 'v0.1.0' was removed
      // (The version is now cleanly displayed inside the Rust Player Version tile under Audio)
      expect(find.text('Rust Player Version'), findsOneWidget);
      expect(find.textContaining('v${AttuneEngineBridge.instance.version}'), findsOneWidget);
    });

    testWidgets('Opens Audio Focus Hook Level floating dialog with all 5 levels', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SettingsAndAudioStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      final tile = find.text('Audio Focus Hook Level');
      expect(tile, findsOneWidget);
      await tester.tap(tile);
      await tester.pump(const Duration(milliseconds: 100));

      // Dialog title and options
      expect(find.text('Audio Focus Hook Level'), findsWidgets);
      expect(find.text('Standard (Default)'), findsWidgets);
      expect(find.text('Sensitive'), findsOneWidget);
      expect(find.text('Ultra Sensitive'), findsOneWidget);
      expect(find.text('Ah f#ck alerts'), findsOneWidget);
      expect(find.text('Ah F#ck everything'), findsOneWidget);

      // Close dialog
      await tester.tap(find.text('Close'));
      await tester.pump(const Duration(milliseconds: 100));
      expect(find.text('Ah F#ck everything'), findsNothing);
    });

    testWidgets('Opens View Equalizer floating dialog and tests Reset to Flat', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SettingsAndAudioStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      final tile = find.text('View Equalizer');
      expect(tile, findsOneWidget);
      await tester.longPress(tile);
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Equalizer DSP'), findsOneWidget);
      expect(find.text('Low / Bass (60 Hz)'), findsOneWidget);
      expect(find.text('Mid / Vocals (1,000 Hz)'), findsOneWidget);
      expect(find.text('High / Treble (10,000 Hz)'), findsOneWidget);
      expect(find.text('Reset to Flat (0 dB)'), findsOneWidget);

      await tester.tap(find.text('Done'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      expect(find.text('Equalizer DSP'), findsNothing);
    });

    testWidgets('Opens Hardware Assessment Checks floating dialog with real dynamic metrics', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SettingsAndAudioStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      final tile = find.text('Hardware Assessment Checks');
      expect(tile, findsOneWidget);
      await tester.tap(tile);
      await tester.pump(const Duration(milliseconds: 100));

      // Dialog must display real non-hardcoded data
      expect(find.text('CPU Logical Cores'), findsOneWidget);
      expect(find.text('${Platform.numberOfProcessors} Cores'), findsOneWidget);
      expect(find.text('Operating System'), findsOneWidget);
      expect(find.text('Rust Player Engine'), findsOneWidget);
      expect(find.text('Forced Alignment Circuit Breaker'), findsOneWidget);
      expect(find.text('Output Audio DAC Pipeline'), findsOneWidget);

      await tester.tap(find.text('Close'));
      await tester.pump(const Duration(milliseconds: 100));
      expect(find.text('Output Audio DAC Pipeline'), findsNothing);
    });

    testWidgets('General section disables wakelock and notifications on desktop targets with clear subtitles', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SettingsAndAudioStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Allow Wakelock Held when Playing'), findsOneWidget);
      expect(find.text('Allow Notifications'), findsOneWidget);

      if (Platform.isLinux || Platform.isMacOS || Platform.isWindows) {
        expect(
          find.text('Disabled on desktop build (managed by desktop OS power manager)'),
          findsOneWidget,
        );
        expect(
          find.text('Disabled on desktop build (native desktop notifications in progress)'),
          findsOneWidget,
        );

        // Switches must be disabled (onChanged == null)
        final switches = tester.widgetList<Switch>(find.byType(Switch)).toList();
        // The first two switches in General must have onChanged == null
        expect(switches[0].onChanged, isNull);
        expect(switches[1].onChanged, isNull);
      }
    });
  });
}

