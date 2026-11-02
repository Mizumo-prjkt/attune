import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/services.dart' show rootBundle;

/// State of the GitHub release update check.
enum UpdateStatus {
  idle,
  checking,
  upToDate,
  updateAvailable,
  noReleasesFound,
  error,
}

/// Downloadable build artifact asset associated with a GitHub release.
class ReleaseAsset {
  final String name;
  final String downloadUrl;
  final int sizeBytes;
  final String contentType;
  final bool isDebug;
  final String platformCategory; // 'linux', 'windows', 'android', 'other'

  const ReleaseAsset({
    required this.name,
    required this.downloadUrl,
    required this.sizeBytes,
    required this.contentType,
    required this.isDebug,
    required this.platformCategory,
  });

  factory ReleaseAsset.fromJson(Map<String, dynamic> json) {
    final name = json['name'] as String? ?? '';
    final url = json['browser_download_url'] as String? ?? '';
    final size = json['size'] as int? ?? 0;
    final type = json['content_type'] as String? ?? 'application/octet-stream';

    final lower = name.toLowerCase();
    final isDbg = lower.contains('debug');

    String category = 'other';
    if (lower.contains('appimage') || lower.contains('linux') || lower.endsWith('.tar.gz')) {
      category = 'linux';
    } else if (lower.contains('setup') || lower.contains('windows') || lower.endsWith('.exe') || (lower.endsWith('.zip') && lower.contains('win'))) {
      category = 'windows';
    } else if (lower.endsWith('.apk') || lower.contains('android')) {
      category = 'android';
    }

    return ReleaseAsset(
      name: name,
      downloadUrl: url,
      sizeBytes: size,
      contentType: type,
      isDebug: isDbg,
      platformCategory: category,
    );
  }

  String get formattedSize {
    if (sizeBytes <= 0) return '';
    if (sizeBytes >= 1024 * 1024) {
      return '${(sizeBytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(sizeBytes / 1024).toStringAsFixed(1)} KB';
  }
}

/// Detailed metadata of a release returned from GitHub Releases API.
class UpdateInfo {
  final String tagName;
  final String title;
  final String releaseNotes;
  final String htmlUrl;
  final DateTime? publishedAt;
  final bool isPrerelease;
  final bool isNewer;
  final List<ReleaseAsset> assets;

  const UpdateInfo({
    required this.tagName,
    required this.title,
    required this.releaseNotes,
    required this.htmlUrl,
    this.publishedAt,
    required this.isPrerelease,
    required this.isNewer,
    required this.assets,
  });

  factory UpdateInfo.fromJson(Map<String, dynamic> json, {required bool isNewer}) {
    final rawAssets = json['assets'] as List<dynamic>? ?? [];
    final parsedAssets = rawAssets
        .whereType<Map<String, dynamic>>()
        .map((a) => ReleaseAsset.fromJson(a))
        .toList();

    DateTime? published;
    final pubStr = json['published_at'] as String?;
    if (pubStr != null) {
      try {
        published = DateTime.parse(pubStr);
      } catch (_) {}
    }

    return UpdateInfo(
      tagName: json['tag_name'] as String? ?? 'v0.0.0',
      title: json['name'] as String? ?? json['tag_name'] as String? ?? 'Attune Release',
      releaseNotes: json['body'] as String? ?? '',
      htmlUrl: json['html_url'] as String? ?? '',
      publishedAt: published,
      isPrerelease: json['prerelease'] as bool? ?? false,
      isNewer: isNewer,
      assets: parsedAssets,
    );
  }

  /// Returns release assets suitable for user's host platform.
  List<ReleaseAsset> get hostPlatformAssets {
    String currentPlatform = 'other';
    if (Platform.isLinux) {
      currentPlatform = 'linux';
    } else if (Platform.isWindows) {
      currentPlatform = 'windows';
    } else if (Platform.isAndroid) {
      currentPlatform = 'android';
    }

    final platformMatches = assets.where((a) => a.platformCategory == currentPlatform).toList();
    return platformMatches.isNotEmpty ? platformMatches : assets;
  }
}

/// Result envelope for update checks.
class UpdateCheckResult {
  final UpdateStatus status;
  final UpdateInfo? updateInfo;
  final String? message;

  const UpdateCheckResult({
    required this.status,
    this.updateInfo,
    this.message,
  });
}

/// In-App Update Service for fetching latest releases from GitHub.
/// Allows checking release & debug builds so users and contributors
/// can easily obtain artifacts and inspect updates.
class UpdateService {
  UpdateService._();
  static final UpdateService instance = UpdateService._();

  static const String defaultApiUrl =
      'https://api.github.com/repos/Mizumo-prjkt/attune/releases/latest';

  UpdateStatus _status = UpdateStatus.idle;
  UpdateInfo? _latestUpdate;
  String? _lastError;
  DateTime? _lastCheckedTime;

  UpdateStatus get status => _status;
  UpdateInfo? get latestUpdate => _latestUpdate;
  String? get lastError => _lastError;
  DateTime? get lastCheckedTime => _lastCheckedTime;

  /// Compares semantic version [current] against [remote] according to SemVer 2.0.0
  /// and natural token precedence (supporting pre-releases like -rc3, -rc4, -beta, -alpha).
  /// Returns negative if current < remote (remote is newer),
  /// 0 if equal, and positive if current > remote.
  static int compareSemanticVersions(String current, String remote) {
    final vCurrent = _ParsedVersion.parse(current);
    final vRemote = _ParsedVersion.parse(remote);
    return vCurrent.compareTo(vRemote);
  }

  /// Resolves the current application version dynamically from assets/build_info.json.
  /// Returns null if the build information is missing or cannot be loaded.
  static Future<String?> resolveCurrentVersion() async {
    try {
      final jsonString = await rootBundle.loadString('assets/build_info.json');
      final data = json.decode(jsonString);
      if (data is Map<String, dynamic>) {
        final fullVersion = data['full_version'] as String?;
        if (fullVersion != null && fullVersion.trim().isNotEmpty) {
          return fullVersion.trim();
        }
        final version = data['version'] as String?;
        final local = data['localversion'] as String? ?? data['suffix_version'] as String? ?? '';
        if (version != null && version.trim().isNotEmpty) {
          return '${version.trim()}$local'.trim();
        }
      }
    } catch (_) {}
    return null;
  }

  /// Queries GitHub Releases API and evaluates version differences.
  /// If [currentVersion] is not explicitly provided, it resolves dynamically
  /// from the app bundle's etched build_info.json metadata.
  Future<UpdateCheckResult> checkForUpdates({
    String? currentVersion,
    String? currentSuffix,
    String endpointUrl = defaultApiUrl,
    HttpClient? customHttpClient,
  }) async {
    _status = UpdateStatus.checking;
    _lastError = null;

    String? effectiveVersion;
    if (currentVersion != null && currentVersion.trim().isNotEmpty) {
      final base = currentVersion.trim();
      final suffix = currentSuffix?.trim() ?? '';
      if (suffix.isNotEmpty && !base.endsWith(suffix)) {
        effectiveVersion = '$base$suffix';
      } else {
        effectiveVersion = base;
      }
    } else {
      effectiveVersion = await resolveCurrentVersion();
    }

    if (effectiveVersion == null || effectiveVersion.isEmpty) {
      final err = 'Unable to determine current application version dynamically (missing build_info.json).';
      _status = UpdateStatus.error;
      _lastError = err;
      _lastCheckedTime = DateTime.now();
      return UpdateCheckResult(status: UpdateStatus.error, message: err);
    }

    final client = customHttpClient ?? HttpClient();
    client.connectionTimeout = const Duration(seconds: 8);

    try {
      final uri = Uri.parse(endpointUrl);
      final request = await client.getUrl(uri);
      request.headers.set('User-Agent', 'Attune-App/$effectiveVersion');
      request.headers.set('Accept', 'application/vnd.github.v3+json');

      final response = await request.close().timeout(const Duration(seconds: 8));

      if (response.statusCode == 404) {
        _status = UpdateStatus.noReleasesFound;
        _lastCheckedTime = DateTime.now();
        return const UpdateCheckResult(
          status: UpdateStatus.noReleasesFound,
          message: 'No published releases found on GitHub repository.',
        );
      }

      if (response.statusCode != 200) {
        final err = 'GitHub API error (HTTP ${response.statusCode})';
        _status = UpdateStatus.error;
        _lastError = err;
        _lastCheckedTime = DateTime.now();
        return UpdateCheckResult(status: UpdateStatus.error, message: err);
      }

      final bodyString = await response.transform(utf8.decoder).join();
      final decoded = json.decode(bodyString);

      if (decoded is! Map<String, dynamic>) {
        throw const FormatException('Invalid JSON payload from GitHub API');
      }

      final remoteTag = decoded['tag_name'] as String? ?? '';
      final cmp = compareSemanticVersions(effectiveVersion, remoteTag);
      final isNewer = cmp < 0;

      final info = UpdateInfo.fromJson(decoded, isNewer: isNewer);
      _latestUpdate = info;
      _lastCheckedTime = DateTime.now();

      if (isNewer) {
        _status = UpdateStatus.updateAvailable;
        return UpdateCheckResult(
          status: UpdateStatus.updateAvailable,
          updateInfo: info,
          message: 'Update available: ${info.tagName}',
        );
      } else {
        _status = UpdateStatus.upToDate;
        final msg = cmp > 0
            ? 'You are running a newer build ($effectiveVersion) than latest release (${info.tagName}).'
            : 'Attune is up to date (${info.tagName}).';
        return UpdateCheckResult(
          status: UpdateStatus.upToDate,
          updateInfo: info,
          message: msg,
        );
      }
    } catch (e) {
      final err = 'Failed to check for updates: $e';
      _status = UpdateStatus.error;
      _lastError = err;
      _lastCheckedTime = DateTime.now();
      return UpdateCheckResult(status: UpdateStatus.error, message: err);
    } finally {
      if (customHttpClient == null) {
        client.close();
      }
    }
  }

  /// Opens the given release or asset URL using native OS launcher.
  static Future<bool> openReleaseUrl(String url) async {
    if (url.isEmpty) return false;
    try {
      if (Platform.isLinux) {
        await Process.run('xdg-open', [url]);
        return true;
      } else if (Platform.isWindows) {
        await Process.run('cmd', ['/c', 'start', '', url]);
        return true;
      } else if (Platform.isMacOS) {
        await Process.run('open', [url]);
        return true;
      } else if (Platform.isAndroid) {
        await Process.run('am', ['start', '-a', 'android.intent.action.VIEW', '-d', url]);
        return true;
      }
    } catch (_) {}
    return false;
  }
}

/// Robust semantic version model adhering to SemVer 2.0.0 and natural tag ordering.
class _ParsedVersion implements Comparable<_ParsedVersion> {
  final int major;
  final int minor;
  final int patch;
  final String? prerelease;

  const _ParsedVersion({
    required this.major,
    required this.minor,
    required this.patch,
    this.prerelease,
  });

  static _ParsedVersion parse(String versionStr) {
    var raw = versionStr.trim();
    if (raw.startsWith('v') || raw.startsWith('V')) {
      raw = raw.substring(1).trim();
    }

    final plusIdx = raw.indexOf('+');
    if (plusIdx != -1) {
      raw = raw.substring(0, plusIdx);
    }

    String core;
    String? prerelease;
    final dashIdx = raw.indexOf('-');
    if (dashIdx != -1) {
      core = raw.substring(0, dashIdx);
      prerelease = raw.substring(dashIdx + 1).trim();
      if (prerelease.isEmpty) prerelease = null;
    } else {
      core = raw;
    }

    final coreParts = core.split('.');
    final major = coreParts.isNotEmpty ? (int.tryParse(coreParts[0]) ?? 0) : 0;
    final minor = coreParts.length > 1 ? (int.tryParse(coreParts[1]) ?? 0) : 0;
    final patch = coreParts.length > 2 ? (int.tryParse(coreParts[2]) ?? 0) : 0;

    return _ParsedVersion(
      major: major,
      minor: minor,
      patch: patch,
      prerelease: prerelease,
    );
  }

  @override
  int compareTo(_ParsedVersion other) {
    if (major != other.major) return major.compareTo(other.major);
    if (minor != other.minor) return minor.compareTo(other.minor);
    if (patch != other.patch) return patch.compareTo(other.patch);

    // SemVer 2.0.0 Clause 11: A normal version without a prerelease has higher
    // precedence than a version with a prerelease (e.g., 0.1.0-rc4 < 0.1.0).
    if (prerelease == null && other.prerelease == null) return 0;
    if (prerelease == null && other.prerelease != null) return 1;
    if (prerelease != null && other.prerelease == null) return -1;

    return _comparePrereleases(prerelease!, other.prerelease!);
  }

  static int _comparePrereleases(String preA, String preB) {
    final tokensA = _tokenizePrerelease(preA);
    final tokensB = _tokenizePrerelease(preB);

    final maxLen = tokensA.length > tokensB.length ? tokensA.length : tokensB.length;
    for (int i = 0; i < maxLen; i++) {
      if (i >= tokensA.length) return -1;
      if (i >= tokensB.length) return 1;

      final tA = tokensA[i];
      final tB = tokensB[i];

      final nA = int.tryParse(tA);
      final nB = int.tryParse(tB);

      if (nA != null && nB != null) {
        if (nA != nB) return nA.compareTo(nB);
      } else if (nA != null && nB == null) {
        return -1;
      } else if (nA == null && nB != null) {
        return 1;
      } else {
        final cmp = tA.toLowerCase().compareTo(tB.toLowerCase());
        if (cmp != 0) return cmp;
      }
    }
    return 0;
  }

  static List<String> _tokenizePrerelease(String pre) {
    final tokens = <String>[];
    final segments = pre.split(RegExp(r'[.\-_]'));
    for (final seg in segments) {
      if (seg.isEmpty) continue;
      final matches = RegExp(r'(\d+|\D+)').allMatches(seg);
      for (final m in matches) {
        final str = m.group(0);
        if (str != null && str.isNotEmpty) {
          tokens.add(str);
        }
      }
    }
    return tokens;
  }
}
