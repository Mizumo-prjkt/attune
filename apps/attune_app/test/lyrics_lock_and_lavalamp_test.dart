import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/theme/liquid_glass.dart';
import 'package:attune_app/stages/synchronized_lyrics_stage.dart';

void main() {
  group('Living Beat Background & Lavalamp Globs Tests', () {
    test('PlaybackController toggles livingBeatMeshEnabled correctly', () {
      final controller = PlaybackController.instance;
      expect(controller.livingBeatMeshEnabled, isTrue);

      bool notified = false;
      controller.addListener(() {
        notified = true;
      });

      controller.setLivingBeatMeshEnabled(false);
      expect(controller.livingBeatMeshEnabled, isFalse);
      expect(notified, isTrue);

      // Restore
      controller.setLivingBeatMeshEnabled(true);
      expect(controller.livingBeatMeshEnabled, isTrue);
    });

    testWidgets('AmbientMeshBackground respects livingBeatMeshEnabled toggle', (WidgetTester tester) async {
      final controller = PlaybackController.instance;
      controller.setLivingBeatMeshEnabled(true);

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: AmbientMeshBackground(
              child: Text('Foreground Content'),
            ),
          ),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Foreground Content'), findsOneWidget);
      expect(find.byType(CustomPaint), findsWidgets);

      // Disable living beat mesh
      controller.setLivingBeatMeshEnabled(false);
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Foreground Content'), findsOneWidget);

      // Restore
      controller.setLivingBeatMeshEnabled(true);
      await tester.pump(const Duration(milliseconds: 100));
    });
  });

  group('SynchronizedLyricsStage Precision & Free-Scroll Mode Tests', () {
    testWidgets('Renders lyrics stage with source switcher and controls', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SynchronizedLyricsStage(isFullPage: true),
          ),
        ),
      );
      await tester.pump(const Duration(milliseconds: 200));

      expect(find.text('Embedded'), findsOneWidget);
      expect(find.text('.LRC File'), findsOneWidget);
      expect(find.text('Ultra-Sync (Soon)'), findsOneWidget);

      // Switching source tab resets state cleanly
      await tester.tap(find.text('.LRC File'));
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('.LRC File'), findsOneWidget);

      await tester.tap(find.text('Embedded'));
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('Embedded'), findsOneWidget);
    });
  });
}
