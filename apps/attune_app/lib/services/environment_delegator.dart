import 'dart:io';

/// Platform & environment execution delegator.
/// Auto-manages paths, directories, and environmental defaults across:
/// - Android: Scoped storage, internal sandbox files/cache, fallback temp dirs
/// - Linux: XDG directories ($XDG_CONFIG_HOME, $XDG_CACHE_HOME, $XDG_MUSIC_DIR), $HOME, workspace relative roots
/// - Windows: %APPDATA%, %LOCALAPPDATA%, %USERPROFILE%, %MUSIC%, workspace relative roots
/// - macOS/iOS: Sandbox paths, ~/Library/Application Support, ~/Music
/// - Test: Hermetic temporary test directories preventing host side-effects
abstract class AppEnvironmentDelegator {
  const AppEnvironmentDelegator();

  static AppEnvironmentDelegator? _customInstance;

  /// Returns the active delegator for the current runtime environment.
  static AppEnvironmentDelegator get current {
    if (_customInstance != null) return _customInstance!;
    if (Platform.environment.containsKey('FLUTTER_TEST')) {
      return const TestEnvironmentDelegator();
    }
    if (Platform.isAndroid) return const AndroidEnvironmentDelegator();
    if (Platform.isWindows) return const WindowsEnvironmentDelegator();
    if (Platform.isMacOS) return const MacOsEnvironmentDelegator();
    if (Platform.isLinux) return const LinuxEnvironmentDelegator();
    return const LinuxEnvironmentDelegator();
  }

  /// Injects a custom delegator (useful for hermetic tests or specific environments).
  static set instance(AppEnvironmentDelegator? delegator) {
    _customInstance = delegator;
  }

  /// Resets custom instance back to auto-detection.
  static void reset() {
    _customInstance = null;
  }

  /// Primary application configuration and metadata directory (e.g. library cache, settings, equalizer profile).
  Directory get appDataDirectory;

  /// Ephemeral cache directory (e.g. temporary waveforms, rendered stem bridges, transcoded audio).
  Directory get cacheDirectory;

  /// Discovers and returns candidate music root directories for the host environment.
  List<String> getMusicCandidateRoots({List<String>? additionalRoots});

  /// Ensures current working directory is valid, recovering from unlinked/stale inodes on POSIX/Windows.
  void ensureValidWorkingDirectory();

  /// Resolves the default username for listener profile displays.
  String get defaultUsername;

  /// Human-readable platform or environment name.
  String get platformName;

  bool get isAndroid => false;
  bool get isLinux => false;
  bool get isWindows => false;
  bool get isMacOS => false;
  bool get isTest => false;
}

/// Helper resolving the first writable directory from an ordered list of candidate paths.
Directory _resolveFirstWritableDirectory(List<String> candidates, {String suffix = 'attune'}) {
  for (final candidate in candidates) {
    try {
      final dir = Directory(candidate);
      if (!dir.existsSync()) {
        dir.createSync(recursive: true);
      }
      final probe = File('${dir.path}/.probe_${DateTime.now().microsecondsSinceEpoch}');
      probe.writeAsStringSync('1');
      probe.deleteSync();
      return dir;
    } catch (_) {
      continue;
    }
  }
  final fallback = Directory('${Directory.systemTemp.path}/$suffix');
  try {
    if (!fallback.existsSync()) fallback.createSync(recursive: true);
  } catch (_) {}
  return fallback;
}

/// Discovers existing workspace 'music' folders relative to CWD or executable location.
void _addWorkspaceCandidateRoots(List<String> roots) {
  try {
    final cwd = Directory.current.path;
    for (final rel in ['music', '../music', '../../music', '../../../music']) {
      final abs = Directory('$cwd/$rel').path;
      if (!roots.contains(abs) && Directory(abs).existsSync()) {
        roots.add(abs);
      }
    }
  } catch (_) {}

  try {
    final exe = File(Platform.resolvedExecutable).parent.path;
    for (final rel in ['music', '../music', '../../music', '../../../music', '../../../../music']) {
      final abs = Directory('$exe/$rel').path;
      if (!roots.contains(abs) && Directory(abs).existsSync()) {
        roots.add(abs);
      }
    }
  } catch (_) {}
}

/// Android runtime environment delegator
class AndroidEnvironmentDelegator extends AppEnvironmentDelegator {
  const AndroidEnvironmentDelegator();

  @override
  String get platformName => 'Android';

  @override
  bool get isAndroid => true;

  @override
  Directory get appDataDirectory {
    final envDir = Platform.environment['ATTUNE_DATA_DIR'] ?? Platform.environment['ATTUNE_CONFIG_DIR'];
    final candidates = <String>[
      if (envDir != null && envDir.isNotEmpty) envDir,
      '/data/user/0/org.mizproject.attune/files',
      '/data/data/org.mizproject.attune/files',
      '/data/user/0/org.mizproject.attune/cache',
      '/data/data/org.mizproject.attune/cache',
      '/data/user/0/com.attune.app/files',
      '/data/data/com.attune.app/files',
      '${Directory.systemTemp.path}/attune',
    ];
    return _resolveFirstWritableDirectory(candidates, suffix: 'attune');
  }

  @override
  Directory get cacheDirectory {
    final envDir = Platform.environment['ATTUNE_CACHE_DIR'];
    final candidates = <String>[
      if (envDir != null && envDir.isNotEmpty) envDir,
      '/data/user/0/org.mizproject.attune/cache',
      '/data/data/org.mizproject.attune/cache',
      '/data/user/0/com.attune.app/cache',
      '/data/data/com.attune.app/cache',
      '${Directory.systemTemp.path}/attune_cache',
    ];
    return _resolveFirstWritableDirectory(candidates, suffix: 'attune_cache');
  }

  @override
  List<String> getMusicCandidateRoots({List<String>? additionalRoots}) {
    final roots = <String>[];
    if (additionalRoots != null) roots.addAll(additionalRoots);

    final envMusic = Platform.environment['ATTUNE_MUSIC_DIR'];
    if (envMusic != null && envMusic.isNotEmpty && !roots.contains(envMusic)) {
      roots.add(envMusic);
    }

    // Standard Android public media directories & app scoped storage candidates
    for (final dir in [
      '/storage/emulated/0/Music',
      '/sdcard/Music',
      '/storage/emulated/0/Download',
      '/storage/emulated/0/Android/media/org.mizproject.attune',
    ]) {
      if (!roots.contains(dir)) {
        roots.add(dir);
      }
    }
    return roots;
  }

  @override
  void ensureValidWorkingDirectory() {
    // Android sandboxes do not rely on POSIX cwd recovery
  }

  @override
  String get defaultUsername => 'Attune Listener';
}

/// Linux runtime environment delegator
class LinuxEnvironmentDelegator extends AppEnvironmentDelegator {
  const LinuxEnvironmentDelegator();

  @override
  String get platformName => 'Linux';

  @override
  bool get isLinux => true;

  @override
  Directory get appDataDirectory {
    final envDir = Platform.environment['ATTUNE_CONFIG_DIR'] ?? Platform.environment['ATTUNE_DATA_DIR'];
    final xdg = Platform.environment['XDG_CONFIG_HOME'];
    final home = Platform.environment['HOME'];

    final candidates = <String>[
      if (envDir != null && envDir.isNotEmpty) envDir,
      if (xdg != null && xdg.isNotEmpty) '$xdg/attune',
      if (home != null && home.isNotEmpty) '$home/.config/attune',
      '${Directory.systemTemp.path}/attune',
    ];
    return _resolveFirstWritableDirectory(candidates, suffix: 'attune');
  }

  @override
  Directory get cacheDirectory {
    final envDir = Platform.environment['ATTUNE_CACHE_DIR'];
    final xdgCache = Platform.environment['XDG_CACHE_HOME'];
    final home = Platform.environment['HOME'];

    final candidates = <String>[
      if (envDir != null && envDir.isNotEmpty) envDir,
      if (xdgCache != null && xdgCache.isNotEmpty) '$xdgCache/attune',
      if (home != null && home.isNotEmpty) '$home/.cache/attune',
      '${Directory.systemTemp.path}/attune_cache',
    ];
    return _resolveFirstWritableDirectory(candidates, suffix: 'attune_cache');
  }

  @override
  List<String> getMusicCandidateRoots({List<String>? additionalRoots}) {
    ensureValidWorkingDirectory();
    final roots = <String>[];
    if (additionalRoots != null) roots.addAll(additionalRoots);

    final envMusic = Platform.environment['ATTUNE_MUSIC_DIR'];
    if (envMusic != null && envMusic.isNotEmpty && !roots.contains(envMusic)) {
      roots.add(envMusic);
    }

    _addWorkspaceCandidateRoots(roots);

    final xdgMusic = Platform.environment['XDG_MUSIC_DIR'];
    if (xdgMusic != null && xdgMusic.isNotEmpty && !roots.contains(xdgMusic)) {
      roots.add(xdgMusic);
    }

    final home = Platform.environment['HOME'];
    if (home != null && home.isNotEmpty) {
      for (final sub in ['Music', 'Music/Soulseek Complete', 'Downloads', 'Audio']) {
        final path = '$home/$sub';
        if (!roots.contains(path)) roots.add(path);
      }
    }

    final user = Platform.environment['USER'] ?? Platform.environment['LOGNAME'];
    if (user != null && user.isNotEmpty) {
      final mediaMusic = '/media/$user/Music';
      if (!roots.contains(mediaMusic)) roots.add(mediaMusic);
    }

    return roots;
  }

  @override
  void ensureValidWorkingDirectory() {
    try {
      Directory.current;
    } catch (_) {
      try {
        final home = Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'];
        if (home != null && Directory(home).existsSync()) {
          Directory.current = home;
        } else {
          final exe = Platform.resolvedExecutable;
          final dir = File(exe).parent;
          if (dir.existsSync()) {
            Directory.current = dir.path;
          }
        }
      } catch (_) {}
    }
  }

  @override
  String get defaultUsername {
    final user = Platform.environment['USER'] ?? Platform.environment['LOGNAME'];
    if (user != null && user.trim().isNotEmpty) return user.trim();
    return 'Attune Listener';
  }
}

/// Windows runtime environment delegator
class WindowsEnvironmentDelegator extends AppEnvironmentDelegator {
  const WindowsEnvironmentDelegator();

  @override
  String get platformName => 'Windows';

  @override
  bool get isWindows => true;

  @override
  Directory get appDataDirectory {
    final envDir = Platform.environment['ATTUNE_CONFIG_DIR'] ?? Platform.environment['ATTUNE_DATA_DIR'];
    final appData = Platform.environment['APPDATA'];
    final userProfile = Platform.environment['USERPROFILE'];
    final localAppData = Platform.environment['LOCALAPPDATA'];

    final candidates = <String>[
      if (envDir != null && envDir.isNotEmpty) envDir,
      if (appData != null && appData.isNotEmpty) '$appData\\attune',
      if (userProfile != null && userProfile.isNotEmpty) '$userProfile\\.config\\attune',
      if (localAppData != null && localAppData.isNotEmpty) '$localAppData\\attune',
      '${Directory.systemTemp.path}\\attune',
    ];
    return _resolveFirstWritableDirectory(candidates, suffix: 'attune');
  }

  @override
  Directory get cacheDirectory {
    final envDir = Platform.environment['ATTUNE_CACHE_DIR'];
    final localAppData = Platform.environment['LOCALAPPDATA'];
    final userProfile = Platform.environment['USERPROFILE'];

    final candidates = <String>[
      if (envDir != null && envDir.isNotEmpty) envDir,
      if (localAppData != null && localAppData.isNotEmpty) '$localAppData\\attune\\cache',
      if (userProfile != null && userProfile.isNotEmpty) '$userProfile\\.cache\\attune',
      '${Directory.systemTemp.path}\\attune_cache',
    ];
    return _resolveFirstWritableDirectory(candidates, suffix: 'attune_cache');
  }

  @override
  List<String> getMusicCandidateRoots({List<String>? additionalRoots}) {
    ensureValidWorkingDirectory();
    final roots = <String>[];
    if (additionalRoots != null) roots.addAll(additionalRoots);

    final envMusic = Platform.environment['ATTUNE_MUSIC_DIR'] ?? Platform.environment['MUSIC'];
    if (envMusic != null && envMusic.isNotEmpty && !roots.contains(envMusic)) {
      roots.add(envMusic);
    }

    _addWorkspaceCandidateRoots(roots);

    final userProfile = Platform.environment['USERPROFILE'];
    if (userProfile != null && userProfile.isNotEmpty) {
      for (final sub in ['Music', 'Downloads']) {
        final path = '$userProfile\\$sub';
        if (!roots.contains(path)) roots.add(path);
      }
    }

    return roots;
  }

  @override
  void ensureValidWorkingDirectory() {
    try {
      Directory.current;
    } catch (_) {
      try {
        final userProfile = Platform.environment['USERPROFILE'];
        if (userProfile != null && Directory(userProfile).existsSync()) {
          Directory.current = userProfile;
        } else {
          final exe = Platform.resolvedExecutable;
          final dir = File(exe).parent;
          if (dir.existsSync()) {
            Directory.current = dir.path;
          }
        }
      } catch (_) {}
    }
  }

  @override
  String get defaultUsername {
    final user = Platform.environment['USERNAME'];
    if (user != null && user.trim().isNotEmpty) return user.trim();
    return 'Attune Listener';
  }
}

/// macOS runtime environment delegator
class MacOsEnvironmentDelegator extends AppEnvironmentDelegator {
  const MacOsEnvironmentDelegator();

  @override
  String get platformName => 'macOS';

  @override
  bool get isMacOS => true;

  @override
  Directory get appDataDirectory {
    final envDir = Platform.environment['ATTUNE_CONFIG_DIR'] ?? Platform.environment['ATTUNE_DATA_DIR'];
    final home = Platform.environment['HOME'];

    final candidates = <String>[
      if (envDir != null && envDir.isNotEmpty) envDir,
      if (home != null && home.isNotEmpty) '$home/Library/Application Support/attune',
      if (home != null && home.isNotEmpty) '$home/.config/attune',
      '${Directory.systemTemp.path}/attune',
    ];
    return _resolveFirstWritableDirectory(candidates, suffix: 'attune');
  }

  @override
  Directory get cacheDirectory {
    final envDir = Platform.environment['ATTUNE_CACHE_DIR'];
    final home = Platform.environment['HOME'];

    final candidates = <String>[
      if (envDir != null && envDir.isNotEmpty) envDir,
      if (home != null && home.isNotEmpty) '$home/Library/Caches/attune',
      if (home != null && home.isNotEmpty) '$home/.cache/attune',
      '${Directory.systemTemp.path}/attune_cache',
    ];
    return _resolveFirstWritableDirectory(candidates, suffix: 'attune_cache');
  }

  @override
  List<String> getMusicCandidateRoots({List<String>? additionalRoots}) {
    ensureValidWorkingDirectory();
    final roots = <String>[];
    if (additionalRoots != null) roots.addAll(additionalRoots);

    final envMusic = Platform.environment['ATTUNE_MUSIC_DIR'];
    if (envMusic != null && envMusic.isNotEmpty && !roots.contains(envMusic)) {
      roots.add(envMusic);
    }

    _addWorkspaceCandidateRoots(roots);

    final home = Platform.environment['HOME'];
    if (home != null && home.isNotEmpty) {
      for (final sub in ['Music', 'Downloads']) {
        final path = '$home/$sub';
        if (!roots.contains(path)) roots.add(path);
      }
    }

    return roots;
  }

  @override
  void ensureValidWorkingDirectory() {
    try {
      Directory.current;
    } catch (_) {
      try {
        final home = Platform.environment['HOME'];
        if (home != null && Directory(home).existsSync()) {
          Directory.current = home;
        } else {
          final exe = Platform.resolvedExecutable;
          final dir = File(exe).parent;
          if (dir.existsSync()) {
            Directory.current = dir.path;
          }
        }
      } catch (_) {}
    }
  }

  @override
  String get defaultUsername {
    final user = Platform.environment['USER'];
    if (user != null && user.trim().isNotEmpty) return user.trim();
    return 'Attune Listener';
  }
}

/// Hermetic Test environment delegator preventing filesystem side-effects during tests
class TestEnvironmentDelegator extends AppEnvironmentDelegator {
  final Directory? customTestDir;

  const TestEnvironmentDelegator({this.customTestDir});

  @override
  String get platformName => 'Test';

  @override
  bool get isTest => true;

  @override
  Directory get appDataDirectory {
    if (customTestDir != null) return customTestDir!;
    final dir = Directory('${Directory.systemTemp.path}/attune_test_$pid');
    if (!dir.existsSync()) {
      try {
        dir.createSync(recursive: true);
      } catch (_) {}
    }
    return dir;
  }

  @override
  Directory get cacheDirectory {
    final dir = Directory('${Directory.systemTemp.path}/attune_test_cache_$pid');
    if (!dir.existsSync()) {
      try {
        dir.createSync(recursive: true);
      } catch (_) {}
    }
    return dir;
  }

  @override
  List<String> getMusicCandidateRoots({List<String>? additionalRoots}) {
    final roots = <String>[];
    if (additionalRoots != null) roots.addAll(additionalRoots);
    _addWorkspaceCandidateRoots(roots);
    return roots;
  }

  @override
  void ensureValidWorkingDirectory() {
    // No-op in test runner
  }

  @override
  String get defaultUsername => 'Test Listener';
}
