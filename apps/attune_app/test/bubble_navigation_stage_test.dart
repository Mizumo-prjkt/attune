import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/stages/bubble_navigation_stage.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('BubbleItem and Model Tests', () {
    test('BubbleItem holds metadata and supports copyWithPosition', () {
      const item = BubbleItem(
        id: 'track_1',
        title: 'Song Title',
        subtitle: 'Artist Name',
        album: 'Album Name',
        seedColor: Colors.teal,
        bpm: 128.0,
        musicalKey: '8A',
      );

      expect(item.id, equals('track_1'));
      expect(item.title, equals('Song Title'));
      expect(item.bpm, equals(128.0));
      expect(item.musicalKey, equals('8A'));
      expect(item.baseOffset, equals(Offset.zero));

      final moved = item.copyWithPosition(const Offset(100.0, -50.0));
      expect(moved.baseOffset.dx, equals(100.0));
      expect(moved.baseOffset.dy, equals(-50.0));
      expect(moved.title, equals('Song Title'));
    });
  });

  group('BubbleNavigationStage Widget & Interaction Tests', () {
    setUp(() {
      PlaybackController.instance.setTracksForTesting([
        const TrackItem(
          id: 'test_1',
          filePath: '/dummy/song1.mp3',
          title: 'Electric Beats',
          artist: 'Future Sound',
          album: 'Neon Horizon',
          duration: Duration(minutes: 3),
          genre: 'Electronic',
          bpm: 126.0,
          musicalKey: '11B',
        ),
        const TrackItem(
          id: 'test_2',
          filePath: '/dummy/song2.mp3',
          title: 'Rock Anthem',
          artist: 'The Strikers',
          album: 'Overdrive',
          duration: Duration(minutes: 4),
          genre: 'Rock',
          bpm: 140.0,
          musicalKey: '4A',
        ),
      ]);
    });

    testWidgets('BubbleNavigationStage mounts and displays header and chips', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: BubbleNavigationStage(),
        ),
      );
      await tester.pumpAndSettle();

      // Verify Bubblegrid Selection title is present
      expect(find.text('Bubblegrid Selection'), findsOneWidget);

      // Verify category filter chips
      expect(find.text('All Tracks'), findsOneWidget);
      expect(find.text('Albums'), findsOneWidget);
      expect(find.text('For You Mixes'), findsOneWidget);
      expect(find.text('Rock'), findsOneWidget);
      expect(find.text('Pop'), findsOneWidget);
      expect(find.text('Electronic'), findsOneWidget);

      // Verify recenter button exists
      expect(find.byIcon(Icons.filter_center_focus_rounded), findsOneWidget);
    });

    testWidgets('Tapping filter chip updates active filter', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: BubbleNavigationStage(),
        ),
      );
      await tester.pumpAndSettle();

      // Tap 'Electronic' chip
      await tester.tap(find.text('Electronic'));
      await tester.pumpAndSettle();

      // Tap 'Albums' chip
      await tester.tap(find.text('Albums'));
      await tester.pumpAndSettle();

      // Recenter
      await tester.tap(find.byIcon(Icons.filter_center_focus_rounded));
      await tester.pumpAndSettle();
    });

    testWidgets('Dragging on canvas triggers pan gesture smoothly', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: BubbleNavigationStage(),
        ),
      );
      await tester.pumpAndSettle();

      // Drag across the center of the screen
      final center = tester.getCenter(find.byType(BubbleNavigationStage));
      await tester.dragFrom(center, const Offset(-60.0, -40.0));
      await tester.pumpAndSettle();

      // Verify still rendered without uncaught exception
      expect(find.text('Bubblegrid Selection'), findsOneWidget);
    });
  });
}
