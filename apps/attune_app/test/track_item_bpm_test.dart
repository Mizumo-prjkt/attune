import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/track_item.dart';

void main() {
  group('TrackItem BPM Detection & Fallback Tests', () {
    test('Track with direct metadata BPM', () {
      const track = TrackItem(
        id: '1',
        title: 'Track One',
        artist: 'Artist One',
        album: 'Album One',
        filePath: '/music/track1.flac',
        duration: Duration(minutes: 3),
        bpm: 128.0,
        musicalKey: '8A',
      );

      expect(track.hasBpm, isTrue);
      expect(track.effectiveBpm, equals(128.0));
      expect(track.formattedBpm, equals('128.0 BPM'));
    });

    test('Track with zero metadata BPM falls back to tapProfile nominal_bpm', () {
      const track = TrackItem(
        id: '2',
        title: 'Track Two',
        artist: 'Artist Two',
        album: 'Album Two',
        filePath: '/music/track2.flac',
        duration: Duration(minutes: 4),
        bpm: 0.0,
        musicalKey: 'Pending Profile',
        tapProfile: {
          'global_summary': {
            'nominal_bpm': 132.5,
            'primary_key': '11B',
          },
        },
      );

      expect(track.hasBpm, isTrue);
      expect(track.effectiveBpm, equals(132.5));
      expect(track.formattedBpm, equals('132.5 BPM'));
    });

    test('Track with zero metadata BPM and no profile returns unprofiled fallback', () {
      const track = TrackItem(
        id: '3',
        title: 'Track Three',
        artist: 'Artist Three',
        album: 'Album Three',
        filePath: '/music/track3.flac',
        duration: Duration(minutes: 2),
        bpm: 0.0,
        musicalKey: 'Pending Profile',
      );

      expect(track.hasBpm, isFalse);
      expect(track.effectiveBpm, equals(0.0));
      expect(track.formattedBpm, equals('-- BPM'));
    });

    test('Track with both metadata and tapProfile prioritizes metadata tag', () {
      const track = TrackItem(
        id: '4',
        title: 'Track Four',
        artist: 'Artist Four',
        album: 'Album Four',
        filePath: '/music/track4.flac',
        duration: Duration(minutes: 5),
        bpm: 124.0,
        musicalKey: '5A',
        tapProfile: {
          'global_summary': {
            'nominal_bpm': 125.0,
          },
        },
      );

      expect(track.hasBpm, isTrue);
      expect(track.effectiveBpm, equals(124.0));
      expect(track.formattedBpm, equals('124.0 BPM'));
    });
  });
}
