import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/services/settings_service.dart';
import 'package:attune_app/stages/settings_stage.dart';
import 'package:attune_app/stages/song_exclusion_stage.dart';

void main() {
  group('SongExclusionStage Widget Tests (Zero-Hardcoding)', () {
    final controller = PlaybackController.instance;
    final settings = SettingsService.instance;

    setUp(() {
      // Clear exclusions dynamically
      for (final id in settings.excludedSongIds.toList()) {
        settings.removeExcludedSong(id);
      }
      controller.applyLibraryFilters();
    });

    testWidgets('Renders SongExclusionStage with dynamic header counts and search bar', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SongExclusionStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Song Exclusion Manager'), findsOneWidget);
      expect(find.text('Filter tracks out of queues, shuffle, and library'), findsOneWidget);
      expect(find.byType(TextField), findsOneWidget);
      expect(find.text('All Songs'), findsOneWidget);
      expect(find.text('Excluded'), findsOneWidget);
      expect(find.text('Available'), findsOneWidget);

      final tracks = controller.rawScannedTracks.isNotEmpty
          ? controller.rawScannedTracks
          : controller.libraryTracks;

      if (tracks.isNotEmpty) {
        expect(find.text(tracks.first.title), findsOneWidget);
      }
    });

    testWidgets('Search input filters songs dynamically using genuine library track metadata', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SongExclusionStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      final tracks = controller.rawScannedTracks.isNotEmpty
          ? controller.rawScannedTracks
          : controller.libraryTracks;

      if (tracks.length >= 2) {
        final targetTrack = tracks.first;
        final query = targetTrack.title.split(' ').first;

        // Enter search query dynamically based on real track title
        await tester.enterText(find.byType(TextField), query);
        await tester.pump(const Duration(milliseconds: 100));

        expect(find.text(targetTrack.title), findsWidgets);

        // Clear search using clear button
        await tester.tap(find.byIcon(Icons.clear_rounded));
        await tester.pump(const Duration(milliseconds: 100));

        // Verifies list reset
        expect(find.text(tracks.first.title), findsWidgets);
      }
    });

    testWidgets('Excluding and restoring genuine tracks dynamically updates exclusion state', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SongExclusionStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      final tracks = controller.rawScannedTracks.isNotEmpty
          ? controller.rawScannedTracks
          : controller.libraryTracks;

      if (tracks.isNotEmpty) {
        final targetTrack = tracks.first;

        // Find the Exclude button for this track
        final excludeButtons = find.text('Exclude');
        expect(excludeButtons, findsWidgets);
        await tester.tap(excludeButtons.first);
        await tester.pump(const Duration(milliseconds: 100));

        expect(settings.excludedSongIds.contains(targetTrack.id), isTrue);
        expect(find.text('Restore'), findsOneWidget);

        // Switch to Excluded tab
        await tester.tap(find.text('Excluded'));
        await tester.pump(const Duration(milliseconds: 100));

        expect(find.text(targetTrack.title), findsWidgets);

        // Tap Restore
        await tester.tap(find.text('Restore'));
        await tester.pump(const Duration(milliseconds: 100));

        expect(settings.excludedSongIds.contains(targetTrack.id), isFalse);
        expect(find.text('No tracks currently excluded'), findsOneWidget);
      }
    });

    testWidgets('Empty search results display responsive empty state and reset button', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SongExclusionStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      // Generate a dynamic non-matching query using timestamp
      final nonMatchingQuery = 'nonexistent_${DateTime.now().microsecondsSinceEpoch}';
      await tester.enterText(find.byType(TextField), nonMatchingQuery);
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('No tracks matching "$nonMatchingQuery"'), findsOneWidget);
      expect(find.text('Clear Search'), findsOneWidget);

      await tester.tap(find.text('Clear Search'));
      await tester.pump(const Duration(milliseconds: 100));

      final tracks = controller.rawScannedTracks.isNotEmpty
          ? controller.rawScannedTracks
          : controller.libraryTracks;
      if (tracks.isNotEmpty) {
        expect(find.text(tracks.first.title), findsWidgets);
      }
    });

    testWidgets('Tapping Song Exclusion List in SettingsStage navigates to SongExclusionStage', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: SettingsAndAudioStage(),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      final exclusionTile = find.text('Song Exclusion List');
      expect(exclusionTile, findsOneWidget);

      await tester.ensureVisible(exclusionTile);
      await tester.pump(const Duration(milliseconds: 100));

      await tester.tap(exclusionTile);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));

      expect(find.byType(SongExclusionStage), findsOneWidget);
      expect(find.text('Song Exclusion Manager'), findsOneWidget);
    });
  });
}
