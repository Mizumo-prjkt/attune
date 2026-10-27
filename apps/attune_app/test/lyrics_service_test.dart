import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/services/lyrics_service.dart';

void main() {
  group('LyricsService Unit Tests', () {
    test('parseLrcText parses standard timestamped LRC with millisecond accuracy', () {
      const rawLrc = '''
[ti:Tokyo Teddy Bear]
[ar:Neru]
[al:Sekai Seifuku]
[00:00.80] Father, Mother, I am sorry until now
[00:05.34] Knees trembling, sucking my thumb
[00:10.12]
[00:29.01] Taking pattern measurements in an overreaching size
''';

      final lines = LyricsService.instance.parseLrcText(rawLrc);

      expect(lines.length, 4);

      // Line 1
      expect(lines[0].timeSec, closeTo(0.80, 0.01));
      expect(lines[0].text, 'Father, Mother, I am sorry until now');
      expect(lines[0].isInstrumental, isFalse);

      // Line 2
      expect(lines[1].timeSec, closeTo(5.34, 0.01));
      expect(lines[1].text, 'Knees trembling, sucking my thumb');

      // Line 3 (Empty text marks instrumental)
      expect(lines[2].timeSec, closeTo(10.12, 0.01));
      expect(lines[2].isInstrumental, isTrue);

      // Line 4
      expect(lines[3].timeSec, closeTo(29.01, 0.01));
      expect(lines[3].text, 'Taking pattern measurements in an overreaching size');
    });

    test('parseLrcText handles multiple timestamps on a single line', () {
      const multiTimestamp = '''
[00:15.50][00:45.50] Love, blah, blah, blah
''';
      final lines = LyricsService.instance.parseLrcText(multiTimestamp);
      expect(lines.length, 2);
      expect(lines[0].timeSec, closeTo(15.50, 0.01));
      expect(lines[1].timeSec, closeTo(45.50, 0.01));
      expect(lines[0].text, 'Love, blah, blah, blah');
      expect(lines[1].text, 'Love, blah, blah, blah');
    });

    test('parseLrcText gracefully distributes unsynchronized lines across track duration', () {
      const plainText = '''
Verse 1 line 1
Verse 1 line 2
Chorus line 1
Chorus line 2
''';
      final lines = LyricsService.instance.parseLrcText(
        plainText,
        duration: const Duration(seconds: 120),
      );

      expect(lines.length, 4);
      expect(lines[0].isEstimated, isTrue);
      expect(lines[0].timeSec, greaterThanOrEqualTo(4.0));
      expect(lines[3].timeSec, lessThanOrEqualTo(120.0));
      // Timestamps must be strictly ascending
      for (var i = 1; i < lines.length; i++) {
        expect(lines[i].timeSec, greaterThan(lines[i - 1].timeSec));
      }
    });

    test('Resolves real companion .lrc file if present on local disk', () async {
      const realLrcPath = '/home/miku/Music/Soulseek Complete/Machine Love/01 Machine Love.lrc';
      final file = File(realLrcPath);
      if (!file.existsSync()) {
        // Skip if running in environment without local music files
        return;
      }

      const track = TrackItem(
        id: 'machine_love_test',
        title: 'Machine Love',
        artist: 'Jamie Paige',
        album: 'Machine Love',
        filePath: '/home/miku/Music/Soulseek Complete/Machine Love/01 Machine Love.flac',
        duration: Duration(minutes: 3, seconds: 36),
        bpm: 124.0,
        musicalKey: '8B',
      );

      final result = await LyricsService.instance.loadLyrics(track, LyricSourceType.lrcFile);
      expect(result.hasLyrics, isTrue);
      expect(result.lines.length, greaterThan(20));
      expect(result.lines.first.text, contains('Your companion, undeterred'));
    });
  });
}
