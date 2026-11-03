import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/music_metadata_service.dart';

class MockHttpClient implements HttpClient {
  final Future<HttpClientRequest> Function(Uri url)? onGetUrl;

  MockHttpClient({this.onGetUrl});

  @override
  Duration? connectionTimeout;

  @override
  Future<HttpClientRequest> getUrl(Uri url) {
    if (onGetUrl != null) {
      return onGetUrl!(url);
    }
    throw UnimplementedError();
  }

  @override
  void close({bool force = false}) {}

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class MockHttpClientRequest implements HttpClientRequest {
  @override
  final HttpHeaders headers = MockHttpHeaders();

  final Future<HttpClientResponse> Function()? onClose;

  MockHttpClientRequest({this.onClose});

  @override
  Future<HttpClientResponse> close() {
    if (onClose != null) {
      return onClose!();
    }
    throw UnimplementedError();
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class MockHttpHeaders implements HttpHeaders {
  final Map<String, String> _headers = {};

  @override
  void set(String name, Object value, {bool preserveHeaderCase = false}) {
    _headers[name.toLowerCase()] = value.toString();
  }

  @override
  String? value(String name) => _headers[name.toLowerCase()];

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class MockHttpClientResponse extends Stream<List<int>> implements HttpClientResponse {
  @override
  final int statusCode;
  final String body;

  MockHttpClientResponse({required this.statusCode, required this.body});

  @override
  StreamSubscription<List<int>> listen(
    void Function(List<int> event)? onData, {
    Function? onError,
    void Function()? onDone,
    bool? cancelOnError,
  }) {
    return Stream.value(utf8.encode(body)).listen(
      onData,
      onError: onError,
      onDone: onDone,
      cancelOnError: cancelOnError,
    );
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  group('OnlineMusicMetadata & Artist Comma Splitting Tests', () {
    test('splitArtists separates comma-delimited artists correctly', () {
      final res1 = OnlineMusicMetadata.splitArtists('DECO*27, PinocchioP');
      expect(res1.primary, equals('DECO*27'));
      expect(res1.additional, equals(['PinocchioP']));

      final res2 = OnlineMusicMetadata.splitArtists('DECO*27, PinocchioP, OSTER project');
      expect(res2.primary, equals('DECO*27'));
      expect(res2.additional, equals(['PinocchioP', 'OSTER project']));
    });

    test('splitArtists handles featuring notation (feat. / ft.) and slashes', () {
      final res = OnlineMusicMetadata.splitArtists('EasyPop feat. GUMI, Megurine Luka');
      expect(res.primary, equals('EasyPop'));
      expect(res.additional, equals(['GUMI', 'Megurine Luka']));

      final resSlash = OnlineMusicMetadata.splitArtists('kz / livetune feat. Hatsune Miku');
      expect(resSlash.primary, equals('kz'));
      expect(resSlash.additional, equals(['livetune', 'Hatsune Miku']));
    });

    test('splitArtists handles edge cases gracefully (empty string, single artist)', () {
      final empty = OnlineMusicMetadata.splitArtists('');
      expect(empty.primary, equals('Unknown Artist'));
      expect(empty.additional, isEmpty);

      final single = OnlineMusicMetadata.splitArtists('ABBA');
      expect(single.primary, equals('ABBA'));
      expect(single.additional, isEmpty);
    });

    test('allArtists getter preserves primary and appends non-duplicated contributors', () {
      const meta = OnlineMusicMetadata(
        title: '(Not) A Devil',
        primaryArtist: 'DECO*27',
        additionalArtists: ['PinocchioP', 'DECO*27', 'Hatsune Miku'],
        sourceProvider: 'MusicBrainz',
      );
      expect(meta.allArtists, equals(['DECO*27', 'PinocchioP', 'Hatsune Miku']));
    });
  });

  group('MusicMetadataService Compliance & Fallback Tests', () {
    test('Declares RFC compliant User-Agent for MusicBrainz API', () {
      expect(
        MusicMetadataService.musicBrainzUserAgent,
        equals('Attune/0.1.0 ( https://github.com/Mizumo-prjkt/attune )'),
      );
      expect(MusicMetadataService.musicBrainzUserAgent, contains('Attune/'));
      expect(MusicMetadataService.musicBrainzUserAgent, contains('https://github.com/'));
    });

    test('MusicBrainz successful lookup parses album, label, catalog and artists', () async {
      final service = MusicMetadataService.instance;
      String? sentUserAgent;

      final mockClient = MockHttpClient(
        onGetUrl: (uri) async {
          late MockHttpClientRequest request;
          request = MockHttpClientRequest(
            onClose: () async {
              sentUserAgent = request.headers.value('user-agent');
              final mbJsonResponse = jsonEncode({
                'releases': [
                  {
                    'id': '8f6b7c2a-9e12-4d2b-a012-123456789abc',
                    'title': 'Super Trouper',
                    'date': '1980-11-03',
                    'barcode': '4988002561234',
                    'artist-credit': [
                      {
                        'artist': {'name': 'ABBA'}
                      }
                    ],
                    'label-info': [
                      {
                        'label': {'name': 'Polar Music'},
                        'catalog-number': 'POLS-322'
                      }
                    ],
                    'release-group': {
                      'primary-type': 'Studio Album',
                    },
                    'text-representation': {
                      'language': 'eng',
                    }
                  }
                ]
              });
              return MockHttpClientResponse(statusCode: 200, body: mbJsonResponse);
            },
          );
          return request;
        },
      );

      final result = await service.fetchAlbumMetadata(
        albumName: 'Super Trouper',
        artistName: 'ABBA',
        customHttpClient: mockClient,
      );

      expect(sentUserAgent, equals(MusicMetadataService.musicBrainzUserAgent));
      expect(result, isNotNull);
      expect(result!.title, equals('Super Trouper'));
      expect(result.primaryArtist, equals('ABBA'));
      expect(result.recordLabel, equals('Polar Music'));
      expect(result.catalogNumber, equals('POLS-322'));
      expect(result.releaseYear, equals('1980'));
      expect(result.releaseDate, equals('1980-11-03'));
      expect(result.albumType, equals('Studio Album'));
      expect(result.language, equals('English'));
      expect(result.musicBrainzId, equals('8f6b7c2a-9e12-4d2b-a012-123456789abc'));
      expect(result.sourceProvider, equals('MusicBrainz'));
    });

    test('MusicBrainz 429 Rate-Limit triggers transparent fallback to ReccoBeats API', () async {
      final service = MusicMetadataService.instance;
      int requestCount = 0;

      final mockClient = MockHttpClient(
        onGetUrl: (uri) async {
          requestCount++;
          final isMusicBrainz = uri.host.contains('musicbrainz.org');
          return MockHttpClientRequest(
            onClose: () async {
              if (isMusicBrainz) {
                // Simulate 429 Too Many Requests
                return MockHttpClientResponse(statusCode: 429, body: 'Rate limit exceeded');
              } else {
                // ReccoBeats API response
                final rbResponse = jsonEncode({
                  'content': [
                    {
                      'id': 'rb_album_9988',
                      'title': '(Not) A Devil',
                      'artist': 'DECO*27, PinocchioP',
                      'release_date': '2023-01-13',
                      'label': 'OTONARI',
                      'album_type': 'Single',
                      'image_url': 'https://reccobeats.com/covers/not_a_devil.jpg',
                    }
                  ]
                });
                return MockHttpClientResponse(statusCode: 200, body: rbResponse);
              }
            },
          );
        },
      );

      final result = await service.fetchAlbumMetadata(
        albumName: '(Not) A Devil',
        artistName: 'DECO*27',
        customHttpClient: mockClient,
      );

      expect(requestCount, equals(2)); // Tried MB -> Failed 429 -> Succeeded on ReccoBeats
      expect(result, isNotNull);
      expect(result!.sourceProvider, equals('ReccoBeats'));
      expect(result.title, equals('(Not) A Devil'));
      expect(result.primaryArtist, equals('DECO*27'));
      expect(result.additionalArtists, contains('PinocchioP'));
      expect(result.recordLabel, equals('OTONARI'));
      expect(result.releaseYear, equals('2023'));
      expect(result.coverArtUrl, equals('https://reccobeats.com/covers/not_a_devil.jpg'));
    });
  });
}
