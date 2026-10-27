import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/playback_controller.dart';

void main() {
  test('Smart Shuffle does not repeatedly pick index 0 and respects recency variety', () async {
    final controller = PlaybackController.instance;
    await controller.scanLocalAudioDirectories();
    expect(controller.queue.isNotEmpty, isTrue);

    // Switch to Smart Shuffle
    while (controller.shuffleMode != ShuffleMode.smart) {
      controller.cycleShuffleMode();
    }
    expect(controller.shuffleMode, ShuffleMode.smart);

    final Set<int> pickedIndices = {};
    int consecutiveRabbitHoleCount = 0;

    // Simulate 20 consecutive transitions / skips in Smart Shuffle
    for (int step = 0; step < 20; step++) {
      final upNext = controller.upNextQueueIndex;
      pickedIndices.add(upNext);

      if (upNext == 0) {
        consecutiveRabbitHoleCount++;
      } else {
        consecutiveRabbitHoleCount = 0;
      }

      // Assert that the engine NEVER gets stuck picking index 0 repeatedly in a row
      expect(consecutiveRabbitHoleCount <= 1, isTrue,
          reason: 'Engine should not pick index 0 consecutively');

      controller.skipNext();
    }

    // Verify that across 20 smart transitions, multiple diverse tracks were picked
    expect(pickedIndices.length, greaterThanOrEqualTo(5),
        reason: 'Smart shuffle must provide variety across the library without collapsing to a single track');
  });
}
