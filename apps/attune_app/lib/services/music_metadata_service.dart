import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';

/// Structured online music metadata returned by external providers (MusicBrainz, ReccoBeats).
class OnlineMusicMetadata {
  final String title;
  final String primaryArtist;
  final List<String> additionalArtists;
  final String? album;
  final String? releaseYear;
  final String? releaseDate;
  final String? genre;
  final String? recordLabel;
  final String? catalogNumber;
  final String? albumType;
  final String? language;
  final String? musicBrainzId;
  final String? coverArtUrl;
  final String? onlineQueryUrl;
  final String sourceProvider; // 'MusicBrainz', 'ReccoBeats', 'Enriched'
  final double? bpm;
  final String? musicalKey;
  final double? energy;
  final double? danceability;
  final String? description;

  const OnlineMusicMetadata({
    required this.title,
    required this.primaryArtist,
    this.additionalArtists = const [],
    this.album,
    this.releaseYear,
    this.releaseDate,
    this.genre,
    this.recordLabel,
    this.catalogNumber,
    this.albumType,
    this.language,
    this.musicBrainzId,
    this.coverArtUrl,
    this.onlineQueryUrl,
    required this.sourceProvider,
    this.bpm,
    this.musicalKey,
    this.energy,
    this.danceability,
    this.description,
  });

  /// All participating artists, beginning with primary artist
  List<String> get allArtists {
    final list = <String>[primaryArtist];
    for (final extra in additionalArtists) {
      if (!list.contains(extra)) {
        list.add(extra);
      }
    }
    return list;
  }

  /// Helper to parse comma-separated artists into a primary artist and additional artists list
  static ({String primary, List<String> additional}) splitArtists(String raw) {
    if (raw.trim().isEmpty) {
      return (primary: 'Unknown Artist', additional: <String>[]);
    }
    final parts = raw
        .split(RegExp(r'[,/;&]|\bfeat(?:\.|\b)|\bft(?:\.|\b)', caseSensitive: false))
        .map((s) => s.replaceAll(RegExp(r'^[.\s]+|[.\s]+$'), '').trim())
        .where((s) => s.isNotEmpty)
        .toList();

    if (parts.isEmpty) {
      return (primary: raw.trim(), additional: <String>[]);
    }
    final primary = parts.first;
    final additional = parts.skip(1).toList();
    return (primary: primary, additional: additional);
  }
}

/// Service that coordinates music metadata lookups against MusicBrainz and ReccoBeats.
/// Enforces MusicBrainz RFC-compliant User-Agent and client-side rate limits, with automatic
/// seamless fallback to ReccoBeats API in case of blocking, 429 rate-limiting, or missing records.
class MusicMetadataService {
  MusicMetadataService._();
  static final MusicMetadataService instance = MusicMetadataService._();

  /// MusicBrainz API compliance User-Agent string: ApplicationName/Version ( URL )
  static const String musicBrainzUserAgent = 'Attune/0.1.0 ( https://github.com/Mizumo-prjkt/attune )';
  static const String reccobeatsBaseUrl = 'https://api.reccobeats.com/v1';

  DateTime? _lastMusicBrainzRequestTime;

  /// Queries online metadata for an album using MusicBrainz first, with transparent fallback to ReccoBeats.
  Future<OnlineMusicMetadata?> fetchAlbumMetadata({
    required String albumName,
    String? artistName,
    bool forceReccoBeats = false,
    HttpClient? customHttpClient,
  }) async {
    final client = customHttpClient ?? HttpClient();
    client.connectionTimeout = const Duration(seconds: 7);

    try {
      if (!forceReccoBeats) {
        final mbResult = await _queryMusicBrainz(
          albumName: albumName,
          artistName: artistName,
          client: client,
        );
        if (mbResult != null) {
          debugPrint('[MusicMetadataService] Successfully fetched album metadata from MusicBrainz: ${mbResult.title}');
          return mbResult;
        }
        debugPrint('[MusicMetadataService] MusicBrainz returned no match or was unavailable. Falling back to ReccoBeats API...');
      }

      // Fallback: ReccoBeats API
      final rbResult = await _queryReccoBeats(
        albumName: albumName,
        artistName: artistName,
        client: client,
      );
      if (rbResult != null) {
        debugPrint('[MusicMetadataService] Successfully fetched album metadata from ReccoBeats: ${rbResult.title}');
        return rbResult;
      }
    } catch (e) {
      debugPrint('[MusicMetadataService] Error fetching online metadata: $e');
    } finally {
      if (customHttpClient == null) {
        client.close();
      }
    }
    return null;
  }

  /// Queries MusicBrainz Web Service v2 with compliant headers and 1 req/sec rate throttling.
  Future<OnlineMusicMetadata?> _queryMusicBrainz({
    required String albumName,
    String? artistName,
    required HttpClient client,
  }) async {
    // MusicBrainz rate-limit adherence: Ensure minimum 1000ms between calls
    if (_lastMusicBrainzRequestTime != null) {
      final elapsed = DateTime.now().difference(_lastMusicBrainzRequestTime!);
      if (elapsed < const Duration(milliseconds: 1050)) {
        await Future.delayed(const Duration(milliseconds: 1050) - elapsed);
      }
    }
    _lastMusicBrainzRequestTime = DateTime.now();

    final cleanAlbum = albumName.replaceAll(RegExp(r'\(\d{4}\)'), '').trim();
    String query = 'release:"$cleanAlbum"';
    if (artistName != null && artistName.trim().isNotEmpty && artistName != 'Unknown Artist') {
      final cleanArtist = artistName.split(',').first.trim();
      query += ' AND artist:"$cleanArtist"';
    }

    final uri = Uri.parse(
      'https://musicbrainz.org/ws/2/release/?query=${Uri.encodeComponent(query)}&fmt=json&limit=5',
    );

    try {
      final request = await client.getUrl(uri);
      request.headers.set('User-Agent', musicBrainzUserAgent);
      request.headers.set('Accept', 'application/json');

      final response = await request.close().timeout(const Duration(seconds: 8));

      if (response.statusCode == 429 || response.statusCode == 503) {
        debugPrint('[MusicMetadataService] MusicBrainz returned HTTP ${response.statusCode} (Rate limited / Service Unavailable).');
        return null;
      }

      if (response.statusCode != 200) {
        debugPrint('[MusicMetadataService] MusicBrainz returned HTTP ${response.statusCode}');
        return null;
      }

      final body = await response.transform(utf8.decoder).join();
      final data = jsonDecode(body);
      if (data is! Map || data['releases'] is! List) return null;

      final releases = data['releases'] as List;
      if (releases.isEmpty) return null;

      // Extract best matching release
      final best = releases.first as Map;
      final mbid = best['id']?.toString() ?? '';
      final title = best['title']?.toString() ?? cleanAlbum;

      // Artists parsing with comma awareness
      String primaryArtist = 'Unknown Artist';
      final additionalArtists = <String>[];

      if (best['artist-credit'] is List) {
        final credits = best['artist-credit'] as List;
        for (int i = 0; i < credits.length; i++) {
          final c = credits[i];
          if (c is Map && c['artist'] is Map) {
            final aName = c['artist']['name']?.toString() ?? '';
            // If the name itself has commas, split
            final split = OnlineMusicMetadata.splitArtists(aName);
            if (i == 0) {
              primaryArtist = split.primary;
              additionalArtists.addAll(split.additional);
            } else {
              if (!additionalArtists.contains(split.primary)) additionalArtists.add(split.primary);
              additionalArtists.addAll(split.additional);
            }
          }
        }
      }

      // Date / Year
      final dateStr = best['date']?.toString();
      final yearStr = (dateStr != null && dateStr.length >= 4) ? dateStr.substring(0, 4) : null;

      // Record label & catalog number
      String? recordLabel;
      String? catalogNumber;
      if (best['label-info'] is List && (best['label-info'] as List).isNotEmpty) {
        final labelInfo = (best['label-info'] as List).first as Map;
        if (labelInfo['label'] is Map) {
          recordLabel = labelInfo['label']['name']?.toString();
        }
        catalogNumber = labelInfo['catalog-number']?.toString();
      }

      // Barcode fallback for catalog number
      if (catalogNumber == null || catalogNumber.isEmpty) {
        catalogNumber = best['barcode']?.toString();
      }

      // Release type
      String? albumType = 'Studio Album';
      if (best['release-group'] is Map) {
        final rg = best['release-group'] as Map;
        final pType = rg['primary-type']?.toString();
        if (pType != null && pType.isNotEmpty) {
          albumType = pType;
        }
      }

      // Language
      String? language;
      if (best['text-representation'] is Map) {
        final tr = best['text-representation'] as Map;
        final langCode = tr['language']?.toString();
        if (langCode != null) {
          language = _mapLanguageCode(langCode);
        }
      }

      // Check cover art archive availability in MusicBrainz metadata
      bool hasFrontArt = false;
      if (best['cover-art-archive'] is Map) {
        final caa = best['cover-art-archive'] as Map;
        final front = caa['front'];
        final artwork = caa['artwork'];
        final count = caa['count'] as num? ?? 0;
        if (front == true || (artwork == true && count > 0)) {
          hasFrontArt = true;
        }
      }

      String? verifiedCoverArtUrl;
      if (hasFrontArt && mbid.isNotEmpty) {
        final candidateUrl = 'https://coverartarchive.org/release/$mbid/front';
        verifiedCoverArtUrl = await _verifyImageUri(candidateUrl, client);
      }

      final onlineQueryUrl = mbid.isNotEmpty ? 'https://musicbrainz.org/release/$mbid' : null;

      // Dynamic factual description
      final desc = 'Official release "$title" by $primaryArtist'
          '${additionalArtists.isNotEmpty ? " (feat. ${additionalArtists.join(", ")})" : ""}. '
          '${recordLabel != null ? "Published by $recordLabel. " : ""}'
          '${dateStr != null ? "Catalog release date: $dateStr. " : ""}'
          'Indexed in the MusicBrainz Open Catalog (MBID: $mbid).';

      return OnlineMusicMetadata(
        title: title,
        primaryArtist: primaryArtist,
        additionalArtists: additionalArtists,
        album: title,
        releaseYear: yearStr,
        releaseDate: dateStr,
        genre: null,
        recordLabel: recordLabel,
        catalogNumber: catalogNumber,
        albumType: albumType,
        language: language,
        musicBrainzId: mbid,
        coverArtUrl: verifiedCoverArtUrl,
        onlineQueryUrl: onlineQueryUrl,
        sourceProvider: 'MusicBrainz',
        description: desc,
      );
    } catch (e) {
      debugPrint('[MusicMetadataService] MusicBrainz query error: $e');
      return null;
    }
  }

  /// Queries ReccoBeats Free Music Metadata & Audio Features API (`https://api.reccobeats.com/v1`).
  /// Operates as an independent fallback provider when MusicBrainz is rate-limited or blocked.
  Future<OnlineMusicMetadata?> _queryReccoBeats({
    required String albumName,
    String? artistName,
    required HttpClient client,
  }) async {
    final cleanAlbum = albumName.replaceAll(RegExp(r'\(\d{4}\)'), '').trim();
    final searchUri = Uri.parse(
      '$reccobeatsBaseUrl/album/search?searchText=${Uri.encodeComponent(cleanAlbum)}&page=1&size=5',
    );

    try {
      final request = await client.getUrl(searchUri);
      request.headers.set('User-Agent', 'Attune/0.1.0');
      request.headers.set('Accept', 'application/json');

      final response = await request.close().timeout(const Duration(seconds: 8));

      if (response.statusCode != 200) {
        debugPrint('[MusicMetadataService] ReccoBeats album search returned HTTP ${response.statusCode}');
        return null;
      }

      final body = await response.transform(utf8.decoder).join();
      final data = jsonDecode(body);

      // ReccoBeats returns list or paged content
      List items = [];
      if (data is List) {
        items = data;
      } else if (data is Map && data['content'] is List) {
        items = data['content'] as List;
      } else if (data is Map && data['items'] is List) {
        items = data['items'] as List;
      }

      if (items.isEmpty) {
        // Try track search as alternative
        return await _queryReccoBeatsTrack(query: cleanAlbum, client: client);
      }

      final album = items.first as Map;
      final albumId = album['id']?.toString() ?? '';
      final title = album['name']?.toString() ?? album['title']?.toString() ?? cleanAlbum;

      // Extract artists and consider commas as separation
      String rawArtist = '';
      if (album['artist'] is String) {
        rawArtist = album['artist'] as String;
      } else if (album['artists'] is List) {
        final artistList = album['artists'] as List;
        rawArtist = artistList.map((a) => a is Map ? (a['name'] ?? '') : a.toString()).join(', ');
      } else if (artistName != null) {
        rawArtist = artistName;
      }

      final split = OnlineMusicMetadata.splitArtists(rawArtist);
      final primaryArtist = split.primary;
      final additionalArtists = split.additional;

      final releaseDate = album['release_date']?.toString() ?? album['releaseDate']?.toString();
      final yearStr = (releaseDate != null && releaseDate.length >= 4) ? releaseDate.substring(0, 4) : null;
      final rawCoverUrl = album['image_url']?.toString() ?? album['cover_url']?.toString() ?? album['imageUrl']?.toString();
      final albumType = album['album_type']?.toString() ?? 'Album';
      final label = album['label']?.toString();

      String? verifiedCoverUrl;
      if (rawCoverUrl != null && rawCoverUrl.isNotEmpty) {
        verifiedCoverUrl = await _verifyImageUri(rawCoverUrl, client);
      }

      final desc = 'Album "$title" by $primaryArtist'
          '${additionalArtists.isNotEmpty ? " (feat. ${additionalArtists.join(", ")})" : ""}. '
          '${label != null ? "Released via $label. " : ""}'
          'Data retrieved from ReccoBeats Open Music Catalog.';

      return OnlineMusicMetadata(
        title: title,
        primaryArtist: primaryArtist,
        additionalArtists: additionalArtists,
        album: title,
        releaseYear: yearStr,
        releaseDate: releaseDate,
        genre: null,
        recordLabel: label,
        catalogNumber: null,
        albumType: albumType,
        language: null,
        musicBrainzId: null,
        coverArtUrl: verifiedCoverUrl,
        onlineQueryUrl: albumId.isNotEmpty ? 'https://reccobeats.com/album/$albumId' : null,
        sourceProvider: 'ReccoBeats',
        description: desc,
      );
    } catch (e) {
      debugPrint('[MusicMetadataService] ReccoBeats album search error: $e');
      return null;
    }
  }

  /// Alternative query to ReccoBeats track search when direct album search yields no match.
  Future<OnlineMusicMetadata?> _queryReccoBeatsTrack({
    required String query,
    required HttpClient client,
  }) async {
    try {
      final trackUri = Uri.parse('$reccobeatsBaseUrl/track/search?query=${Uri.encodeComponent(query)}&page=1&size=3');
      final request = await client.getUrl(trackUri);
      request.headers.set('User-Agent', 'Attune/0.1.0');
      request.headers.set('Accept', 'application/json');

      final response = await request.close().timeout(const Duration(seconds: 8));
      if (response.statusCode != 200) return null;

      final body = await response.transform(utf8.decoder).join();
      final data = jsonDecode(body);
      List tracks = [];
      if (data is List) {
        tracks = data;
      } else if (data is Map && data['content'] is List) {
        tracks = data['content'] as List;
      }

      if (tracks.isEmpty) return null;

      final track = tracks.first as Map;
      final title = track['name']?.toString() ?? query;
      final albumName = track['album_name']?.toString() ?? title;

      String rawArtist = '';
      if (track['artists'] is List) {
        rawArtist = (track['artists'] as List).map((a) => a is Map ? a['name'] : a.toString()).join(', ');
      } else {
        rawArtist = track['artist']?.toString() ?? 'Various Artists';
      }

      final split = OnlineMusicMetadata.splitArtists(rawArtist);

      String? verifiedTrackCover;
      final rawTrackCover = track['image_url']?.toString();
      if (rawTrackCover != null && rawTrackCover.isNotEmpty) {
        verifiedTrackCover = await _verifyImageUri(rawTrackCover, client);
      }

      return OnlineMusicMetadata(
        title: title,
        primaryArtist: split.primary,
        additionalArtists: split.additional,
        album: albumName,
        releaseYear: null,
        releaseDate: null,
        genre: null,
        recordLabel: null,
        catalogNumber: null,
        albumType: 'Single / Track',
        language: null,
        musicBrainzId: null,
        coverArtUrl: verifiedTrackCover,
        onlineQueryUrl: 'https://reccobeats.com',
        sourceProvider: 'ReccoBeats',
      );
    } catch (_) {
      return null;
    }
  }

  /// Probes an image URI (HTTP/HTTPS) and verifies it returns HTTP 200 with an image/* content type.
  /// If it returns HTTP 404, 500, non-200, HTML, or any non-image content type, it is rejected and returns null.
  Future<String?> _verifyImageUri(String uriString, HttpClient client) async {
    try {
      final uri = Uri.tryParse(uriString);
      if (uri == null || (!uri.isScheme('http') && !uri.isScheme('https'))) {
        return null;
      }
      final request = await client.getUrl(uri);
      request.headers.set('User-Agent', musicBrainzUserAgent);
      request.followRedirects = true;
      request.maxRedirects = 4;

      final response = await request.close().timeout(const Duration(seconds: 4));
      if (response.statusCode != 200) {
        debugPrint('[MusicMetadataService] Cover art URI rejected: HTTP ${response.statusCode} for $uriString');
        return null;
      }

      final contentType = response.headers.contentType?.mimeType.toLowerCase();
      if (contentType == null || !contentType.startsWith('image/')) {
        debugPrint('[MusicMetadataService] Cover art URI rejected: Non-image Content-Type "$contentType" for $uriString');
        return null;
      }

      return uriString;
    } catch (e) {
      debugPrint('[MusicMetadataService] Cover art URI verification failed for $uriString: $e');
      return null;
    }
  }

  /// Validates whether a file path or URL points to a genuine, existing image.
  /// For local files: checks existence, non-empty size, and image file extension (.jpg, .jpeg, .png, .webp, .gif, .bmp).
  /// For HTTP/HTTPS URLs: verifies HTTP 200 and image/* Content-Type.
  static Future<bool> isImageUriValid(String uriString, {HttpClient? customHttpClient}) async {
    final trimmed = uriString.trim();
    if (trimmed.isEmpty) return false;

    final uri = Uri.tryParse(trimmed);
    if (uri != null && (uri.isScheme('http') || uri.isScheme('https'))) {
      final client = customHttpClient ?? HttpClient();
      client.connectionTimeout = const Duration(seconds: 4);
      try {
        final request = await client.getUrl(uri);
        request.headers.set('User-Agent', musicBrainzUserAgent);
        request.followRedirects = true;
        request.maxRedirects = 4;
        final response = await request.close().timeout(const Duration(seconds: 4));
        if (response.statusCode != 200) return false;
        final cType = response.headers.contentType?.mimeType.toLowerCase();
        return cType != null && cType.startsWith('image/');
      } catch (_) {
        return false;
      } finally {
        if (customHttpClient == null) client.close();
      }
    } else {
      final file = File(trimmed);
      if (!file.existsSync()) return false;
      final lower = file.path.toLowerCase();
      final validExtensions = ['.jpg', '.jpeg', '.png', '.webp', '.gif', '.bmp'];
      if (!validExtensions.any((ext) => lower.endsWith(ext))) return false;
      try {
        return file.lengthSync() > 0;
      } catch (_) {
        return false;
      }
    }
  }

  static String _mapLanguageCode(String code) {
    switch (code.toLowerCase()) {
      case 'jpn':
        return 'Japanese';
      case 'eng':
        return 'English';
      case 'zho':
      case 'chi':
        return 'Chinese';
      case 'kor':
        return 'Korean';
      case 'spa':
        return 'Spanish';
      case 'fra':
      case 'fre':
        return 'French';
      case 'deu':
      case 'ger':
        return 'German';
      case 'ita':
        return 'Italian';
      case 'rus':
        return 'Russian';
      case 'por':
        return 'Portuguese';
      default:
        return code.toUpperCase();
    }
  }
}
