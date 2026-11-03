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

    testWidgets('Favorites filter tab and heart toggle in PlaylistQueueStage', (WidgetTester tester) async {
      final controller = PlaybackController.instance;
      controller.clearQueue();
      controller.addTracksToQueue(sampleTracks);
      controller.playTrack(sampleTracks[0]);

      // Ensure favorites starts empty
      controller.setFavorite('song_1', false);
      controller.setFavorite('song_2', false);

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: PlaylistQueueStage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Tap Favorites filter chip
      final favoritesChip = find.text('Favorites');
      expect(favoritesChip, findsOneWidget);
      await tester.tap(favoritesChip);
      await tester.pumpAndSettle();

      // Empty state shown
      expect(find.text('No Favorite Tracks'), findsOneWidget);

      // Now favorite song_1
      controller.setFavorite('song_1', true);
      await tester.pumpAndSettle();

      expect(find.text('1 FAVORITE TRACKS'), findsOneWidget);
      expect(find.text('Super Trouper'), findsWidgets);

      // Switch back to Up Next
      await tester.tap(find.text('Up Next'));
      await tester.pumpAndSettle();
      expect(find.text('Super Trouper'), findsWidgets);
    });

    testWidgets('AlbumEditorStage multi-artists, sorting, and online catalog metadata', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 3000);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      AlbumCustomMetadata? savedMetadata;

      await tester.pumpWidget(
        MaterialApp(
          home: AlbumEditorStage(
            initialAlbumName: 'Super Trouper',
            initialTracks: sampleTracks,
            initialMetadata: const AlbumCustomMetadata(
              year: '1980',
              genre: 'Pop',
              recordLabel: 'Polar Music',
              catalogNumber: 'POLS-322',
              albumType: 'Studio Album',
              releaseDate: '1980-11-03',
              language: 'English',
              musicBrainzId: 'mb-uuid-1234',
            ),
            onSaved: (oldAlbum, newAlbum, meta, finalTracks) {
              savedMetadata = meta;
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Verify Online Query & Catalog section
      expect(find.text('ONLINE QUERY & CATALOG IDENTIFIERS'), findsOneWidget);
      expect(find.text('Fetch Online Metadata (MusicBrainz / ReccoBeats)'), findsOneWidget);
      expect(find.text('Polar Music'), findsOneWidget);
      expect(find.text('POLS-322'), findsOneWidget);

      // Add a contributing artist
      final artistInput = find.widgetWithText(TextField, 'Add contributing artist name (comma-separated)...');
      expect(artistInput, findsOneWidget);
      await tester.enterText(artistInput, 'Benny Andersson');
      await tester.tap(find.text('Add'));
      await tester.pumpAndSettle();

      expect(find.text('Benny Andersson'), findsOneWidget);

      // Verify sorting button is present
      expect(find.text('Track Index'), findsOneWidget);

      // Save
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();

      expect(savedMetadata, isNotNull);
      expect(savedMetadata!.additionalArtists, contains('Benny Andersson'));
      expect(savedMetadata!.recordLabel, equals('Polar Music'));
      expect(savedMetadata!.catalogNumber, equals('POLS-322'));
    });

    testWidgets('AlbumEditorStage splits comma-delimited artists on Primary Artist and Contributors', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 3000);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      AlbumCustomMetadata? savedMetadata;

      final multiArtistTracks = [
        const TrackItem(
          id: 'song_collab',
          title: '(Not) A Devil',
          artist: 'DECO*27, PinocchioP',
          album: '(Not) A Devil',
          filePath: '/music/not_a_devil.flac',
          duration: Duration(minutes: 2, seconds: 43),
          bpm: 149.2,
          musicalKey: 'F# Minor',
          genre: 'Vocaloid',
          isLossless: true,
        ),
      ];

      await tester.pumpWidget(
        MaterialApp(
          home: AlbumEditorStage(
            initialAlbumName: '(Not) A Devil',
            initialTracks: multiArtistTracks,
            onSaved: (oldAlbum, newAlbum, meta, finalTracks) {
              savedMetadata = meta;
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Primary Artist starts with 'DECO*27, PinocchioP'
      // Should show the 'Split Artists (,)' badge
      final splitBadge = find.text('Split Artists (,)');
      expect(splitBadge, findsOneWidget);

      // Tap split badge
      await tester.tap(splitBadge);
      await tester.pumpAndSettle();

      // PinocchioP is now in contributing artists list!
      expect(find.text('PinocchioP'), findsOneWidget);

      // Add comma-delimited artists in new artist field
      final artistInput = find.widgetWithText(TextField, 'Add contributing artist name (comma-separated)...');
      await tester.enterText(artistInput, 'GUMI, Megurine Luka');
      await tester.tap(find.text('Add'));
      await tester.pumpAndSettle();

      expect(find.text('GUMI'), findsOneWidget);
      expect(find.text('Megurine Luka'), findsOneWidget);

      // Save
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();

      expect(savedMetadata, isNotNull);
      expect(savedMetadata!.additionalArtists, contains('PinocchioP'));
      expect(savedMetadata!.additionalArtists, contains('GUMI'));
      expect(savedMetadata!.additionalArtists, contains('Megurine Luka'));
    });
  });
}
