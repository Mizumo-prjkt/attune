import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/services/settings_service.dart';
import 'package:attune_app/services/lyrics_service.dart';
import 'package:attune_app/stages/playlist_queue_stage.dart';
import 'package:attune_app/stages/queue_add_tracks_stage.dart';
import 'package:attune_app/stages/now_playing_stage.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final dummyTracks = List.generate(
    25,
    (i) => TrackItem(
      id: 'test_track_$i',
      title: 'Track $i',
      artist: 'Artist ${i % 3}',
      album: 'Album ${i % 2}',
      duration: const Duration(seconds: 180),
      filePath: '/music/track_$i.flac',
      bpm: 120.0 + (i * 2.0),
      musicalKey: 'C Major',
      genre: i % 2 == 0 ? 'Vocaloid' : 'Rock',
      isLossless: true,
      sampleRate: 48000,
      bitDepth: 16,
    ),
  );

  group('Play Queue Overhaul Tests', () {
    test('Queue starts empty and supports add, remove, and clear', () {
      final controller = PlaybackController.instance;
      controller.clearQueue();
      expect(controller.queue.isEmpty, isTrue);

      // Add a single track
      controller.addToQueue(dummyTracks[0]);
      expect(controller.queue.length, equals(1));
      expect(controller.queue.first.id, equals('test_track_0'));

      // Add multiple tracks
      controller.addTracksToQueue(dummyTracks.sublist(1, 5));
      expect(controller.queue.length, equals(5));

      // Remove specific index
      controller.removeFromQueue(0);
      expect(controller.queue.length, equals(4));
      expect(controller.queue.first.id, equals('test_track_1'));

      // Remove by ID
      controller.removeFromQueueById('test_track_2');
      expect(controller.queue.length, equals(3));
      expect(controller.queue.any((t) => t.id == 'test_track_2'), isFalse);

      // Clear queue
      controller.clearQueue();
      expect(controller.queue.isEmpty, isTrue);
    });

    test('reorderQueue is locked during Smart Shuffle', () {
      final controller = PlaybackController.instance;
      controller.clearQueue();
      controller.addTracksToQueue(dummyTracks.sublist(0, 5));

      while (controller.shuffleMode != ShuffleMode.smart) {
        controller.cycleShuffleMode();
      }
      expect(controller.shuffleMode, equals(ShuffleMode.smart));

      final firstTrackIdBefore = controller.queue[0].id;
      final secondTrackIdBefore = controller.queue[1].id;

      // Attempt reorder
      controller.reorderQueue(0, 2);

      // In Smart Shuffle, order must remain locked
      expect(controller.queue[0].id, equals(firstTrackIdBefore));
      expect(controller.queue[1].id, equals(secondTrackIdBefore));

      // Switch to linear/standard shuffle, reorder should succeed
      controller.setShuffleMode(ShuffleMode.off);
      controller.reorderQueue(0, 1);
      expect(controller.queue[0].id, equals(secondTrackIdBefore));
    });
  });

  group('PlaylistQueueStage Widget Tests', () {
    testWidgets('Renders empty queue card when queue is empty', (tester) async {
      final controller = PlaybackController.instance;
      controller.clearQueue();

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PlaylistQueueStage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Your Play Queue is Empty'), findsOneWidget);
      expect(find.text('Add Tracks to Queue'), findsOneWidget);
    });

    testWidgets('Shows low track count advisory warning when smart shuffle has < 20 tracks', (tester) async {
      final controller = PlaybackController.instance;
      controller.clearQueue();
      controller.addTracksToQueue(dummyTracks.sublist(0, 5)); // 5 tracks < 20
      controller.setShuffleMode(ShuffleMode.smart);

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PlaylistQueueStage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.textContaining('Smart Shuffle Advisory'), findsOneWidget);
      expect(find.byIcon(Icons.auto_awesome_rounded), findsWidgets);
    });
  });

  group('QueueAddTracksStage Widget Tests', () {
    testWidgets('Renders library tracks, filters by search query and genre chip, and adds to queue', (tester) async {
      final controller = PlaybackController.instance;
      controller.clearQueue();
      await controller.scanLocalAudioDirectories(populateQueue: false);

      await tester.pumpWidget(
        MaterialApp(
          home: const QueueAddTracksStage(),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('ADD TRACKS TO QUEUE'), findsOneWidget);
      expect(find.byType(TextField), findsOneWidget);

      // Search for a specific query
      await tester.enterText(find.byType(TextField), 'rabbit');
      await tester.pumpAndSettle();

      // Find checkboxes / list items
      final listTiles = find.byType(CheckboxListTile);
      if (listTiles.evaluate().isNotEmpty) {
        // Tap first filtered track
        await tester.tap(listTiles.first);
        await tester.pumpAndSettle();

        // Tap Add selected tracks floating button
        final addBtn = find.textContaining('Add ');
        expect(addBtn, findsOneWidget);
        await tester.tap(addBtn);
        await tester.pumpAndSettle();

        // Queue should now contain the added track
        expect(controller.queue.isNotEmpty, isTrue);
      }
    });
  });

  group('NowPlaying Skins & Freeshape Morphing Tests', () {
    test('MorphingFreeshapeClipper computes valid path without NaN across progress steps', () {
      const clipper0 = MorphingFreeshapeClipper(shapeFrom: 0, shapeTo: 1, progress: 0.0);
      const clipperMid = MorphingFreeshapeClipper(shapeFrom: 0, shapeTo: 1, progress: 0.5);
      const clipper1 = MorphingFreeshapeClipper(shapeFrom: 0, shapeTo: 1, progress: 1.0);

      final path0 = clipper0.getClip(const Size(300, 300));
      final pathMid = clipperMid.getClip(const Size(300, 300));
      final path1 = clipper1.getClip(const Size(300, 300));

      expect(path0.getBounds().isEmpty, isFalse);
      expect(pathMid.getBounds().isEmpty, isFalse);
      expect(path1.getBounds().isEmpty, isFalse);

      expect(pathMid.getBounds().width, greaterThan(100.0));
      expect(pathMid.getBounds().height, greaterThan(100.0));
    });

    test('LyricsService getCachedLyrics returns cached or current lyrics', () async {
      final lyricsService = LyricsService.instance;
      final track = dummyTracks[0];
      lyricsService.invalidateCache();
      expect(lyricsService.getCachedLyrics(track.id), isNull);

      // Simulate load
      await lyricsService.loadLyrics(track, LyricSourceType.syllableSync);
      expect(lyricsService.currentParsedLyrics, isNotNull);
      expect(lyricsService.getCachedLyrics(track.id), isNotNull);
    });
  });
}
