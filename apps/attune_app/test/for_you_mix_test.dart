import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/for_you_mix.dart';
import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/services/for_you_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('ForYouService Mix Generation Tests', () {
    testWidgets('Generates sample fallback mixes when library is empty', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) {
              final mixes = ForYouService.instance.generateMixes(context, []);
              expect(mixes.length, 5);

              expect(mixes[0].id, 'mix_quick_picks');
              expect(mixes[0].artStyle, ForYouArtStyle.geometricPulse);
              expect(mixes[0].tracks.isNotEmpty, true);

              expect(mixes[1].id, 'mix_energy');
              expect(mixes[1].artStyle, ForYouArtStyle.starburst);

              expect(mixes[2].id, 'mix_chill');
              expect(mixes[2].artStyle, ForYouArtStyle.sonicWave);

              expect(mixes[3].id, 'mix_bpm_sync');
              expect(mixes[3].artStyle, ForYouArtStyle.radialDisc);

              expect(mixes[4].id, 'mix_rediscover');
              expect(mixes[4].artStyle, ForYouArtStyle.orbitBloom);

              return const SizedBox();
            },
          ),
        ),
      );
    });

    testWidgets('Clusters and buckets mixes from real library tracks', (tester) async {
      final List<TrackItem> mockTracks = [
        const TrackItem(
          id: '1',
          title: 'High Energy Rock',
          artist: 'Artist A',
          album: 'Album 1',
          duration: Duration(minutes: 3),
          filePath: '/path/1.mp3',
          bpm: 140.0,
          musicalKey: '8B',
          genre: 'Rock',
        ),
        const TrackItem(
          id: '2',
          title: 'Chill Acoustic',
          artist: 'Artist B',
          album: 'Album 2',
          duration: Duration(minutes: 3, seconds: 30),
          filePath: '/path/2.flac',
          bpm: 85.0,
          musicalKey: '11B',
          genre: 'Acoustic',
        ),
        const TrackItem(
          id: '3',
          title: 'Club Groove',
          artist: 'Artist C',
          album: 'Album 3',
          duration: Duration(minutes: 3, seconds: 15),
          filePath: '/path/3.wav',
          bpm: 128.0,
          musicalKey: '4A',
          genre: 'Electronic',
        ),
      ];

      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) {
              final mixes = ForYouService.instance.generateMixes(context, mockTracks);
              expect(mixes.length, 5);

              final energyMix = mixes.firstWhere((m) => m.id == 'mix_energy');
              expect(energyMix.tracks.any((t) => t.bpm >= 125 || t.genre == 'Rock'), true);

              final chillMix = mixes.firstWhere((m) => m.id == 'mix_chill');
              expect(chillMix.tracks.any((t) => t.genre == 'Acoustic'), true);

              return const SizedBox();
            },
          ),
        ),
      );
    });
  });
}
