import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/track_item.dart';
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

  test('Smart Shuffle fallback recipe anchors transition to outro and never triggers mid-track', () {
    final controller = PlaybackController.instance;
    final trackA = TrackItem(
      id: 'track_a',
      title: 'Track A',
      artist: 'Artist A',
      album: 'Album A',
      duration: const Duration(seconds: 240), // 4-minute song
      filePath: '/music/track_a.flac',
      bpm: 128.0,
      musicalKey: 'C Major',
    );
    final trackB = TrackItem(
      id: 'track_b',
      title: 'Track B',
      artist: 'Artist B',
      album: 'Album B',
      duration: const Duration(seconds: 210),
      filePath: '/music/track_b.flac',
      bpm: 128.0,
      musicalKey: 'G Major',
    );

    final recipe = controller.buildFallbackRecipe(trackA, trackB);
    final timing = recipe['timing'] as Map<String, dynamic>;
    final startMs = timing['track_a_transition_start_ms'] as int;
    final endMs = timing['track_a_exit_end_ms'] as int;

    // Must be anchored strictly to the end of the song (240,000 - 1,400 = 238,600 ms), NOT at 0 or mid-track
    expect(startMs, equals(240000 - 1400));
    expect(endMs, equals(240000));
    expect(startMs, greaterThan(195000)); // Outro cue zone (> 240s - 45s)
  });
}
