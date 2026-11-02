import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/services/audio_storage_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('TrackItem Memory & Compact Profile Tests', () {
    test('extractCompactProfile strips heavy cue_zones and keeps global_summary', () {
      final heavyTap = {
        'schema_version': 1,
        'track_id': {'title': 'Epic Song', 'duration_ms': 240000},
        'analysis_tier': 'Tier1FullCqt',
        'global_summary': {
          'nominal_bpm': 128.0,
          'primary_key': 'A Minor',
          'mean_aggression_index': 0.75,
        },
        'cue_zones': [
          {
            'zone_type': 'Outro',
            'start_ms': 180000,
            'end_ms': 240000,
            'frames_20hz': {
              'energy_4band_db': List.generate(1200, (i) => [0.1, 0.2, 0.3, 0.4]),
              'chroma_12': List.generate(1200, (i) => List.filled(12, 0.5)),
            },
          },
        ],
      };

      final compact = TrackItem.extractCompactProfile(heavyTap);
      expect(compact.containsKey('cue_zones'), isFalse);
      expect(compact['schema_version'], equals(1));
      expect(compact['global_summary']['nominal_bpm'], equals(128.0));
      expect(compact['global_summary']['primary_key'], equals('A Minor'));
      expect(compact['global_summary']['mean_aggression_index'], equals(0.75));
    });

    test('TrackItem toJson and fromJson preserve compact profile without bloating', () {
      final heavyTap = {
        'schema_version': 1,
        'track_id': {'title': 'Test Song'},
        'global_summary': {'nominal_bpm': 140.0, 'primary_key': 'C Major'},
        'cue_zones': [
          {'zone': 'Outro', 'large_array': List.filled(5000, 1.0)},
        ],
      };

      const track = TrackItem(
        id: 'test_1',
        title: 'Test Song',
        artist: 'Test Artist',
        album: 'Test Album',
        duration: Duration(minutes: 3),
        bpm: 140.0,
        musicalKey: 'C Major',
        filePath: '/music/test.mp3',
      );

      final trackWithTap = track.copyWith(tapProfile: heavyTap);
      final json = trackWithTap.toJson();

      // Check serialized json stripped cue_zones
      final tapJson = json['tapProfile'] as Map<String, dynamic>;
      expect(tapJson.containsKey('cue_zones'), isFalse);
      expect(tapJson['global_summary']['nominal_bpm'], equals(140.0));

      final restored = TrackItem.fromJson(json);
      expect(restored.bpm, equals(140.0));
      expect(restored.effectiveBpm, equals(140.0));
      expect(restored.hasBpm, isTrue);
      expect(restored.tapProfile!.containsKey('cue_zones'), isFalse);
    });
  });

  group('LibraryCacheMeta & 7-Day TTL Tests', () {
    test('LibraryCacheMeta serializes and deserializes accurately', () {
      final now = DateTime.now();
      final meta = LibraryCacheMeta(
        cachedAt: now,
        trackCount: 159,
        roots: ['/home/miku/Music'],
      );

      final json = meta.toJson();
      final restored = LibraryCacheMeta.fromJson(json);

      expect(restored.trackCount, equals(159));
      expect(restored.roots, equals(['/home/miku/Music']));
      expect(restored.cachedAt.difference(now).inSeconds.abs(), lessThanOrEqualTo(1));
    });

    test('Detects cache expiration when data is >= 7 days old', () {
      final oldDate = DateTime.now().subtract(const Duration(days: 8));
      final meta = LibraryCacheMeta(cachedAt: oldDate, trackCount: 159);
      final age = DateTime.now().difference(meta.cachedAt);
      expect(age.inDays >= 7, isTrue);

      final freshDate = DateTime.now().subtract(const Duration(days: 2));
      final freshMeta = LibraryCacheMeta(cachedAt: freshDate, trackCount: 159);
      final freshAge = DateTime.now().difference(freshMeta.cachedAt);
      expect(freshAge.inDays >= 7, isFalse);
    });
  });
}
