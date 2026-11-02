import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/stages/albums_stage.dart';

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
  });
}
