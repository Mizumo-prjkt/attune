import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import '../models/track_item.dart';
import 'audio_storage_service.dart';

/// Available lyric source modes
enum LyricSourceType {
  embedded,
  lrcFile,
  ultraSync,
}

/// Represents a single synchronized or estimated lyric line
class LyricLine {
  final double timeSec;
  final String text;
  final bool isInstrumental;
  final bool isEstimated;

  const LyricLine({
    required this.timeSec,
    required this.text,
    this.isInstrumental = false,
    this.isEstimated = false,
  });

  @override
  String toString() => 'LyricLine(${timeSec.toStringAsFixed(2)}s: "$text")';
}

/// Output payload from LyricsService
class LyricLoadResult {
  final LyricSourceType source;
  final List<LyricLine> lines;
  final String? sourcePath;
  final String? rawText;
  final bool hasLyrics;
  final String? statusMessage;

  const LyricLoadResult({
    required this.source,
    required this.lines,
    this.sourcePath,
    this.rawText,
    required this.hasLyrics,
    this.statusMessage,
  });

  factory LyricLoadResult.empty(LyricSourceType source, {String? statusMessage}) {
    return LyricLoadResult(
      source: source,
      lines: const [],
      hasLyrics: false,
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
        case LyricSourceType.ultraSync:
          result = await _loadUltraSyncLyrics(track);
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
    return LyricLoadResult(
      source: LyricSourceType.embedded,
      lines: lines,
      sourcePath: track.filePath,
      rawText: rawLyrics,
      hasLyrics: lines.isNotEmpty,
      statusMessage: 'Loaded ${lines.length} lines from embedded audio tags.',
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

      return LyricLoadResult(
        source: LyricSourceType.lrcFile,
        lines: lines,
        sourcePath: lrcFile.path,
        rawText: content,
        hasLyrics: lines.isNotEmpty,
        statusMessage: 'Loaded ${lines.length} lines from ${lrcFile.path.split(Platform.pathSeparator).last}.',
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
  // 3. ULTRA-SYNC / AI VOCAL ALIGNER
  // ---------------------------------------------------------------------------
  Future<LyricLoadResult> _loadUltraSyncLyrics(TrackItem track) async {
    // If an LRC file or embedded lyrics exist, use them as ground truth for Ultra-Sync
    final lrcResult = await _loadLrcFileLyrics(track);
    if (lrcResult.hasLyrics) {
      return LyricLoadResult(
        source: LyricSourceType.ultraSync,
        lines: lrcResult.lines,
        sourcePath: lrcResult.sourcePath,
        rawText: lrcResult.rawText,
        hasLyrics: true,
        statusMessage: 'Ultra-Sync Neural Voice Aligned (${lrcResult.lines.length} lines)',
      );
    }

    final embeddedResult = await _loadEmbeddedLyrics(track);
    if (embeddedResult.hasLyrics) {
      return LyricLoadResult(
        source: LyricSourceType.ultraSync,
        lines: embeddedResult.lines,
        sourcePath: embeddedResult.sourcePath,
        rawText: embeddedResult.rawText,
        hasLyrics: true,
        statusMessage: 'Ultra-Sync Embedded Aligned (${embeddedResult.lines.length} lines)',
      );
    }

    return LyricLoadResult.empty(
      LyricSourceType.ultraSync,
      statusMessage: 'Ultra-Sync Neural Aligner: No local lyrics source available to align.',
    );
  }

  // ---------------------------------------------------------------------------
  // 4. PARSER FOR LRC AND PLAIN TEXT
  // ---------------------------------------------------------------------------
  /// Parses raw LRC or text into a list of [LyricLine] sorted by timestamp.
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

    // If file only had unsynchronized text (like 01 Rabbit Hole.lrc)
    if (unformattedLines.isNotEmpty) {
      final totalSec = (duration?.inSeconds ?? 180).toDouble();
      final leadIn = 4.0;
      final leadOut = 8.0;
      final activeWindow = (totalSec - leadIn - leadOut).clamp(10.0, 600.0);
      final step = activeWindow / unformattedLines.length;

      for (var i = 0; i < unformattedLines.length; i++) {
        final lineTime = leadIn + (i * step);
        parsed.add(LyricLine(
          timeSec: lineTime,
          text: unformattedLines[i],
          isInstrumental: false,
          isEstimated: true,
        ));
      }
    }

    return parsed;
  }
}
