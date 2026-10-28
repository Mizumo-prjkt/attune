import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/services/playback_controller.dart';
import 'package:attune_app/services/settings_service.dart';
import 'package:attune_app/stages/profile_and_stats_stage.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final controller = PlaybackController.instance;
  final settings = SettingsService.instance;

  TrackItem createTrack({
    required String id,
    required String title,
    required String artist,
    required String genre,
    required double bpm,
    required int sampleRate,
    required int bitDepth,
    required bool isLossless,
    int durationSec = 180,
  }) {
    return TrackItem(
      id: id,
      title: title,
      artist: artist,
      album: 'Test Album',
      filePath: '/music/$id.flac',
      duration: Duration(seconds: durationSec),
      sampleRate: sampleRate,
      bitDepth: bitDepth,
      bpm: bpm,
      musicalKey: '8A',
      genre: genre,
      isLossless: isLossless,
      format: isLossless ? 'flac' : 'mp3',
    );
  }

  setUp(() {
    settings.setProfileName('Audiophile Tester');
  });

  testWidgets('ProfileAndStatsStage displays dynamic profile name, initial, and supports rename dialog', (WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1080, 1920));

    controller.setTracksForTesting([
      createTrack(
        id: '1',
        title: 'Song 1',
        artist: 'Artist 1',
        genre: 'Synthwave',
        bpm: 120.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: true,
      ),
    ]);

    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ProfileAndStatsStage(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Verify dynamic profile name and initial
    expect(find.text('Audiophile Tester'), findsOneWidget);
    expect(find.text('A'), findsOneWidget);

    // Tap to open edit dialog
    await tester.tap(find.text('Audiophile Tester'));
    await tester.pumpAndSettle();

    expect(find.text('Edit Profile Name'), findsOneWidget);
    final textField = find.byType(TextField);
    expect(textField, findsOneWidget);

    await tester.enterText(textField, 'Cosmic Voyager');
    await tester.pumpAndSettle();

    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    // Verify name changed dynamically in SettingsService and on UI
    expect(settings.profileName, equals('Cosmic Voyager'));
    expect(find.text('Cosmic Voyager'), findsOneWidget);
    expect(find.text('C'), findsOneWidget);
  });

  testWidgets('Dynamic listener persona adapts based on library composition', (WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1080, 1920));

    // 1. Empty library -> New Listener
    controller.setTracksForTesting([]);
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ProfileAndStatsStage(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('New Listener'), findsOneWidget);

    // 2. Predominantly Hi-Res -> Hi-Res Purist
    controller.setTracksForTesting([
      createTrack(
        id: 'hr1',
        title: 'Master Track 1',
        artist: 'Studio',
        genre: 'Jazz',
        bpm: 90.0,
        sampleRate: 96000,
        bitDepth: 24,
        isLossless: true,
      ),
      createTrack(
        id: 'hr2',
        title: 'Master Track 2',
        artist: 'Studio',
        genre: 'Jazz',
        bpm: 92.0,
        sampleRate: 192000,
        bitDepth: 24,
        isLossless: true,
      ),
      createTrack(
        id: 'cd1',
        title: 'CD Track',
        artist: 'Studio',
        genre: 'Jazz',
        bpm: 95.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: true,
      ),
    ]);
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ProfileAndStatsStage(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Hi-Res Purist'), findsOneWidget);

    // 3. Dominant genre -> [Genre] Connoisseur
    controller.setTracksForTesting([
      createTrack(
        id: 'g1',
        title: 'Rock Anthem',
        artist: 'Band',
        genre: 'J-Rock',
        bpm: 150.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: false,
      ),
      createTrack(
        id: 'g2',
        title: 'Another Rock Song',
        artist: 'Band',
        genre: 'J-Rock',
        bpm: 140.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: false,
      ),
      createTrack(
        id: 'g3',
        title: 'A Pop Song',
        artist: 'Idol',
        genre: 'Pop',
        bpm: 120.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: false,
      ),
    ]);
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ProfileAndStatsStage(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('J-Rock Connoisseur'), findsOneWidget);
  });

  testWidgets('Transition engine and DAC metrics compute real values without hardcoding', (WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1080, 1920));

    // When 0 transitions completed, calculates phrase estimation from average BPM (150 BPM => (4*60)/150 = 1.6s)
    controller.setTracksForTesting(
      [
        createTrack(
          id: 't1',
          title: 'Fast Track',
          artist: 'DJ',
          genre: 'Drum and Bass',
          bpm: 150.0,
          sampleRate: 48000,
          bitDepth: 24,
          isLossless: true,
        ),
      ],
      hardwareSampleRate: 48000,
      transitionsCount: 0,
    );

    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ProfileAndStatsStage(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // 1.6s 4-Beat estimated from 150 BPM
    expect(find.text('1.6s 4-Beat'), findsOneWidget);
    expect(find.textContaining('Avg 150 BPM'), findsOneWidget);

    // Bit-perfect when 48kHz track on 48kHz hardware
    expect(find.text('1.000x Bit-Perfect'), findsOneWidget);
    expect(find.text('48.0kHz Native DAC'), findsOneWidget);

    // When transitions have completed, displays real count and last recipe details
    controller.setTracksForTesting(
      [
        createTrack(
          id: 't1',
          title: 'Track A',
          artist: 'DJ',
          genre: 'Techno',
          bpm: 128.0,
          sampleRate: 44100,
          bitDepth: 16,
          isLossless: true,
        ),
      ],
      hardwareSampleRate: 48000,
      transitionsCount: 7,
      lastArch: 'Riff Handoff',
      lastDur: 2.1,
    );

    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ProfileAndStatsStage(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('7 Mixed'), findsOneWidget);
    expect(find.text('Riff Handoff (2.1s)'), findsOneWidget);

    // Resampling ratio: 48000 / 44100 = 1.0884... -> 1.088x Hermite
    expect(find.text('1.088x Hermite'), findsOneWidget);
    expect(find.text('44.1k → 48.0k Native'), findsOneWidget);
  });

  testWidgets('Dynamic genre distribution shows real genres without hardcoded mock chips', (WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1080, 1920));

    controller.setTracksForTesting([
      createTrack(
        id: '1',
        title: 'City Pop Song',
        artist: 'Tatsuro',
        genre: 'City Pop',
        bpm: 110.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: true,
      ),
      createTrack(
        id: '2',
        title: 'Chillhop Song',
        artist: 'Nujabes',
        genre: 'Chillhop',
        bpm: 85.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: true,
      ),
      createTrack(
        id: '3',
        title: 'City Pop 2',
        artist: 'Mariya',
        genre: 'City Pop',
        bpm: 115.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: true,
      ),
    ]);

    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ProfileAndStatsStage(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Verify real dynamic genres are present with counts and percentages
    expect(find.text('City Pop: 2 (67%)'), findsOneWidget);
    expect(find.text('Chillhop: 1 (33%)'), findsOneWidget);

    // Verify old hardcoded mock tags are NOT present
    expect(find.textContaining('Other / Soulseek'), findsNothing);
    expect(find.textContaining('Rock / Punk:'), findsNothing);
  });

  testWidgets('Audio fidelity breakdown accurately reflects Hi-Res, Lossless, and Compressed counts', (WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1080, 1920));

    controller.setTracksForTesting([
      createTrack(
        id: '1',
        title: 'Hi-Res Flac',
        artist: 'Band',
        genre: 'Rock',
        bpm: 120.0,
        sampleRate: 96000,
        bitDepth: 24,
        isLossless: true,
      ),
      createTrack(
        id: '2',
        title: 'CD Flac',
        artist: 'Band',
        genre: 'Rock',
        bpm: 120.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: true,
      ),
      createTrack(
        id: '3',
        title: 'MP3 Lossy',
        artist: 'Band',
        genre: 'Rock',
        bpm: 120.0,
        sampleRate: 44100,
        bitDepth: 16,
        isLossless: false,
      ),
    ]);

    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ProfileAndStatsStage(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('AUDIO FIDELITY BREAKDOWN'), findsOneWidget);
    expect(find.text('67% Lossless'), findsOneWidget);
    expect(find.text('Hi-Res (1) • 33%'), findsOneWidget);
    expect(find.text('Standard Lossless (1) • 33%'), findsOneWidget);
    expect(find.text('Compressed (1) • 33%'), findsOneWidget);
  });
}
