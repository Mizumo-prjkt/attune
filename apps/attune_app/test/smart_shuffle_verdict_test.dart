import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/stages/now_playing_stage.dart';
import 'package:attune_app/widgets/transition_connector_pill.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('SmartShuffleVerdict Service & Scoring Tests', () {
    final controller = PlaybackController.instance;

    setUp(() async {
      await controller.scanLocalAudioDirectories();
    });

    test('getOrComputeSmartShuffleVerdict computes non-null structured verdict with dynamic rationale', () {
      expect(controller.queue.isNotEmpty, isTrue);

      // Switch to smart shuffle mode
      while (controller.shuffleMode != ShuffleMode.smart) {
        controller.cycleShuffleMode();
      }
      expect(controller.shuffleMode, equals(ShuffleMode.smart));

      final verdict = controller.getOrComputeSmartShuffleVerdict();
      expect(verdict.winner.track.id, equals(controller.upNextTrack?.id));
      expect(verdict.winner.matchPercentage, inInclusiveRange(5, 99));

      // Dynamic non-hardcoded factor verdicts
      expect(verdict.winner.harmonicVerdict.isNotEmpty, isTrue);
      expect(verdict.winner.tempoVerdict.isNotEmpty, isTrue);
      expect(verdict.winner.energyVerdict.isNotEmpty, isTrue);
      expect(verdict.winner.rotationVerdict.isNotEmpty, isTrue);
      expect(verdict.winner.primaryReason.isNotEmpty, isTrue);
      expect(verdict.detailedRationale.isNotEmpty, isTrue);

      // Alternatives evaluated from queue
      expect(verdict.alternatives.isNotEmpty, isTrue);
      expect(verdict.alternatives.length, inInclusiveRange(1, 4));

      for (final alt in verdict.alternatives) {
        expect(alt.track.id, isNot(equals(verdict.winner.track.id)));
        expect(alt.matchPercentage, inInclusiveRange(5, 99));
        expect(alt.primaryReason.isNotEmpty, isTrue);
        expect(alt.harmonicVerdict.isNotEmpty, isTrue);
      }
    });

    test('selectAlternativeAsUpNext successfully promotes candidate to Up Next and rotates winner', () {
      while (controller.shuffleMode != ShuffleMode.smart) {
        controller.cycleShuffleMode();
      }
      final initialVerdict = controller.getOrComputeSmartShuffleVerdict();
      expect(initialVerdict.alternatives.isNotEmpty, isTrue);

      final oldWinnerId = initialVerdict.winner.track.id;
      final candidateToPromote = initialVerdict.alternatives.first;

      controller.selectAlternativeAsUpNext(candidateToPromote);

      // Verify upNext is now the promoted candidate
      expect(controller.upNextQueueIndex, equals(candidateToPromote.queueIndex));
      expect(controller.upNextTrack?.id, equals(candidateToPromote.track.id));

      final updatedVerdict = controller.getOrComputeSmartShuffleVerdict();
      expect(updatedVerdict.winner.track.id, equals(candidateToPromote.track.id));
      expect(
        updatedVerdict.alternatives.any((alt) => alt.track.id == oldWinnerId),
        isTrue,
        reason: 'Previous winner should be placed into alternatives list',
      );
    });
  });

  group('NowPlayingStage Smart Shuffle Verdict UI Tests', () {
    final controller = PlaybackController.instance;

    setUp(() async {
      await controller.scanLocalAudioDirectories();
      while (controller.shuffleMode != ShuffleMode.smart) {
        controller.cycleShuffleMode();
      }
    });

    testWidgets('Opens Transition Selection Rationale dialog and displays dual-tabs with verdict & alternatives',
        (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1200, 2000);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: NowPlayingStage(),
          ),
        ),
      );
      await tester.pump(const Duration(milliseconds: 150));

      // Find and tap the Transition Connector Pill
      final pillFinder = find.byType(TransitionConnectorPill);
      expect(pillFinder, findsOneWidget);
      await tester.tap(pillFinder);
      await tester.pumpAndSettle();

      // Verify dialog header and segmented tab switcher
      expect(find.text('Transition Selection Rationale'), findsOneWidget);
      expect(find.text('✦ Smart Shuffle Verdict'), findsOneWidget);
      expect(find.text('∿ Transition DSP Recipe'), findsOneWidget);

      // Verify Smart Shuffle Verdict Tab content (Winner card)
      expect(find.text('WINNING CANDIDATE • UP NEXT'), findsOneWidget);
      final verdict = controller.getOrComputeSmartShuffleVerdict();
      expect(find.text(verdict.winner.track.title), findsWidgets);
      expect(find.text(verdict.winner.track.artist), findsWidgets);
      expect(find.text('${verdict.winner.matchPercentage}% Match'), findsWidgets);

      // Verify Alternatives Section
      expect(find.textContaining('Other Evaluated Candidates'), findsOneWidget);
      if (verdict.alternatives.isNotEmpty) {
        expect(find.text('#2'), findsOneWidget);
        expect(find.text('Promote'), findsWidgets);

        // Tap Promote on the first alternative
        final firstPromoteButton = find.text('Promote').first;
        await tester.ensureVisible(firstPromoteButton);
        await tester.tap(firstPromoteButton);
        await tester.pumpAndSettle();

        // Verify the promoted track is now displayed in the winner card
        final newVerdict = controller.getOrComputeSmartShuffleVerdict();
        expect(find.text(newVerdict.winner.track.title), findsWidgets);
      }

      // Switch to DSP Recipe tab
      await tester.tap(find.text('∿ Transition DSP Recipe'));
      await tester.pumpAndSettle();

      expect(find.text('3-Band EQ Crossover (180 Hz)'), findsOneWidget);
      expect(find.text('Vocal Completion Protection'), findsOneWidget);
      expect(find.text('Phrase & Tempo Grid Alignment'), findsOneWidget);

      // Close the dialog
      await tester.tap(find.text('Got it'));
      await tester.pumpAndSettle();

      expect(find.text('Transition Selection Rationale'), findsNothing);
    });
  });
}
