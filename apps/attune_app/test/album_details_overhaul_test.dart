import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/models/for_you_mix.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/stages/albums_stage.dart';
import 'package:attune_app/stages/autogen_mix_stage.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('AlbumsStage & Detail Overhaul Tests', () {
    testWidgets('Renders AlbumsStage with collections and On-Repeat Flow banner', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: AlbumsStage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('ALBUMS & COLLECTIONS'), findsOneWidget);
    });

    testWidgets('Enters album detail view with top hero banner and M3 split pill button', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      // Seed sample tracks into library via playback controller
      final controller = PlaybackController.instance;
      const testAlbum = 'Love Words V';
      const sampleTrack = TrackItem(
        id: 'track_sample_test',
        title: 'Ai Kotoba V',
        artist: 'DECO*27',
        album: testAlbum,
        filePath: '/tmp/test.flac',
        duration: Duration(minutes: 3, seconds: 12),
        bpm: 138.0,
        musicalKey: 'F Major',
        genre: 'J-Pop',
      );
      controller.setTracksForTesting([sampleTrack]);

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: AlbumsStage(initialSelectedAlbum: testAlbum),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Verify top hero title overlay
      expect(find.text(testAlbum), findsOneWidget);

      // Verify By Artist
      expect(find.textContaining('By DECO*27'), findsOneWidget);

      // Verify Google M3 Split Pill Button [ Play Album ]
      expect(find.text('Play Album'), findsOneWidget);
      expect(find.byIcon(Icons.arrow_drop_down_rounded), findsOneWidget);

      // Verify Album Liner Notes & Lore card
      expect(find.text('ALBUM LINER NOTES & LORE'), findsOneWidget);
      expect(find.byIcon(Icons.edit_note_rounded), findsOneWidget);

      // Verify Tracklist section & Add Song button
      expect(find.textContaining('TRACKLIST'), findsOneWidget);
      expect(find.text('Add Song'), findsOneWidget);
    });

    test('PlaybackController supports updating track metadata and album assignment', () {
      final controller = PlaybackController.instance;
      const sampleId = 'sample_edit_test';

      // Add to album
      controller.addTrackToAlbum(trackId: sampleId, albumName: 'New Album');

      // Update values
      controller.updateTrackMetadata(
        trackId: sampleId,
        title: 'Updated Title',
        artist: 'Updated Artist',
        bpm: 145.0,
        musicalKey: 'G Major',
        genre: 'Electronic',
      );

      // Remove from album
      controller.removeTrackFromAlbum(trackId: sampleId);
    });

    test('PlaybackController computes Repeat Hook & On-Repeat tracks gracefully', () {
      final controller = PlaybackController.instance;
      final repeatTracks = controller.getRepeatHookTracks(limit: 5);
      expect(repeatTracks, isA<List<TrackItem>>());
    });

    test('PlaybackController addToQueue and addTracksToQueue appends properly', () {
      final controller = PlaybackController.instance;
      const track1 = TrackItem(
        id: 'queue_track_1',
        title: 'Queue 1',
        artist: 'Artist 1',
        album: 'Album 1',
        filePath: '/tmp/q1.mp3',
        duration: Duration(minutes: 2),
        bpm: 120.0,
        musicalKey: '8B',
      );
      const track2 = TrackItem(
        id: 'queue_track_2',
        title: 'Queue 2',
        artist: 'Artist 2',
        album: 'Album 2',
        filePath: '/tmp/q2.mp3',
        duration: Duration(minutes: 3),
        bpm: 124.0,
        musicalKey: '9B',
      );

      controller.addToQueue(track1);
      expect(controller.queue.any((t) => t.id == 'queue_track_1'), isTrue);

      controller.addTracksToQueue([track2]);
      expect(controller.queue.any((t) => t.id == 'queue_track_2'), isTrue);
    });

    testWidgets('AutogenMixStage renders hero vector cover, rationale, and tracklist', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      const sampleTrack = TrackItem(
        id: 'autogen_mix_sample_track',
        title: 'Rabbit Hole',
        artist: 'DECO*27',
        album: 'Love Words',
        filePath: '/tmp/rabbit.flac',
        duration: Duration(minutes: 3, seconds: 5),
        bpm: 140.0,
        musicalKey: 'G Minor',
        genre: 'Vocaloid',
      );

      const testMix = ForYouMix(
        id: 'test_mix_autogen',
        title: 'Vocaloid Drive Mix',
        subtitle: 'High energy synthpop & rock',
        vibeCategory: 'High Energy',
        tracks: [sampleTrack],
        artStyle: ForYouArtStyle.geometricPulse,
        badge: 'CURATED',
        primaryColor: Color(0xFF00E5FF),
        secondaryColor: Color(0xFF7C4DFF),
      );

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: AutogenMixStage(mix: testMix),
          ),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));

      expect(find.text('Vocaloid Drive Mix'), findsNWidgets(2));
      expect(find.text('High energy synthpop & rock'), findsOneWidget);
      expect(find.text('Play Mix'), findsOneWidget);
      expect(find.text('WHY THIS PLAYLIST WAS CHOSEN'), findsOneWidget);
      expect(find.textContaining('MIX TRACKLIST'), findsOneWidget);
      expect(find.text('Rabbit Hole'), findsOneWidget);
    });
  });
}
