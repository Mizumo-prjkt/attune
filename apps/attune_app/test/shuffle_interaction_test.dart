import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/stages/playlist_queue_stage.dart';
import 'package:attune_app/services/playback_controller.dart';

void main() {
  testWidgets('Tapping shuffle chip in PlaylistQueueStage cycles modes smoothly', (WidgetTester tester) async {
    final controller = PlaybackController.instance;

    await tester.pumpWidget(
      MaterialApp(
        theme: ThemeData.dark(),
        home: const Scaffold(
          body: SizedBox(
            width: 800,
            height: 600,
            child: PlaylistQueueStage(),
          ),
        ),
      ),
    );

    final shuffleFinder = find.text('Shuffle Off');
    expect(shuffleFinder, findsOneWidget);

    // Tap 1: Off -> Standard
    await tester.tap(shuffleFinder);
    await tester.pump();
    expect(controller.shuffleMode, ShuffleMode.standard);

    final basicShuffleFinder = find.text('Basic Shuffle');
    expect(basicShuffleFinder, findsOneWidget);

    // Tap 2: Standard -> Smart
    await tester.tap(basicShuffleFinder);
    await tester.pump();
    expect(controller.shuffleMode, ShuffleMode.smart);

    final smartShuffleFinder = find.text('Smart Shuffle');
    expect(smartShuffleFinder, findsOneWidget);

    // Tap 3: Smart -> Off
    await tester.tap(smartShuffleFinder);
    await tester.pump();
    expect(controller.shuffleMode, ShuffleMode.off);
    expect(find.text('Shuffle Off'), findsOneWidget);
  });
}
