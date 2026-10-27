import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/equalizer_service.dart';
import 'package:attune_app/stages/equalizer_stage.dart';
import 'package:attune_app/stages/settings_stage.dart';

void main() {
  group('EqualizerService Unit Tests', () {
    final eq = EqualizerService.instance;

    setUp(() {
      eq.resetToFlat();
    });

    test('Initializes with valid defaults', () {
      expect(eq.isInitialized, isTrue);
      expect(eq.enabled, isTrue);
      expect(eq.preampDb, equals(-3.0));
      expect(eq.activePreset, equals('Flat'));
      expect(eq.bandGainsDb.length, equals(10));
      for (final gain in eq.bandGainsDb) {
        expect(gain, equals(0.0));
      }
      expect(eq.bassEnabled, isFalse);
      expect(eq.clarityEnabled, isFalse);
      expect(eq.stereoWidth, equals(1.0));
    });

    test('Preamp adjustment clamps properly', () {
      eq.setPreampDb(-6.0);
      expect(eq.preampDb, equals(-6.0));
      eq.setPreampDb(10.0);
      expect(eq.preampDb, equals(6.0));
      eq.setPreampDb(-20.0);
      expect(eq.preampDb, equals(-12.0));
    });

    test('Band gain adjustment marks preset as Custom and updates gain', () {
      eq.setBandGain(0, 4.5);
      expect(eq.bandGainsDb[0], equals(4.5));
      expect(eq.activePreset, equals('Custom'));
    });

    test('Applying all defined presets updates band gains correctly', () {
      for (final entry in kEqualizerPresets.entries) {
        eq.applyPreset(entry.key);
        expect(eq.activePreset, equals(entry.key));
        expect(eq.bandGainsDb, equals(entry.value));
      }
    });

    test('ViPER Bass and Clarity adjustments update state', () {
      eq.setBass(enabled: true, gainDb: 6.5, freqHz: 90.0);
      expect(eq.bassEnabled, isTrue);
      expect(eq.bassGainDb, equals(6.5));
      expect(eq.bassFreqHz, equals(90.0));

      eq.setClarity(enabled: true, wet: 0.25);
      expect(eq.clarityEnabled, isTrue);
      expect(eq.clarityWet, equals(0.25));

      eq.setStereoWidth(1.6);
      expect(eq.stereoWidth, equals(1.6));
    });
  });

  group('EqualizerStage Widget Tests (Zero-Hardcoding)', () {
    final eq = EqualizerService.instance;

    setUp(() {
      eq.resetToFlat();
    });

    testWidgets('Renders EqualizerStage with dynamic presets and frequency sliders', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: EqualizerStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Equalizer & Master FX'), findsOneWidget);
      expect(find.text('Studio-Grade 6-Stage Mastering Rack'), findsOneWidget);
      expect(find.text('Pre-Amp Headroom Pad'), findsOneWidget);
      expect(find.text('10-Band Graphic Equalizer'), findsOneWidget);
      expect(find.text('ViPER Bass (Pure Bass+)'), findsOneWidget);
      expect(find.text('ViPER Clarity & Vocal Air'), findsOneWidget);
      expect(find.text('Stereo Field Expander'), findsOneWidget);

      // Verify all dynamic presets are rendered
      for (final preset in kEqualizerPresets.keys) {
        expect(find.text(preset), findsWidgets);
      }

      // Verify all dynamic frequency bands are rendered
      for (final freq in kEqualizerFrequencies) {
        final label = freq >= 1000
            ? (freq / 1000 == (freq / 1000).roundToDouble()
                ? '${(freq / 1000).toInt()}k'
                : '${(freq / 1000).toStringAsFixed(1)}k')
            : '${freq.toInt()}';
        expect(find.text(label), findsWidgets);
      }
    });

    testWidgets('Tapping dynamic preset applies preset gains', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: EqualizerStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      // Find first non-flat preset
      final targetPreset = kEqualizerPresets.keys.firstWhere((k) => k != 'Flat');
      final presetFinder = find.widgetWithText(InkWell, targetPreset);
      expect(presetFinder, findsWidgets);

      await tester.tap(presetFinder.first);
      await tester.pump(const Duration(milliseconds: 100));

      expect(eq.activePreset, equals(targetPreset));
      expect(eq.bandGainsDb, equals(kEqualizerPresets[targetPreset]));
    });

    testWidgets('Reset button restores Flat preset and default values', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      eq.applyPreset('Rock');
      eq.setPreampDb(2.0);
      eq.setBass(enabled: true);

      await tester.pumpWidget(
        const MaterialApp(
          home: EqualizerStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      final resetFinder = find.widgetWithText(TextButton, 'Reset');
      expect(resetFinder, findsOneWidget);
      await tester.tap(resetFinder);
      await tester.pump(const Duration(milliseconds: 100));

      expect(eq.activePreset, equals('Flat'));
      expect(eq.preampDb, equals(-3.0));
      expect(eq.bassEnabled, isFalse);
    });

    testWidgets('SettingsAndAudioStage navigates to EqualizerStage when tapping View Equalizer', (WidgetTester tester) async {
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

      final viewEqFinder = find.text('View Equalizer');
      expect(viewEqFinder, findsOneWidget);

      await tester.ensureVisible(viewEqFinder);
      await tester.pump(const Duration(milliseconds: 100));

      await tester.tap(viewEqFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500)); // allow navigation transition

      expect(find.byType(EqualizerStage), findsOneWidget);
      expect(find.text('Equalizer & Master FX'), findsOneWidget);
    });
  });
}
