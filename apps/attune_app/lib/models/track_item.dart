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

  /// Formatted duration string as `M:SS` (or `H:MM:SS` if 1 hour or longer).
  String get formattedDuration {
    final hours = duration.inHours;
    final minutes = duration.inMinutes % 60;
    final seconds = (duration.inSeconds % 60).toString().padLeft(2, '0');
    if (hours > 0) {
      return '$hours:${minutes.toString().padLeft(2, '0')}:$seconds';
    }
    return '${duration.inMinutes}:$seconds';
  }

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

  /// Extracts lightweight summary from TAP profile, removing heavy frame-by-frame cue_zones
  /// to guarantee the in-memory library and disk cache remain feather-light (< 150 KB instead of 20+ MB).
  static Map<String, dynamic> extractCompactProfile(Map<String, dynamic> tap) {
    return {
      'schema_version': tap['schema_version'] ?? 1,
      if (tap['track_id'] != null) 'track_id': tap['track_id'],
      if (tap['global_summary'] != null) 'global_summary': tap['global_summary'],
      if (tap['analysis_tier'] != null) 'analysis_tier': tap['analysis_tier'],
    };
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
      'tapProfile': tapProfile != null ? extractCompactProfile(tapProfile!) : null,
    };
  }

  factory TrackItem.fromJson(Map<String, dynamic> json) {
    final rawTap = json['tapProfile'] as Map<String, dynamic>?;
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
      tapProfile: rawTap != null ? extractCompactProfile(rawTap) : null,
    );
  }

  /// Whether the track has a recognized or profiled BPM.
  bool get hasBpm => bpm > 0.0 || (tapProfile?['global_summary']?['nominal_bpm'] as num? ?? 0.0) > 0.0;

  /// Effective BPM resolving metadata BPM first, then falling back to computed TAP profile nominal BPM.
  double get effectiveBpm {
    if (bpm > 0.0) return bpm;
    final nominal = tapProfile?['global_summary']?['nominal_bpm'] as num?;
    if (nominal != null && nominal > 0.0) return nominal.toDouble();
    return 0.0;
  }

  /// Formatted BPM string (e.g. "128.0 BPM" or "-- BPM" if not yet profiled/probed).
  String get formattedBpm => hasBpm ? '${effectiveBpm.toStringAsFixed(1)} BPM' : '-- BPM';
}

