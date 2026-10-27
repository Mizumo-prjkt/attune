import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import '../models/track_item.dart';
import 'audio_storage_service.dart';

/// Available lyric source modes
enum LyricSourceType {
  embedded,
  lrcFile,
  syllableSync,
}

/// Represents a single word or syllable span within a synchronized line
class WordSpan {
  final String text;
  final double startSec;
  final double endSec;
  final String? roman; // Aligned phonetic/romaji transliteration

  const WordSpan({
    required this.text,
    required this.startSec,
    required this.endSec,
    this.roman,
  });

  /// Computes intra-word / intra-syllable sweeping progress [0.0, 1.0]
  /// with a 40ms perceptual lookahead to match auditory-visual synchronization.
  double progressAt(double nowSec) {
    final adjustedNow = nowSec + 0.040; // 40ms lookahead
    if (adjustedNow <= startSec) return 0.0;
    if (adjustedNow >= endSec) return 1.0;
    final dur = endSec - startSec;
    if (dur <= 0.001) return 1.0;
    return ((adjustedNow - startSec) / dur).clamp(0.0, 1.0);
  }

  @override
  String toString() => 'WordSpan("$text" [${startSec.toStringAsFixed(2)}s - ${endSec.toStringAsFixed(2)}s]${roman != null ? ' ($roman)' : ''})';
}

/// Represents a single synchronized, syllable-timed, or estimated lyric line
class LyricLine {
  final double timeSec;
  final double endSec;
  final String text;
  final String? romanText; // Line-level romaji if present
  final List<WordSpan> words; // Syllable / word spans
  final bool isInstrumental;
  final bool isEstimated;
  final bool isBackgroundVocal; // e.g. ttm:role="x-bg" / harmony

  const LyricLine({
    required this.timeSec,
    this.endSec = -1.0,
    required this.text,
    this.romanText,
    this.words = const [],
    this.isInstrumental = false,
    this.isEstimated = false,
    this.isBackgroundVocal = false,
  });

  bool get hasSyllableSync => words.isNotEmpty;

  @override
  String toString() => 'LyricLine(${timeSec.toStringAsFixed(2)}s: "$text"${words.isNotEmpty ? ' [${words.length} words]' : ''})';
}

/// Output payload from LyricsService
class LyricLoadResult {
  final LyricSourceType source;
  final List<LyricLine> lines;
  final String? sourcePath;
  final String? rawText;
  final bool hasLyrics;
  final bool isTimed;
  final String? statusMessage;

  const LyricLoadResult({
    required this.source,
    required this.lines,
    this.sourcePath,
    this.rawText,
    required this.hasLyrics,
    this.isTimed = false,
    this.statusMessage,
  });

  factory LyricLoadResult.empty(LyricSourceType source, {String? statusMessage}) {
    return LyricLoadResult(
      source: source,
      lines: const [],
      hasLyrics: false,
      isTimed: false,
      statusMessage: statusMessage ?? 'No lyrics found for this source.',
    );
  }
}

/// Production-grade real lyrics resolution service.
/// Decodes real embedded metadata (FLAC Vorbis comments, MP3 ID3v2 USLT/SYLT, M4A)
/// and resolves companion .lrc / .txt files from local storage without hardcoded catalogs.
class LyricsService {
  static final LyricsService instance = LyricsService._();
  LyricsService._();

  // In-memory cache keyed by track ID and source type
  final Map<String, Map<LyricSourceType, LyricLoadResult>> _cache = {};

  /// Loads lyrics for [track] from the requested [source].
  Future<LyricLoadResult> loadLyrics(TrackItem track, LyricSourceType source) async {
    final trackId = track.id.isNotEmpty ? track.id : track.filePath;
    if (_cache[trackId]?.containsKey(source) == true) {
      return _cache[trackId]![source]!;
    }

    LyricLoadResult result;
    try {
      switch (source) {
        case LyricSourceType.embedded:
          result = await _loadEmbeddedLyrics(track);
          break;
        case LyricSourceType.lrcFile:
          result = await _loadLrcFileLyrics(track);
          break;
        case LyricSourceType.syllableSync:
          result = await _loadSyllableSyncLyrics(track);
          break;
      }
    } catch (e) {
      debugPrint('[LyricsService] Error loading $source lyrics: $e');
      result = LyricLoadResult.empty(source, statusMessage: 'Error parsing lyrics: $e');
    }

    _cache.putIfAbsent(trackId, () => {})[source] = result;
    return result;
  }

  /// Clears cache for a track or all tracks
  void invalidateCache([String? trackId]) {
    if (trackId != null) {
      _cache.remove(trackId);
    } else {
      _cache.clear();
    }
  }

  // ---------------------------------------------------------------------------
  // 1. EMBEDDED LYRICS RESOLUTION (FLAC, MP3, M4A)
  // ---------------------------------------------------------------------------
  Future<LyricLoadResult> _loadEmbeddedLyrics(TrackItem track) async {
    final file = File(track.filePath);
    if (!file.existsSync()) {
      return LyricLoadResult.empty(
        LyricSourceType.embedded,
        statusMessage: 'Audio file not found on disk: ${track.filePath}',
      );
    }

    final ext = track.filePath.toLowerCase();
    String? rawLyrics;

    if (ext.endsWith('.flac') || ext.endsWith('.ogg') || ext.endsWith('.opus')) {
      rawLyrics = await _extractFlacVorbisLyrics(file);
    } else if (ext.endsWith('.mp3')) {
      rawLyrics = await _extractMp3Id3Lyrics(file);
    } else if (ext.endsWith('.m4a') || ext.endsWith('.mp4') || ext.endsWith('.alac')) {
      rawLyrics = await _extractM4aLyrics(file);
    }

    if (rawLyrics == null || rawLyrics.trim().isEmpty) {
      return LyricLoadResult.empty(
        LyricSourceType.embedded,
        statusMessage: 'No embedded Vorbis or ID3 lyrics found in this audio file.',
      );
    }

    final lines = parseLrcText(rawLyrics, duration: track.duration);
    final isTimed = lines.isNotEmpty && lines.any((l) => l.timeSec >= 0.0);
    return LyricLoadResult(
      source: LyricSourceType.embedded,
      lines: lines,
      sourcePath: track.filePath,
      rawText: rawLyrics,
      hasLyrics: lines.isNotEmpty,
      isTimed: isTimed,
      statusMessage: isTimed
          ? 'Loaded ${lines.length} synchronized lines from embedded audio tags.'
          : 'Loaded ${lines.length} static lyric lines from embedded audio tags.',
    );
  }

  Future<String?> _extractFlacVorbisLyrics(File file) async {
    RandomAccessFile? raf;
    try {
      raf = await file.open(mode: FileMode.read);
      final magic = await raf.read(4);
      if (magic.length < 4 ||
          magic[0] != 0x66 ||
          magic[1] != 0x4C ||
          magic[2] != 0x61 ||
          magic[3] != 0x43) {
        return null; // Not valid FLAC header
      }

      bool isLast = false;
      while (!isLast) {
        final header = await raf.read(4);
        if (header.length < 4) break;

        isLast = (header[0] & 0x80) != 0;
        final blockType = header[0] & 0x7F;
        final length = (header[1] << 16) | (header[2] << 8) | header[3];

        if (blockType == 4) {
          // VORBIS_COMMENT block
          final data = await raf.read(length);
          return _parseVorbisComments(data);
        } else {
          // Skip other metadata blocks (e.g. STREAMINFO, PADDING, PICTURE)
          await raf.setPosition(await raf.position() + length);
        }
      }
    } catch (e) {
      debugPrint('[LyricsService] FLAC extraction warning: $e');
    } finally {
      await raf?.close();
    }
    return null;
  }

  String? _parseVorbisComments(Uint8List data) {
    if (data.length < 8) return null;
    final byteData = ByteData.sublistView(data);
    var offset = 0;

    // Vendor length & string
    final vendorLength = byteData.getUint32(offset, Endian.little);
    offset += 4 + vendorLength;

    if (offset + 4 > data.length) return null;
    final numComments = byteData.getUint32(offset, Endian.little);
    offset += 4;

    String? syncedLyrics;
    String? unsyncedLyrics;

    for (var i = 0; i < numComments; i++) {
      if (offset + 4 > data.length) break;
      final commentLength = byteData.getUint32(offset, Endian.little);
      offset += 4;

      if (offset + commentLength > data.length) break;
      final commentBytes = data.sublist(offset, offset + commentLength);
      offset += commentLength;

      final commentStr = utf8.decode(commentBytes, allowMalformed: true);
      final eqIndex = commentStr.indexOf('=');
      if (eqIndex != -1) {
        final key = commentStr.substring(0, eqIndex).toUpperCase();
        final val = commentStr.substring(eqIndex + 1);

        if (key == 'LYRICS') {
          syncedLyrics = val;
        } else if (key == 'UNSYNCEDLYRICS' || key == 'UNSYNCED LYRICS' || key == 'LYRIC') {
          unsyncedLyrics = val;
        }
      }
    }

    return syncedLyrics ?? unsyncedLyrics;
  }

  Future<String?> _extractMp3Id3Lyrics(File file) async {
    RandomAccessFile? raf;
    try {
      raf = await file.open(mode: FileMode.read);
      final header = await raf.read(10);
      if (header.length < 10) return null;
      if (header[0] != 0x49 || header[1] != 0x44 || header[2] != 0x33) {
        return null; // Not ID3v2
      }

      final majorVersion = header[3];
      final tagSize = ((header[6] & 0x7F) << 21) |
          ((header[7] & 0x7F) << 14) |
          ((header[8] & 0x7F) << 7) |
          (header[9] & 0x7F);

      var bytesRead = 0;
      while (bytesRead < tagSize) {
        final frameHeader = await raf.read(10);
        if (frameHeader.length < 10) break;
        bytesRead += 10;

        if (frameHeader[0] == 0) break; // Padding reached

        final frameId = String.fromCharCodes(frameHeader.sublist(0, 4));
        int frameSize;
        if (majorVersion == 4) {
          frameSize = ((frameHeader[4] & 0x7F) << 21) |
              ((frameHeader[5] & 0x7F) << 14) |
              ((frameHeader[6] & 0x7F) << 7) |
              (frameHeader[7] & 0x7F);
        } else {
          frameSize = (frameHeader[4] << 24) |
              (frameHeader[5] << 16) |
              (frameHeader[6] << 8) |
              frameHeader[7];
        }

        if (frameSize <= 0 || bytesRead + frameSize > tagSize + 10) break;

        if (frameId == 'USLT' || frameId == 'SYLT') {
          final frameData = await raf.read(frameSize);
          bytesRead += frameSize;
          return _parseUsltFrame(frameData);
        } else {
          await raf.setPosition(await raf.position() + frameSize);
          bytesRead += frameSize;
        }
      }
    } catch (e) {
      debugPrint('[LyricsService] MP3 ID3 extraction warning: $e');
    } finally {
      await raf?.close();
    }
    return null;
  }

  String? _parseUsltFrame(Uint8List data) {
    if (data.length < 5) return null;
    final encoding = data[0];
    var offset = 4; // Skip encoding byte and 3-byte language code

    if (encoding == 0 || encoding == 3) {
      // 1-byte null delimiter for description
      while (offset < data.length && data[offset] != 0) {
        offset++;
      }
      offset++;
      if (offset >= data.length) return null;
      final lyricBytes = data.sublist(offset);
      return encoding == 3
          ? utf8.decode(lyricBytes, allowMalformed: true)
          : latin1.decode(lyricBytes);
    } else if (encoding == 1 || encoding == 2) {
      // 2-byte null delimiter for UTF-16 description
      while (offset + 1 < data.length) {
        if (data[offset] == 0 && data[offset + 1] == 0) {
          offset += 2;
          break;
        }
        offset += 2;
      }
      if (offset >= data.length) return null;
      final lyricBytes = data.sublist(offset);
      return _decodeUtf16(lyricBytes, isBigEndian: encoding == 2);
    }
    return null;
  }

  String _decodeUtf16(Uint8List bytes, {bool isBigEndian = false}) {
    if (bytes.length < 2) return '';
    var start = 0;
    bool be = isBigEndian;
    if (bytes[0] == 0xFE && bytes[1] == 0xFF) {
      be = true;
      start = 2;
    } else if (bytes[0] == 0xFF && bytes[1] == 0xFE) {
      be = false;
      start = 2;
    }

    final charCodes = <int>[];
    for (var i = start; i + 1 < bytes.length; i += 2) {
      final code = be ? (bytes[i] << 8) | bytes[i + 1] : bytes[i] | (bytes[i + 1] << 8);
      charCodes.add(code);
    }
    return String.fromCharCodes(charCodes);
  }

  Future<String?> _extractM4aLyrics(File file) async {
    // Basic MP4 ©lyr atom scanner
    try {
      final bytes = await file.openRead(0, 1024 * 512).expand((b) => b).toList();
      final data = Uint8List.fromList(bytes);
      // Search for '©lyr' atom: 0xA9 0x6C 0x79 0x72
      for (var i = 0; i < data.length - 8; i++) {
        if (data[i] == 0xA9 && data[i + 1] == 0x6C && data[i + 2] == 0x79 && data[i + 3] == 0x72) {
          // Atom found. Check for 'data' child atom
          var dataOffset = i + 4;
          while (dataOffset < data.length - 8 && dataOffset < i + 100) {
            if (data[dataOffset] == 0x64 &&
                data[dataOffset + 1] == 0x61 &&
                data[dataOffset + 2] == 0x74 &&
                data[dataOffset + 3] == 0x61) {
              // 'data' atom header: 4 bytes length, 4 bytes type, 4 bytes flags/version
              final contentStart = dataOffset + 12;
              if (contentStart < data.length) {
                return utf8.decode(data.sublist(contentStart), allowMalformed: true);
              }
            }
            dataOffset++;
          }
        }
      }
    } catch (_) {}
    return null;
  }

  // ---------------------------------------------------------------------------
  // 2. LRC FILE RESOLUTION (Local File System)
  // ---------------------------------------------------------------------------
  Future<LyricLoadResult> _loadLrcFileLyrics(TrackItem track) async {
    final lrcFile = await _resolveCompanionLrcFile(track);
    if (lrcFile == null || !lrcFile.existsSync()) {
      return LyricLoadResult.empty(
        LyricSourceType.lrcFile,
        statusMessage: 'No .lrc file found for "${track.title}" in music folders.',
      );
    }

    try {
      final content = await lrcFile.readAsString();
      final lines = parseLrcText(content, duration: track.duration);
      final isTimed = lines.isNotEmpty && lines.any((l) => l.timeSec >= 0.0);

      return LyricLoadResult(
        source: LyricSourceType.lrcFile,
        lines: lines,
        sourcePath: lrcFile.path,
        rawText: content,
        hasLyrics: lines.isNotEmpty,
        isTimed: isTimed,
        statusMessage: isTimed
            ? 'Loaded ${lines.length} synchronized lines from ${lrcFile.path.split(Platform.pathSeparator).last}.'
            : 'Loaded ${lines.length} static lyric lines from ${lrcFile.path.split(Platform.pathSeparator).last}.',
      );
    } catch (e) {
      return LyricLoadResult.empty(
        LyricSourceType.lrcFile,
        statusMessage: 'Failed to read .lrc file: $e',
      );
    }
  }

  /// Locates companion .lrc or .txt file next to audio file or in music library
  Future<File?> _resolveCompanionLrcFile(TrackItem track) async {
    final audioFile = File(track.filePath);
    final audioDir = audioFile.parent;

    if (audioFile.existsSync() && audioDir.existsSync()) {
      // 1. Direct path with .lrc extension: song.flac -> song.lrc
      final dotIndex = audioFile.path.lastIndexOf('.');
      if (dotIndex != -1) {
        final directLrc = File('${audioFile.path.substring(0, dotIndex)}.lrc');
        if (directLrc.existsSync()) return directLrc;
        final directTxt = File('${audioFile.path.substring(0, dotIndex)}.txt');
        if (directTxt.existsSync()) return directTxt;
      }

      // 2. Scan same directory for any .lrc or .txt files
      try {
        final entries = audioDir.listSync(followLinks: true);
        final titleTokens = _tokenize(track.title);

        for (final entity in entries) {
          if (entity is! File) continue;
          final lower = entity.path.toLowerCase();
          if (!lower.endsWith('.lrc') && !lower.endsWith('.txt')) continue;

          final lrcName = entity.path.split(Platform.pathSeparator).last.toLowerCase();
          // Direct token matching
          if (titleTokens.any((t) => t.length >= 3 && lrcName.contains(t))) {
            return entity;
          }
        }
      } catch (_) {}
    }

    // 3. Fallback: Scan known candidate roots for matching <title>.lrc
    final candidateRoots = AudioStorageService.instance.getCandidateRoots();
    final titleClean = track.title.toLowerCase().replaceAll(RegExp(r'[^a-z0-9ぁ-んァ-ヶ一-龥]'), '');

    for (final root in candidateRoots) {
      final dir = Directory(root);
      if (!dir.existsSync()) continue;

      try {
        final entities = dir.listSync(recursive: true, followLinks: true);
        for (final entity in entities) {
          if (entity is! File) continue;
          final lower = entity.path.toLowerCase();
          if (!lower.endsWith('.lrc')) continue;

          final filename = entity.path.split(Platform.pathSeparator).last.toLowerCase();
          final candidateClean = filename.replaceAll(RegExp(r'[^a-z0-9ぁ-んァ-ヶ一-龥]'), '');
          if (titleClean.isNotEmpty && candidateClean.contains(titleClean)) {
            return entity;
          }
        }
      } catch (_) {}
    }

    return null;
  }

  Set<String> _tokenize(String text) {
    return text
        .toLowerCase()
        .split(RegExp(r'[\s\-_\.,\(\)\[\]\*\+・]+'))
        .where((s) => s.trim().isNotEmpty)
        .toSet();
  }

  // ---------------------------------------------------------------------------
  // 3. SYLLABLE-SYNCHRONIZED LYRICS (.TTML / .ELRC / .YRC)
  // ---------------------------------------------------------------------------
  /// Loads beat-by-beat / syllable-synchronized lyrics for [track].
  /// Checks open-source companion files (.ttml, .elrc, .yrc, .qrc, .lys) first (zero compute),
  /// or inspects .lrc files for inline syllable timing markers.
  Future<LyricLoadResult> _loadSyllableSyncLyrics(TrackItem track) async {
    // 1. Check for dedicated syllable file (.ttml, .elrc, .yrc, .qrc, .lys, .xml)
    final syllableFile = await _resolveCompanionSyllableFile(track);
    if (syllableFile != null && syllableFile.existsSync()) {
      try {
        final content = await syllableFile.readAsString();
        final lines = parseSyllableText(content);
        final isTimed = lines.isNotEmpty && lines.any((l) => l.timeSec >= 0.0);

        return LyricLoadResult(
          source: LyricSourceType.syllableSync,
          lines: lines,
          sourcePath: syllableFile.path,
          rawText: content,
          hasLyrics: lines.isNotEmpty,
          isTimed: isTimed,
          statusMessage: 'Loaded ${lines.length} syllable-synchronized lines from ${syllableFile.path.split(Platform.pathSeparator).last}.',
        );
      } catch (e) {
        return LyricLoadResult.empty(
          LyricSourceType.syllableSync,
          statusMessage: 'Failed to read syllable file: $e',
        );
      }
    }

    // 2. Check if companion .lrc contains inline syllable markers (<mm:ss.xx> or [start,dur])
    final lrcFile = await _resolveCompanionLrcFile(track);
    if (lrcFile != null && lrcFile.existsSync()) {
      try {
        final content = await lrcFile.readAsString();
        if (RegExp(r'<\d{1,2}:\d{2}').hasMatch(content) || RegExp(r'\[\d+,\d+\]\(\d+,\d+').hasMatch(content)) {
          final lines = parseSyllableText(content);
          if (lines.any((l) => l.hasSyllableSync)) {
            final isTimed = lines.isNotEmpty && lines.any((l) => l.timeSec >= 0.0);
            return LyricLoadResult(
              source: LyricSourceType.syllableSync,
              lines: lines,
              sourcePath: lrcFile.path,
              rawText: content,
              hasLyrics: lines.isNotEmpty,
              isTimed: isTimed,
              statusMessage: 'Loaded ${lines.length} syllable-synchronized lines from ${lrcFile.path.split(Platform.pathSeparator).last}.',
            );
          }
        }
      } catch (_) {}
    }

    return LyricLoadResult.empty(
      LyricSourceType.syllableSync,
      statusMessage: 'No syllable-synchronized (.ttml, .elrc, .yrc) file found for "${track.title}". '
          'Syllable Sync requires beat-by-beat word timing data.',
    );
  }

  /// Locates companion syllable file (.ttml, .elrc, .yrc, .qrc, .lys, .xml)
  /// next to audio file or in candidate music directories.
  Future<File?> _resolveCompanionSyllableFile(TrackItem track) async {
    final audioFile = File(track.filePath);
    final audioDir = audioFile.parent;
    const extensions = ['.ttml', '.elrc', '.yrc', '.qrc', '.lys', '.xml'];

    if (audioFile.existsSync() && audioDir.existsSync()) {
      // 1. Direct path with matching extension: song.flac -> song.ttml, song.elrc, etc.
      final dotIndex = audioFile.path.lastIndexOf('.');
      if (dotIndex != -1) {
        final base = audioFile.path.substring(0, dotIndex);
        for (final ext in extensions) {
          final directFile = File('$base$ext');
          if (directFile.existsSync()) return directFile;
        }
      }

      // 2. Scan same directory for any matching syllable files by title tokens
      try {
        final entries = audioDir.listSync(followLinks: true);
        final titleTokens = _tokenize(track.title);

        for (final entity in entries) {
          if (entity is! File) continue;
          final lower = entity.path.toLowerCase();
          if (!extensions.any((ext) => lower.endsWith(ext))) continue;

          final filename = entity.path.split(Platform.pathSeparator).last.toLowerCase();
          if (titleTokens.any((t) => t.length >= 3 && filename.contains(t))) {
            return entity;
          }
        }
      } catch (_) {}
    }

    // 3. Fallback: Scan known candidate roots
    final candidateRoots = AudioStorageService.instance.getCandidateRoots();
    final titleClean = track.title.toLowerCase().replaceAll(RegExp(r'[^a-z0-9ぁ-んァ-ヶ一-龥]'), '');

    for (final root in candidateRoots) {
      final dir = Directory(root);
      if (!dir.existsSync()) continue;

      try {
        final entities = dir.listSync(recursive: true, followLinks: true);
        for (final entity in entities) {
          if (entity is! File) continue;
          final lower = entity.path.toLowerCase();
          if (!extensions.any((ext) => lower.endsWith(ext))) continue;

          final filename = entity.path.split(Platform.pathSeparator).last.toLowerCase();
          final candidateClean = filename.replaceAll(RegExp(r'[^a-z0-9ぁ-んァ-ヶ一-龥]'), '');
          if (titleClean.isNotEmpty && candidateClean.contains(titleClean)) {
            return entity;
          }
        }
      } catch (_) {}
    }

    return null;
  }

  /// Automatically identifies syllable format and dispatches to appropriate parser
  List<LyricLine> parseSyllableText(String rawText) {
    final trimmed = rawText.trim();
    if (trimmed.isEmpty) return const [];

    if (trimmed.contains('<tt') || trimmed.contains('<p') || trimmed.contains('<span')) {
      return parseTtmlText(rawText);
    } else if (RegExp(r'\[\d+,\d+\]\(\d+,\d+').hasMatch(trimmed)) {
      return parseYrcText(rawText);
    } else if (RegExp(r'<\d{1,2}:\d{2}').hasMatch(trimmed)) {
      return parseEnhancedLrcText(rawText);
    } else {
      final elrc = parseEnhancedLrcText(rawText);
      if (elrc.any((l) => l.hasSyllableSync)) return elrc;
      return parseLrcText(rawText);
    }
  }

  /// Parses AMLL / open specification TTML (.ttml / .xml) syllable-synchronized lyrics.
  /// Handles `<p begin="..." end="...">`, child `<span begin="..." end="...">`,
  /// aligned phonetic transliteration (`ttm:role="x-roman"`), and background vocals (`ttm:role="x-bg"`).
  List<LyricLine> parseTtmlText(String rawText) {
    if (rawText.trim().isEmpty) return const [];

    final parsed = <LyricLine>[];
    final pRegex = RegExp(r'<p\b([^>]*)>([\s\S]*?)<\/p>', caseSensitive: false);
    final spanRegex = RegExp(r'<span\b([^>]*)>([\s\S]*?)<\/span>', caseSensitive: false);
    final attrRegex = RegExp(r'''([a-zA-Z0-9:_-]+)\s*=\s*(?:"([^"]*)"|'([^']*)')''');

    Map<String, String> parseAttributes(String attrString) {
      final attrs = <String, String>{};
      for (final match in attrRegex.allMatches(attrString)) {
        final key = match.group(1)!.toLowerCase();
        final value = match.group(2) ?? match.group(3) ?? '';
        attrs[key] = value;
      }
      return attrs;
    }

    String decodeXml(String str) {
      return str
          .replaceAll('&amp;', '&')
          .replaceAll('&lt;', '<')
          .replaceAll('&gt;', '>')
          .replaceAll('&quot;', '"')
          .replaceAll('&apos;', "'")
          .replaceAll('&#39;', "'");
    }

    final pMatches = pRegex.allMatches(rawText);
    for (final pMatch in pMatches) {
      final pAttrs = parseAttributes(pMatch.group(1) ?? '');
      final pContent = pMatch.group(2) ?? '';

      final lineBegin = _parseTimestamp(pAttrs['begin']) ?? 0.0;
      final dur = _parseTimestamp(pAttrs['dur']);
      final lineEnd = _parseTimestamp(pAttrs['end']) ?? (dur != null ? lineBegin + dur : -1.0);

      final role = (pAttrs['ttm:role'] ?? pAttrs['role'] ?? '').toLowerCase();
      final isRoman = role.contains('roman');
      final isBg = role.contains('bg') || role.contains('background');

      final spans = spanRegex.allMatches(pContent).toList();
      final words = <WordSpan>[];

      for (int s = 0; s < spans.length; s++) {
        final spanMatch = spans[s];
        final spanAttrs = parseAttributes(spanMatch.group(1) ?? '');
        var spanRawText = spanMatch.group(2) ?? '';

        // Check for ruby transliteration: <rt>roman</rt>
        String? inlineRoman;
        final rtMatch = RegExp(r'<rt\b[^>]*>([\s\S]*?)<\/rt>', caseSensitive: false).firstMatch(spanRawText);
        if (rtMatch != null) {
          inlineRoman = decodeXml(rtMatch.group(1) ?? '').trim();
          spanRawText = spanRawText.replaceAll(RegExp(r'<rt\b[^>]*>[\s\S]*?<\/rt>', caseSensitive: false), '');
        }

        final spanText = decodeXml(spanRawText.replaceAll(RegExp(r'<[^>]+>'), ''));
        if (spanText.isEmpty && s + 1 == spans.length) continue;

        final spanBegin = _parseTimestamp(spanAttrs['begin']) ?? lineBegin;
        final spanDur = _parseTimestamp(spanAttrs['dur']);
        var spanEnd = _parseTimestamp(spanAttrs['end']) ?? (spanDur != null ? spanBegin + spanDur : null);

        if (spanEnd == null || spanEnd <= spanBegin) {
          if (s + 1 < spans.length) {
            final nextAttrs = parseAttributes(spans[s + 1].group(1) ?? '');
            spanEnd = _parseTimestamp(nextAttrs['begin']) ?? (spanBegin + 0.5);
          } else if (lineEnd > spanBegin) {
            spanEnd = lineEnd;
          } else {
            spanEnd = spanBegin + 0.6;
          }
        }

        inlineRoman ??= spanAttrs['roman'] ?? spanAttrs['ttm:roman'];

        words.add(WordSpan(
          text: spanText,
          startSec: spanBegin,
          endSec: spanEnd,
          roman: inlineRoman,
        ));
      }

      final rawCleanText = decodeXml(pContent.replaceAll(RegExp(r'<[^>]+>'), '')).trim();

      // If this <p> is an aligned romanization line, pair it with the preceding line
      if (isRoman && parsed.isNotEmpty) {
        final lastLine = parsed.last;
        if ((lastLine.timeSec - lineBegin).abs() <= 0.8) {
          if (words.isNotEmpty && lastLine.words.isNotEmpty && words.length == lastLine.words.length) {
            final updatedWords = <WordSpan>[];
            for (int w = 0; w < lastLine.words.length; w++) {
              updatedWords.add(WordSpan(
                text: lastLine.words[w].text,
                startSec: lastLine.words[w].startSec,
                endSec: lastLine.words[w].endSec,
                roman: words[w].text.trim(),
              ));
            }
            parsed[parsed.length - 1] = LyricLine(
              timeSec: lastLine.timeSec,
              endSec: lastLine.endSec,
              text: lastLine.text,
              romanText: rawCleanText,
              words: updatedWords,
              isInstrumental: lastLine.isInstrumental,
              isEstimated: lastLine.isEstimated,
              isBackgroundVocal: lastLine.isBackgroundVocal,
            );
            continue;
          } else {
            parsed[parsed.length - 1] = LyricLine(
              timeSec: lastLine.timeSec,
              endSec: lastLine.endSec,
              text: lastLine.text,
              romanText: rawCleanText,
              words: lastLine.words,
              isInstrumental: lastLine.isInstrumental,
              isEstimated: lastLine.isEstimated,
              isBackgroundVocal: lastLine.isBackgroundVocal,
            );
            continue;
          }
        }
      }

      final fullText = words.isNotEmpty
          ? words.map((w) => w.text).join('')
          : rawCleanText;

      parsed.add(LyricLine(
        timeSec: lineBegin,
        endSec: lineEnd,
        text: fullText.isNotEmpty ? fullText : 'Instrumental Interlude',
        words: words,
        isInstrumental: fullText.isEmpty ||
            fullText.toLowerCase().contains('instrumental') ||
            fullText.toLowerCase().contains('solo'),
        isEstimated: false,
        isBackgroundVocal: isBg,
      ));
    }

    parsed.sort((a, b) => a.timeSec.compareTo(b.timeSec));
    return parsed;
  }

  /// Parses Enhanced LRC (.elrc / .lrc) with inline syllable markers `<mm:ss.xx>word`
  /// and companion `[romaji]` sub-lines.
  List<LyricLine> parseEnhancedLrcText(String rawText) {
    if (rawText.trim().isEmpty) return const [];

    final rawLines = rawText.split(RegExp(r'\r?\n'));
    final parsed = <LyricLine>[];

    final lineTimeRegex = RegExp(r'^\[(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?\]');
    final wordTagRegex = RegExp(r'<(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?>');

    double parseTag(String minStr, String secStr, String? fracStr) {
      final min = int.tryParse(minStr) ?? 0;
      final sec = int.tryParse(secStr) ?? 0;
      double fracSec = 0.0;
      if (fracStr != null) {
        final fracInt = int.tryParse(fracStr) ?? 0;
        fracSec = fracStr.length == 3 ? fracInt / 1000.0 : fracInt / 100.0;
      }
      return (min * 60.0) + sec + fracSec;
    }

    for (int i = 0; i < rawLines.length; i++) {
      final line = rawLines[i].trim();
      if (line.isEmpty) continue;

      if (line.startsWith('[ar:') ||
          line.startsWith('[ti:') ||
          line.startsWith('[al:') ||
          line.startsWith('[by:') ||
          line.startsWith('[offset:') ||
          line.startsWith('[length:')) {
        continue;
      }

      final isRomajiLine = line.toLowerCase().contains('[romaji]') ||
          line.toLowerCase().contains('(romaji)');

      final lineMatch = lineTimeRegex.firstMatch(line);
      double lineStartSec = -1.0;
      String remainder = line;

      if (lineMatch != null) {
        lineStartSec = parseTag(lineMatch.group(1)!, lineMatch.group(2)!, lineMatch.group(3));
        remainder = line.substring(lineMatch.end).trim();
      }

      // Handle companion romaji lines
      if (isRomajiLine) {
        remainder = remainder
            .replaceAll(RegExp(r'\[romaji\]|\(romaji\)', caseSensitive: false), '')
            .trim();

        if (parsed.isNotEmpty &&
            (lineStartSec < 0 || (parsed.last.timeSec - lineStartSec).abs() <= 0.6)) {
          final cleanRomaji = remainder.replaceAll(wordTagRegex, '').trim();
          final romanMatches = wordTagRegex.allMatches(remainder).toList();
          if (romanMatches.isNotEmpty && parsed.last.words.isNotEmpty) {
            final romanWordTexts = <String>[];
            for (int m = 0; m < romanMatches.length; m++) {
              final startIdx = romanMatches[m].end;
              final endIdx = (m + 1 < romanMatches.length)
                  ? romanMatches[m + 1].start
                  : remainder.length;
              final text = remainder.substring(startIdx, endIdx).trim();
              if (text.isEmpty && m + 1 == romanMatches.length) continue;
              romanWordTexts.add(text);
            }
            if (romanWordTexts.length == parsed.last.words.length) {
              final updatedWords = <WordSpan>[];
              for (int w = 0; w < parsed.last.words.length; w++) {
                updatedWords.add(WordSpan(
                  text: parsed.last.words[w].text,
                  startSec: parsed.last.words[w].startSec,
                  endSec: parsed.last.words[w].endSec,
                  roman: romanWordTexts[w],
                ));
              }
              parsed[parsed.length - 1] = LyricLine(
                timeSec: parsed.last.timeSec,
                endSec: parsed.last.endSec,
                text: parsed.last.text,
                romanText: cleanRomaji,
                words: updatedWords,
                isInstrumental: parsed.last.isInstrumental,
                isEstimated: parsed.last.isEstimated,
                isBackgroundVocal: parsed.last.isBackgroundVocal,
              );
              continue;
            }
          }

          parsed[parsed.length - 1] = LyricLine(
            timeSec: parsed.last.timeSec,
            endSec: parsed.last.endSec,
            text: parsed.last.text,
            romanText: cleanRomaji,
            words: parsed.last.words,
            isInstrumental: parsed.last.isInstrumental,
            isEstimated: parsed.last.isEstimated,
            isBackgroundVocal: parsed.last.isBackgroundVocal,
          );
          continue;
        }
      }

      // Parse inline syllable tags <mm:ss.xx>
      final wordMatches = wordTagRegex.allMatches(remainder).toList();
      if (wordMatches.isNotEmpty) {
        final words = <WordSpan>[];
        for (int m = 0; m < wordMatches.length; m++) {
          final match = wordMatches[m];
          final startSec = parseTag(match.group(1)!, match.group(2)!, match.group(3));
          final textStart = match.end;
          final textEnd = (m + 1 < wordMatches.length)
              ? wordMatches[m + 1].start
              : remainder.length;

          var rawWordText = remainder.substring(textStart, textEnd);
          if (rawWordText.isEmpty && m + 1 == wordMatches.length) {
            continue;
          }

          double endSec;
          if (m + 1 < wordMatches.length) {
            endSec = parseTag(
              wordMatches[m + 1].group(1)!,
              wordMatches[m + 1].group(2)!,
              wordMatches[m + 1].group(3),
            );
          } else {
            endSec = startSec + 0.8;
          }

          String? inlineRoman;
          final parenMatch = RegExp(r'^([^\(\[（]+)[\(\[（]([a-zA-Z\s]+)[\)\]）]$').firstMatch(rawWordText.trim());
          if (parenMatch != null) {
            rawWordText = parenMatch.group(1) ?? rawWordText;
            inlineRoman = parenMatch.group(2);
          }

          words.add(WordSpan(
            text: rawWordText,
            startSec: startSec,
            endSec: endSec,
            roman: inlineRoman,
          ));
        }

        if (words.isNotEmpty) {
          if (lineStartSec < 0) {
            lineStartSec = words.first.startSec;
          }
          final lineEndSec = words.last.endSec;
          final fullText = words.map((w) => w.text).join('').trim();
          parsed.add(LyricLine(
            timeSec: lineStartSec,
            endSec: lineEndSec,
            text: fullText.isNotEmpty ? fullText : 'Instrumental Interlude',
            words: words,
            isInstrumental: fullText.isEmpty ||
                fullText.toLowerCase().contains('instrumental') ||
                fullText.toLowerCase().contains('solo'),
            isEstimated: false,
          ));
          continue;
        }
      }

      final cleanText = remainder.replaceAll(wordTagRegex, '').trim();
      if (lineStartSec >= 0 || cleanText.isNotEmpty) {
        parsed.add(LyricLine(
          timeSec: lineStartSec,
          text: cleanText.isNotEmpty ? cleanText : 'Instrumental Interlude',
          isInstrumental: cleanText.isEmpty ||
              cleanText.toLowerCase().contains('instrumental') ||
              cleanText.toLowerCase().contains('solo'),
          isEstimated: false,
        ));
      }
    }

    parsed.sort((a, b) => a.timeSec.compareTo(b.timeSec));
    return parsed;
  }

  /// Parses NetEase YRC (.yrc) format lyrics:
  /// `[line_start_ms, line_dur_ms](word_start_ms, word_dur_ms, flag)word...`
  List<LyricLine> parseYrcText(String rawText) {
    if (rawText.trim().isEmpty) return const [];

    final parsed = <LyricLine>[];
    final rawLines = rawText.split(RegExp(r'\r?\n'));
    final lineHeaderRegex = RegExp(r'^\[(\d+),(\d+)\](.*)$');
    final wordRegex = RegExp(r'\((\d+),(\d+)(?:,\d+)?\)([^\(\[\r\n]+)');

    for (final line in rawLines) {
      final trimmed = line.trim();
      if (trimmed.isEmpty) continue;

      final headerMatch = lineHeaderRegex.firstMatch(trimmed);
      if (headerMatch != null) {
        final lineStartSec = int.parse(headerMatch.group(1)!) / 1000.0;
        final lineDurSec = int.parse(headerMatch.group(2)!) / 1000.0;
        final lineEndSec = lineStartSec + lineDurSec;
        final remainder = headerMatch.group(3) ?? '';

        final wordMatches = wordRegex.allMatches(remainder);
        final words = <WordSpan>[];

        for (final w in wordMatches) {
          final wStartSec = int.parse(w.group(1)!) / 1000.0;
          final wDurSec = int.parse(w.group(2)!) / 1000.0;
          final wEndSec = wStartSec + wDurSec;
          final wText = w.group(3) ?? '';

          words.add(WordSpan(
            text: wText,
            startSec: wStartSec,
            endSec: wEndSec,
          ));
        }

        final text = words.isNotEmpty
            ? words.map((w) => w.text).join('')
            : remainder.replaceAll(wordRegex, '').trim();

        parsed.add(LyricLine(
          timeSec: lineStartSec,
          endSec: lineEndSec,
          text: text.isNotEmpty ? text : 'Instrumental Interlude',
          words: words,
          isInstrumental: text.isEmpty || text.toLowerCase().contains('instrumental'),
          isEstimated: false,
        ));
      }
    }

    parsed.sort((a, b) => a.timeSec.compareTo(b.timeSec));
    return parsed;
  }

  /// Robust timestamp parser supporting formats:
  /// `00:12.34`, `01:23.456`, `01:23:45.678`, `12.345s`, `12345ms`
  static double? _parseTimestamp(String? raw) {
    if (raw == null) return null;
    final trimmed = raw.trim();
    if (trimmed.isEmpty) return null;

    if (trimmed.endsWith('ms')) {
      final ms = double.tryParse(trimmed.substring(0, trimmed.length - 2));
      if (ms != null) return ms / 1000.0;
    }
    if (trimmed.endsWith('s')) {
      final s = double.tryParse(trimmed.substring(0, trimmed.length - 1));
      if (s != null) return s;
    }

    final parts = trimmed.split(':');
    if (parts.length == 2) {
      final min = double.tryParse(parts[0]) ?? 0.0;
      final sec = double.tryParse(parts[1]) ?? 0.0;
      return (min * 60.0) + sec;
    } else if (parts.length == 3) {
      final hr = double.tryParse(parts[0]) ?? 0.0;
      final min = double.tryParse(parts[1]) ?? 0.0;
      final sec = double.tryParse(parts[2]) ?? 0.0;
      return (hr * 3600.0) + (min * 60.0) + sec;
    }

    return double.tryParse(trimmed);
  }

  // ---------------------------------------------------------------------------
  // 4. SMART SOURCE PRIORITY RESOLVER (Waterfall Architecture)
  // ---------------------------------------------------------------------------
  /// Determines the preferred default [LyricSourceType] for [track]:
  /// 1. Syllable Sync (.ttml, .elrc, .yrc) if present and syllable-timed -> .syllableSync
  /// 2. If no syllable file, checks .lrc vs embedded tags:
  ///    - If no embedded lyrics but has .lrc -> .lrcFile
  ///    - If only embedded and no .lrc -> .embedded
  ///    - If both are present:
  ///      * If .lrc is timed and embedded is NOT timed -> .lrcFile
  ///      * If embedded is timed and .lrc is NOT timed -> .embedded
  ///      * If both are timed -> .embedded
  ///      * If neither is timed -> .embedded
  /// 3. If neither exists -> .embedded (clean default empty view)
  Future<LyricSourceType> determinePreferredSource(TrackItem track) async {
    // 1. Highest priority: Syllable-synced lyrics (TTML, ELRC, YRC)
    final syllableResult = await loadLyrics(track, LyricSourceType.syllableSync);
    if (syllableResult.hasLyrics && syllableResult.lines.any((l) => l.hasSyllableSync)) {
      return LyricSourceType.syllableSync;
    }

    // 2. Line-level LRC vs Embedded priority matrix
    final embeddedResult = await loadLyrics(track, LyricSourceType.embedded);
    final lrcResult = await loadLyrics(track, LyricSourceType.lrcFile);

    final hasEmbedded = embeddedResult.hasLyrics;
    final hasLrc = lrcResult.hasLyrics;

    if (hasLrc && !hasEmbedded) {
      return LyricSourceType.lrcFile;
    }
    if (hasEmbedded && !hasLrc) {
      return LyricSourceType.embedded;
    }
    if (hasEmbedded && hasLrc) {
      final embeddedTimed = embeddedResult.isTimed;
      final lrcTimed = lrcResult.isTimed;

      if (lrcTimed && !embeddedTimed) {
        return LyricSourceType.lrcFile;
      }
      if (embeddedTimed && !lrcTimed) {
        return LyricSourceType.embedded;
      }
      return LyricSourceType.embedded;
    }

    return LyricSourceType.embedded;
  }

  // ---------------------------------------------------------------------------
  // 5. PARSER FOR LRC AND PLAIN TEXT
  // ---------------------------------------------------------------------------
  /// Parses raw LRC or text into a list of [LyricLine] sorted by timestamp.
  /// If no timestamps are present, lines are treated as static untimed text (timeSec: -1.0)
  /// with zero artificial pacing or fake auto-scrolling animations.
  List<LyricLine> parseLrcText(String rawText, {Duration? duration}) {
    if (rawText.trim().isEmpty) return const [];

    final rawLines = rawText.split(RegExp(r'\r?\n'));
    final parsed = <LyricLine>[];
    final unformattedLines = <String>[];

    // Standard LRC timestamp regex: [mm:ss.xx] or [mm:ss.xxx] or [mm:ss]
    final timeRegex = RegExp(r'\[(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?\]');

    for (final line in rawLines) {
      final trimmed = line.trim();
      if (trimmed.isEmpty) continue;

      // Ignore ID3 / LRC metadata tags like [ar:Artist], [ti:Title], [al:Album]
      if (trimmed.startsWith('[') &&
          (trimmed.startsWith('[ar:') ||
              trimmed.startsWith('[ti:') ||
              trimmed.startsWith('[al:') ||
              trimmed.startsWith('[by:') ||
              trimmed.startsWith('[offset:') ||
              trimmed.startsWith('[length:'))) {
        continue;
      }

      final matches = timeRegex.allMatches(trimmed);
      if (matches.isNotEmpty) {
        // Line has one or more timestamps
        final cleanText = trimmed.replaceAll(timeRegex, '').trim();
        for (final m in matches) {
          final min = int.tryParse(m.group(1) ?? '0') ?? 0;
          final sec = int.tryParse(m.group(2) ?? '0') ?? 0;
          final rawFrac = m.group(3);
          double fracSec = 0.0;
          if (rawFrac != null) {
            final fracInt = int.tryParse(rawFrac) ?? 0;
            fracSec = rawFrac.length == 3 ? fracInt / 1000.0 : fracInt / 100.0;
          }

          final timeSec = (min * 60.0) + sec + fracSec;
          final isInstrumental = cleanText.isEmpty ||
              cleanText.toLowerCase().contains('instrumental') ||
              cleanText.toLowerCase().contains('solo');

          parsed.add(LyricLine(
            timeSec: timeSec,
            text: cleanText.isNotEmpty ? cleanText : 'Instrumental Interlude',
            isInstrumental: isInstrumental,
            isEstimated: false,
          ));
        }
      } else {
        // Plain unsynchronized text
        unformattedLines.add(trimmed);
      }
    }

    if (parsed.isNotEmpty) {
      parsed.sort((a, b) => a.timeSec.compareTo(b.timeSec));
      return parsed;
    }

    // If file only had unsynchronized plain text (e.g. 01 Rabbit Hole.lrc)
    // Keep exact lines without generating synthetic timestamps (timeSec = -1.0)
    // so the UI treats it strictly as a static readable text list without scrolling jitter.
    if (unformattedLines.isNotEmpty) {
      for (final line in unformattedLines) {
        parsed.add(LyricLine(
          timeSec: -1.0,
          text: line,
          isInstrumental: false,
          isEstimated: false,
        ));
      }
    }

    return parsed;
  }
}

/// Local Forced Alignment Configuration & Hardware Safety Gate.
/// Guards against thermal throttling and audio dropouts by enforcing core limits
/// and Real-Time Factor (RTF) budgets before running local CTC / whisper DTW alignment.
class AlignmentCircuitBreaker {
  /// Minimum CPU physical core count required before permitting local CTC/DTW alignment
  static const int minCoresRequired = 4;

  /// Maximum Real-Time Factor (RTF) threshold. If local processing takes > 0.35x real audio duration,
  /// the circuit breaker trips to prevent thermal throttling and audio dropouts.
  static const double maxRtfThreshold = 0.35;

  /// Checks if device hardware qualifies for background acoustic forced alignment
  static bool canRunLocalAlignment() {
    return Platform.numberOfProcessors >= minCoresRequired;
  }
}

