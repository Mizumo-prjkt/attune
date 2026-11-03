/// Structured custom metadata for albums (Year, Genre, Splash Art, Lore/Description,
/// Additional Artists, and Online Query / Catalog Identifiers)
class AlbumCustomMetadata {
  final String? year;
  final String? genre;
  final String? customSplashArt;
  final String? description;

  // Multi-artist support
  final List<String> additionalArtists;

  // Online metadata query and catalog preparation
  final String? recordLabel;
  final String? catalogNumber;
  final String? albumType; // e.g. Studio Album, EP, Single, Compilation, Soundtrack
  final String? releaseDate;
  final String? language;
  final String? musicBrainzId;
  final String? onlineQueryUrl;

  const AlbumCustomMetadata({
    this.year,
    this.genre,
    this.customSplashArt,
    this.description,
    this.additionalArtists = const [],
    this.recordLabel,
    this.catalogNumber,
    this.albumType,
    this.releaseDate,
    this.language,
    this.musicBrainzId,
    this.onlineQueryUrl,
  });

  AlbumCustomMetadata copyWith({
    String? year,
    String? genre,
    String? customSplashArt,
    String? description,
    List<String>? additionalArtists,
    String? recordLabel,
    String? catalogNumber,
    String? albumType,
    String? releaseDate,
    String? language,
    String? musicBrainzId,
    String? onlineQueryUrl,
  }) {
    return AlbumCustomMetadata(
      year: year ?? this.year,
      genre: genre ?? this.genre,
      customSplashArt: customSplashArt ?? this.customSplashArt,
      description: description ?? this.description,
      additionalArtists: additionalArtists ?? this.additionalArtists,
      recordLabel: recordLabel ?? this.recordLabel,
      catalogNumber: catalogNumber ?? this.catalogNumber,
      albumType: albumType ?? this.albumType,
      releaseDate: releaseDate ?? this.releaseDate,
      language: language ?? this.language,
      musicBrainzId: musicBrainzId ?? this.musicBrainzId,
      onlineQueryUrl: onlineQueryUrl ?? this.onlineQueryUrl,
    );
  }

  Map<String, dynamic> toJson() => {
    if (year != null && year!.isNotEmpty) 'year': year,
    if (genre != null && genre!.isNotEmpty) 'genre': genre,
    if (customSplashArt != null && customSplashArt!.isNotEmpty) 'customSplashArt': customSplashArt,
    if (description != null && description!.isNotEmpty) 'description': description,
    if (additionalArtists.isNotEmpty) 'additionalArtists': additionalArtists,
    if (recordLabel != null && recordLabel!.isNotEmpty) 'recordLabel': recordLabel,
    if (catalogNumber != null && catalogNumber!.isNotEmpty) 'catalogNumber': catalogNumber,
    if (albumType != null && albumType!.isNotEmpty) 'albumType': albumType,
    if (releaseDate != null && releaseDate!.isNotEmpty) 'releaseDate': releaseDate,
    if (language != null && language!.isNotEmpty) 'language': language,
    if (musicBrainzId != null && musicBrainzId!.isNotEmpty) 'musicBrainzId': musicBrainzId,
    if (onlineQueryUrl != null && onlineQueryUrl!.isNotEmpty) 'onlineQueryUrl': onlineQueryUrl,
  };

  factory AlbumCustomMetadata.fromJson(dynamic json) {
    if (json is String) {
      return AlbumCustomMetadata(description: json);
    }
    if (json is Map) {
      List<String> parsedArtists = [];
      if (json['additionalArtists'] is List) {
        parsedArtists = (json['additionalArtists'] as List)
            .map((e) => e.toString().trim())
            .where((e) => e.isNotEmpty)
            .toList();
      }

      return AlbumCustomMetadata(
        year: json['year']?.toString(),
        genre: json['genre']?.toString(),
        customSplashArt: json['customSplashArt']?.toString(),
        description: json['description']?.toString(),
        additionalArtists: parsedArtists,
        recordLabel: json['recordLabel']?.toString(),
        catalogNumber: json['catalogNumber']?.toString(),
        albumType: json['albumType']?.toString(),
        releaseDate: json['releaseDate']?.toString(),
        language: json['language']?.toString(),
        musicBrainzId: json['musicBrainzId']?.toString(),
        onlineQueryUrl: json['onlineQueryUrl']?.toString(),
      );
    }
    return const AlbumCustomMetadata();
  }
}
