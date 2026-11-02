import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/album_custom_metadata.dart';
import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/stages/album_editor_stage.dart';
import 'package:attune_app/stages/playlist_queue_stage.dart';
import 'package:attune_app/widgets/transition_connector_pill.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final sampleTracks = [
    const TrackItem(
      id: 'song_1',
      title: 'Super Trouper',
      artist: 'ABBA',
      album: 'Super Trouper',
      filePath: '/music/super_trouper.flac',
      duration: Duration(minutes: 4, seconds: 13),
      bpm: 118.0,
      musicalKey: 'C Major',
      genre: 'Pop',
      isLossless: true,
    ),
    const TrackItem(
      id: 'song_2',
      title: 'The Winner Takes It All',
      artist: 'ABBA',
      album: 'Super Trouper',
      filePath: '/music/winner_takes_it_all.flac',
      duration: Duration(minutes: 4, seconds: 56),
      bpm: 126.0,
      musicalKey: 'Gb Major',
      genre: 'Pop',
      isLossless: true,
    ),
  ];

  final extraLibraryTrack = const TrackItem(
    id: 'song_3',
    title: 'Dancing Queen',
    artist: 'ABBA',
    album: 'Arrival',
    filePath: '/music/dancing_queen.flac',
    duration: Duration(minutes: 3, seconds: 51),
    bpm: 100.0,
    musicalKey: 'A Major',
    genre: 'Disco',
    isLossless: true,
  );

  group('AlbumEditorStage Tests', () {
    testWidgets('Renders all metadata sections and allows editing values', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      final controller = PlaybackController.instance;
      controller.setTracksForTesting([...sampleTracks, extraLibraryTrack]);

      bool savedCallbackTriggered = false;
      String? savedNewAlbumName;

      await tester.pumpWidget(
        MaterialApp(
          home: AlbumEditorStage(
            initialAlbumName: 'Super Trouper',
            initialTracks: sampleTracks,
            initialMetadata: const AlbumCustomMetadata(
              year: '1980',
              genre: 'Pop',
              description: 'Classic album by ABBA.',
            ),
            onSaved: (oldAlbum, newAlbum, meta, finalTracks) {
              savedCallbackTriggered = true;
              savedNewAlbumName = newAlbum;
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Verify sections are visible
      expect(find.text('EDIT ALBUM & GROUP'), findsOneWidget);
      expect(find.text('ALBUM METADATA'), findsOneWidget);
      expect(find.text('ALBUM SPLASH ART'), findsOneWidget);
      expect(find.text('ALBUM LORE & DESCRIPTION'), findsOneWidget);
      expect(find.text('TRACKS IN GROUP (2)'), findsOneWidget);

      // Verify track rows
      expect(find.text('Super Trouper'), findsWidgets);
      expect(find.text('The Winner Takes It All'), findsOneWidget);

      // Tap Auto-Synthesize button
      final autoSynthButton = find.text('Auto-Synthesize');
      expect(autoSynthButton, findsOneWidget);
      await tester.tap(autoSynthButton);
      await tester.pumpAndSettle();

      // Remove a track from the group
      final removeButtons = find.byIcon(Icons.remove_circle_outline_rounded);
      expect(removeButtons, findsNWidgets(2));
      await tester.tap(removeButtons.first);
      await tester.pumpAndSettle();

      expect(find.text('TRACKS IN GROUP (1)'), findsOneWidget);

      // Tap Save
      final saveBtn = find.text('Save');
      expect(saveBtn, findsOneWidget);
      await tester.tap(saveBtn);
      await tester.pumpAndSettle();

      expect(savedCallbackTriggered, isTrue);
      expect(savedNewAlbumName, equals('Super Trouper'));
    });
  });

  group('PlaylistQueueStage Connector Pill & Verdict Popup Tests', () {
    testWidgets('Connector pill is hidden when shuffle is off, and shown when smart shuffle is active', (WidgetTester tester) async {
      final controller = PlaybackController.instance;
      controller.clearQueue();
      controller.addTracksToQueue(sampleTracks);
      controller.playTrack(sampleTracks[0]);

      // 1. Shuffle Off: Pill MUST be hidden
      controller.setShuffleMode(ShuffleMode.off);

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: PlaylistQueueStage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(TransitionConnectorPill), findsNothing);

      // 2. Standard Shuffle: Pill MUST still be hidden
      controller.setShuffleMode(ShuffleMode.standard);
      await tester.pumpAndSettle();
      expect(find.byType(TransitionConnectorPill), findsNothing);

      // 3. Smart Shuffle: Pill MUST appear
      controller.setShuffleMode(ShuffleMode.smart);
      await tester.pumpAndSettle();
      expect(find.byType(TransitionConnectorPill), findsOneWidget);

      // 4. Tapping TransitionConnectorPill opens Transition Selection Rationale dialog
      await tester.tap(find.byType(TransitionConnectorPill));
      await tester.pumpAndSettle();

      expect(find.text('Transition Selection Rationale'), findsOneWidget);
      expect(find.text('✦ Smart Shuffle Verdict'), findsOneWidget);
      expect(find.text('∿ Transition DSP Recipe'), findsOneWidget);
    });
  });
}
