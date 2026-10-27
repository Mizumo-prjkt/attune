import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/main.dart';
import 'package:attune_app/theme/color_engine.dart';
import 'package:attune_app/widgets/audio_quality_badge.dart';
import 'package:attune_app/widgets/transition_connector_pill.dart';
import 'package:attune_app/stages/synchronized_lyrics_stage.dart';

void main() {
  group('AudioQualityBadge Tests (Section 3.1 Compliance)', () {
    testWidgets('Renders Very Good Quality Audio for lossless FLAC 24/96', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: AudioQualityBadge(
              format: 'FLAC',
              sampleRate: 96000,
              bitDepth: 24,
            ),
          ),
        ),
      );

      // Must display human-friendly text
      expect(find.text('Very Good Quality Audio'), findsOneWidget);
      expect(find.text('24-bit / 96.0 kHz FLAC'), findsOneWidget);
      expect(find.byIcon(Icons.diamond_rounded), findsOneWidget);

      // Section 3.1 strictly forbids marketing hype / proprietary branding
      expect(find.textContaining('Dolby', findRichText: true), findsNothing);
      expect(find.textContaining('MQA', findRichText: true), findsNothing);
      expect(find.textContaining('Spatial Audio', findRichText: true), findsNothing);
    });

    testWidgets('Renders Good Quality Audio for standard format', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: AudioQualityBadge(
              format: 'OGG',
              sampleRate: 44100,
              bitDepth: 16,
            ),
          ),
        ),
      );

      expect(find.text('Good Quality Audio'), findsOneWidget);
      expect(find.text('16-bit / 44.1 kHz OGG'), findsOneWidget);
      expect(find.byIcon(Icons.music_note_rounded), findsOneWidget);
    });

    testWidgets('Renders Proprietary Audio Codec for proprietary stream', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: AudioQualityBadge(
              format: 'AAC',
              sampleRate: 48000,
              isProprietary: true,
            ),
          ),
        ),
      );

      expect(find.text('Proprietary Audio Codec'), findsOneWidget);
      expect(find.text('16-bit / 48.0 kHz AAC'), findsOneWidget);
      expect(find.byIcon(Icons.lock_outline_rounded), findsOneWidget);
    });
  });

  group('TransitionConnectorPill Tests', () {
    testWidgets('Renders transition pill and displays metrics', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: TransitionConnectorPill(
              archetype: 'Vocal Stem Anchor Morphs',
              durationSeconds: 7.4,
              confidenceScore: 0.94,
            ),
          ),
        ),
      );

      expect(find.text('Vocal Stem Anchor Morphs • 7.4s'), findsOneWidget);
      expect(find.text('94% match'), findsOneWidget);
      expect(find.byIcon(Icons.graphic_eq_rounded), findsOneWidget);
    });
  });

  group('ColorEngine Tests', () {
    test('HCT color engine creates valid dark color scheme from seed', () {
      final scheme = AttuneColorEngine.createSchemeFromSeed(const Color(0xFF6366F1));
      expect(scheme.brightness, Brightness.dark);
      expect(scheme.primary, isNotNull);
      expect(scheme.surface, isNotNull);
    });

    test('lerpSchemes interpolates between two color schemes smoothly', () {
      final schemeA = AttuneColorEngine.createSchemeFromSeed(const Color(0xFFFF0000));
      final schemeB = AttuneColorEngine.createSchemeFromSeed(const Color(0xFF0000FF));

      final at0 = AttuneColorEngine.lerpSchemes(schemeA, schemeB, 0.0);
      final at1 = AttuneColorEngine.lerpSchemes(schemeA, schemeB, 1.0);
      final atHalf = AttuneColorEngine.lerpSchemes(schemeA, schemeB, 0.5);

      expect(at0.primary, schemeA.primary);
      expect(at1.primary, schemeB.primary);
      expect(atHalf.primary, isNotNull);
    });
  });

  group('AttuneApp Shell Smoke Test', () {
    testWidgets('Initializes ResponsiveGlassScaffold', (WidgetTester tester) async {
      await tester.pumpWidget(const AttuneApp());
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.byType(AttuneApp), findsOneWidget);
    });
  });

  group('SynchronizedLyricsStage Tests', () {
    testWidgets('Renders lyric source switcher and empty/real states gracefully', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SynchronizedLyricsStage(),
          ),
        ),
      );
      await tester.pump();

      expect(find.text('Embedded'), findsOneWidget);
      expect(find.text('.LRC File'), findsOneWidget);
      expect(find.text('Ultra-Sync (Soon)'), findsOneWidget);

      // Tap .LRC File tab
      await tester.tap(find.text('.LRC File'));
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('.LRC File'), findsOneWidget);
    });
  });

  group('Collapsible Side Panels Tests', () {
    testWidgets('Right pane and rail collapse and expand correctly', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(const AttuneApp());
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));

      // Right docked pane has collapse button
      final collapseRight = find.byTooltip('Collapse side panel');
      expect(collapseRight, findsOneWidget);

      // Collapse right pane
      await tester.tap(collapseRight);
      await tester.pump(const Duration(milliseconds: 300));

      // Right pane is now collapsed, "Show Sidebar" pill appears
      expect(find.text('Show Sidebar'), findsOneWidget);

      // Restore right pane
      await tester.tap(find.text('Show Sidebar'));
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.byTooltip('Collapse side panel'), findsOneWidget);
    });
  });
}

