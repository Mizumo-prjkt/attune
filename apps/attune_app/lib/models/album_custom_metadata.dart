/// Structured custom metadata for albums (Year, Genre, Splash Art, Lore/Description)
class AlbumCustomMetadata {
  final String? year;
  final String? genre;
  final String? customSplashArt;
  final String? description;

  const AlbumCustomMetadata({
    this.year,
    this.genre,
    this.customSplashArt,
    this.description,
  });

  AlbumCustomMetadata copyWith({
    String? year,
    String? genre,
    String? customSplashArt,
    String? description,
  }) {
    return AlbumCustomMetadata(
      year: year ?? this.year,
      genre: genre ?? this.genre,
      customSplashArt: customSplashArt ?? this.customSplashArt,
      description: description ?? this.description,
    );
  }

  Map<String, dynamic> toJson() => {
    if (year != null && year!.isNotEmpty) 'year': year,
    if (genre != null && genre!.isNotEmpty) 'genre': genre,
    if (customSplashArt != null && customSplashArt!.isNotEmpty) 'customSplashArt': customSplashArt,
    if (description != null && description!.isNotEmpty) 'description': description,
  };

  factory AlbumCustomMetadata.fromJson(dynamic json) {
    if (json is String) {
      return AlbumCustomMetadata(description: json);
    }
    if (json is Map) {
      return AlbumCustomMetadata(
        year: json['year']?.toString(),
        genre: json['genre']?.toString(),
        customSplashArt: json['customSplashArt']?.toString(),
        description: json['description']?.toString(),
      );
    }
    return const AlbumCustomMetadata();
  }
}
