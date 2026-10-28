import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/models/track_item.dart';
import 'package:attune_app/services/environment_delegator.dart';
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

    test('parseLrcText treats untimed text as static list without synthesizing fake timestamps', () {
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
      // Untimed lyrics must NOT have fake synthesized timestamps or auto-scroll markers
      for (final line in lines) {
        expect(line.timeSec, -1.0);
        expect(line.isEstimated, isFalse);
      }
      expect(lines[0].text, 'Verse 1 line 1');
      expect(lines[3].text, 'Chorus line 2');
    });

    test('parseTtmlText parses beat-by-beat syllable spans, aligned romaji, and background vocals', () {
      const rawTtml = '''<?xml version="1.0" encoding="utf-8"?>
<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata">
  <body>
    <div>
      <p begin="00:12.100" end="00:15.800">
        <span begin="00:12.100" end="00:12.450">き</span>
        <span begin="00:12.450" end="00:12.800">み</span>
        <span begin="00:12.800" end="00:13.150">が</span>
        <span begin="00:13.150" end="00:13.600"> </span>
        <span begin="00:13.600" end="00:14.200">涙</span>
      </p>
      <p begin="00:12.100" end="00:15.800" ttm:role="x-roman">
        <span begin="00:12.100" end="00:12.450">ki</span>
        <span begin="00:12.450" end="00:12.800">mi</span>
        <span begin="00:12.800" end="00:13.150">ga</span>
        <span begin="00:13.150" end="00:13.600"> </span>
        <span begin="00:13.600" end="00:14.200">namida</span>
      </p>
      <p begin="00:16.000" end="00:18.500" ttm:role="x-bg">
        <span begin="00:16.000" end="00:18.500">(Ah... yeah)</span>
      </p>
    </div>
  </body>
</tt>''';

      final lines = LyricsService.instance.parseTtmlText(rawTtml);
      expect(lines.length, 2);

      // Line 1: Main vocal with paired aligned romaji
      final line1 = lines[0];
      expect(line1.timeSec, closeTo(12.10, 0.01));
      expect(line1.hasSyllableSync, isTrue);
      expect(line1.words.length, 5);
      expect(line1.words[0].text, 'き');
      expect(line1.words[0].roman, 'ki');
      expect(line1.words[4].text, '涙');
      expect(line1.words[4].roman, 'namida');
      expect(line1.romanText, contains('namida'));
      expect(line1.isBackgroundVocal, isFalse);

      // Line 2: Background vocal
      final line2 = lines[1];
      expect(line2.timeSec, closeTo(16.00, 0.01));
      expect(line2.text, '(Ah... yeah)');
      expect(line2.isBackgroundVocal, isTrue);
    });

    test('parseEnhancedLrcText parses inline syllable markers and [romaji] companion lines', () {
      const elrc = '''
[00:12.10] <00:12.10>き <00:12.45>み <00:12.80>が <00:13.60>涙<00:14.20>
[00:12.10][romaji] <00:12.10>ki <00:12.45>mi <00:12.80>ga <00:13.60>namida<00:14.20>
''';

      final lines = LyricsService.instance.parseEnhancedLrcText(elrc);
      expect(lines.length, 1);

      final line = lines.first;
      expect(line.hasSyllableSync, isTrue);
      expect(line.words.length, 4);
      expect(line.words[0].text, 'き ');
      expect(line.words[0].roman, 'ki');
      expect(line.words[3].text, '涙');
      expect(line.words[3].roman, 'namida');
    });

    test('parseYrcText parses NetEase millisecond syllable tags accurately', () {
      const yrc = '[12100,3700](12100,350,0)き(12450,350,0)み(12800,350,0)が(13600,600,0)涙';
      final lines = LyricsService.instance.parseYrcText(yrc);
      expect(lines.length, 1);

      final line = lines.first;
      expect(line.timeSec, closeTo(12.10, 0.01));
      expect(line.endSec, closeTo(15.80, 0.01));
      expect(line.words.length, 4);
      expect(line.words[0].text, 'き');
      expect(line.words[0].startSec, closeTo(12.10, 0.01));
      expect(line.words[0].endSec, closeTo(12.45, 0.01));
    });

    test('WordSpan.progressAt calculates smooth intra-word sweep with 40ms lookahead', () {
      const word = WordSpan(
        text: '涙',
        startSec: 10.000,
        endSec: 11.000,
        roman: 'namida',
      );

      // Before word (account for 40ms lookahead)
      expect(word.progressAt(9.90), 0.0);

      // At start (lookahead activates slightly early)
      expect(word.progressAt(9.960), closeTo(0.0, 0.01));

      // Mid-word: 50% sweep through "涙" and "namida"
      expect(word.progressAt(10.460), closeTo(0.50, 0.02));

      // Completed
      expect(word.progressAt(11.00), 1.0);
      expect(word.progressAt(12.00), 1.0);
    });

    test('AlignmentCircuitBreaker verifies hardware cores threshold', () {
      expect(AlignmentCircuitBreaker.minCoresRequired, 4);
      expect(AlignmentCircuitBreaker.maxRtfThreshold, 0.35);
      expect(AlignmentCircuitBreaker.canRunLocalAlignment(), isA<bool>());
    });

    test('Syllable Sync returns clean empty state when no syllable files exist', () async {
      const track = TrackItem(
        id: 'rabbit_hole_test',
        title: 'Rabbit Hole',
        artist: 'DECO*27',
        album: 'Rabbit Hole',
        filePath: '/tmp/test_rabbit_hole_01.flac',
        duration: Duration(minutes: 2, seconds: 39),
        bpm: 135.0,
        musicalKey: '11B',
      );

      final result = await LyricsService.instance.loadLyrics(track, LyricSourceType.syllableSync);
      expect(result.source, LyricSourceType.syllableSync);
      expect(result.hasLyrics, isFalse);
      expect(result.statusMessage, contains('syllable-synchronized'));
    });

    test('determinePreferredSource prioritizes timed lyrics according to user matrix', () async {
      // 1. Dynamic probe for test track with companion .lrc
      final roots = AppEnvironmentDelegator.current.getMusicCandidateRoots();
      String? testFlacPath;
      for (final root in roots) {
        final p = '$root/Machine Love/01 Machine Love.flac';
        if (File(p).existsSync()) {
          testFlacPath = p;
          break;
        }
      }

      if (testFlacPath != null) {
        final machineLoveTrack = TrackItem(
          id: 'machine_love_pref',
          title: 'Machine Love',
          artist: 'Jamie Paige',
          album: 'Machine Love',
          filePath: testFlacPath,
          duration: const Duration(minutes: 3, seconds: 36),
          bpm: 124.0,
          musicalKey: '8B',
        );

        final preferred = await LyricsService.instance.determinePreferredSource(machineLoveTrack);
        expect(preferred, anyOf(LyricSourceType.lrcFile, LyricSourceType.embedded));
      }

      // 2. Non-existent track defaults cleanly to embedded
      const emptyTrack = TrackItem(
        id: 'empty_track_pref',
        title: 'Non-Existent Song',
        artist: 'Unknown',
        album: 'Unknown',
        filePath: '/tmp/non_existent_file.flac',
        duration: Duration(minutes: 3),
        bpm: 120.0,
        musicalKey: '8B',
      );

      final emptyPref = await LyricsService.instance.determinePreferredSource(emptyTrack);
      expect(emptyPref, LyricSourceType.embedded);
    });

    test('Resolves real companion .lrc file if present on local disk', () async {
      final roots = AppEnvironmentDelegator.current.getMusicCandidateRoots();
      String? testFlacPath;
      for (final root in roots) {
        final p = '$root/Machine Love/01 Machine Love.flac';
        final lrc = '$root/Machine Love/01 Machine Love.lrc';
        if (File(p).existsSync() && File(lrc).existsSync()) {
          testFlacPath = p;
          break;
        }
      }

      if (testFlacPath == null) {
        // Skip if running in environment without local music files
        return;
      }

      final track = TrackItem(
        id: 'machine_love_test',
        title: 'Machine Love',
        artist: 'Jamie Paige',
        album: 'Machine Love',
        filePath: testFlacPath,
        duration: const Duration(minutes: 3, seconds: 36),
        bpm: 124.0,
        musicalKey: '8B',
      );

      final result = await LyricsService.instance.loadLyrics(track, LyricSourceType.lrcFile);
      expect(result.hasLyrics, isTrue);
      expect(result.isTimed, isTrue);
      expect(result.lines.length, greaterThan(20));
      expect(result.lines.first.text, contains('Your companion, undeterred'));
    });
  });
}
