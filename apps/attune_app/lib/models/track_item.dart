import 'package:flutter/material.dart';

/// Data model representing a playable music track in Attune
class TrackItem {
  final String id;
  final String title;
  final String artist;
  final String album;
  final String filePath;
  final Duration duration;
  final double bpm;
  final String musicalKey;
  final String format;
  final String genre;
  final int sampleRate;
  final int bitDepth;
  final bool isLossless;
  final bool isProprietary;
  final Color seedColor;
  final String? artworkAsset;
  final String? artworkFile;
  final Map<String, dynamic>? tapProfile;

  const TrackItem({
    required this.id,
    required this.title,
    required this.artist,
    required this.album,
    required this.filePath,
    required this.duration,
    required this.bpm,
    required this.musicalKey,
    this.format = 'FLAC',
    this.genre = 'Pop',
    this.sampleRate = 44100,
    this.bitDepth = 16,
    this.isLossless = true,
    this.isProprietary = false,
    this.seedColor = const Color(0xFF6366F1),
    this.artworkAsset,
    this.artworkFile,
    this.tapProfile,
  });

  TrackItem copyWith({
    String? id,
    String? title,
    String? artist,
    String? album,
    String? filePath,
    Duration? duration,
    double? bpm,
    String? musicalKey,
    String? format,
    String? genre,
    int? sampleRate,
    int? bitDepth,
    bool? isLossless,
    bool? isProprietary,
    Color? seedColor,
    String? artworkAsset,
    String? artworkFile,
    Map<String, dynamic>? tapProfile,
  }) {
    return TrackItem(
      id: id ?? this.id,
      title: title ?? this.title,
      artist: artist ?? this.artist,
      album: album ?? this.album,
      filePath: filePath ?? this.filePath,
      duration: duration ?? this.duration,
      bpm: bpm ?? this.bpm,
      musicalKey: musicalKey ?? this.musicalKey,
      format: format ?? this.format,
      genre: genre ?? this.genre,
      sampleRate: sampleRate ?? this.sampleRate,
      bitDepth: bitDepth ?? this.bitDepth,
      isLossless: isLossless ?? this.isLossless,
      isProprietary: isProprietary ?? this.isProprietary,
      seedColor: seedColor ?? this.seedColor,
      artworkAsset: artworkAsset ?? this.artworkAsset,
      artworkFile: artworkFile ?? this.artworkFile,
      tapProfile: tapProfile ?? this.tapProfile,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'title': title,
      'artist': artist,
      'album': album,
      'filePath': filePath,
      'durationMs': duration.inMilliseconds,
      'bpm': bpm,
      'musicalKey': musicalKey,
      'format': format,
      'genre': genre,
      'sampleRate': sampleRate,
      'bitDepth': bitDepth,
      'isLossless': isLossless,
      'isProprietary': isProprietary,
      'seedColorValue': seedColor.toARGB32(),
      'artworkAsset': artworkAsset,
      'artworkFile': artworkFile,
      'tapProfile': tapProfile,
    };
  }

  factory TrackItem.fromJson(Map<String, dynamic> json) {
    return TrackItem(
      id: json['id'] as String? ?? '',
      title: json['title'] as String? ?? 'Unknown Title',
      artist: json['artist'] as String? ?? 'Unknown Artist',
      album: json['album'] as String? ?? 'Unknown Album',
      filePath: json['filePath'] as String? ?? '',
      duration: Duration(milliseconds: (json['durationMs'] as num? ?? 0).toInt()),
      bpm: (json['bpm'] as num? ?? 0.0).toDouble(),
      musicalKey: json['musicalKey'] as String? ?? 'Pending Profile',
      format: json['format'] as String? ?? 'Audio',
      genre: json['genre'] as String? ?? 'General',
      sampleRate: (json['sampleRate'] as num? ?? 44100).toInt(),
      bitDepth: (json['bitDepth'] as num? ?? 16).toInt(),
      isLossless: json['isLossless'] as bool? ?? true,
      isProprietary: json['isProprietary'] as bool? ?? false,
      seedColor: Color((json['seedColorValue'] as num? ?? 0xFF6366F1).toInt()),
      artworkAsset: json['artworkAsset'] as String?,
      artworkFile: json['artworkFile'] as String?,
      tapProfile: json['tapProfile'] as Map<String, dynamic>?,
    );
  }
}

