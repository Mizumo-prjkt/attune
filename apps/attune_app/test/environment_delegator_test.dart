import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/environment_delegator.dart';
import 'package:attune_app/services/audio_storage_service.dart';
import 'package:attune_app/services/settings_service.dart';

class MockCustomEnvironmentDelegator extends AppEnvironmentDelegator {
  final Directory customDataDir;
  final Directory customCacheDir;

  const MockCustomEnvironmentDelegator({
    required this.customDataDir,
    required this.customCacheDir,
  });

  @override
  String get platformName => 'MockCustom';

  @override
  Directory get appDataDirectory => customDataDir;

  @override
  Directory get cacheDirectory => customCacheDir;

  @override
  List<String> getMusicCandidateRoots({List<String>? additionalRoots}) {
    return [
      ...?additionalRoots,
      '${customDataDir.path}/custom_music',
    ];
  }

  @override
  void ensureValidWorkingDirectory() {}

  @override
  String get defaultUsername => 'CustomMockListener';
}

void main() {
  group('AppEnvironmentDelegator Architecture Tests', () {
    tearDown(() {
      AppEnvironmentDelegator.reset();
    });

    test('Default delegator in test runner resolves to TestEnvironmentDelegator', () {
      final delegator = AppEnvironmentDelegator.current;
      expect(delegator, isNotNull);
      expect(delegator.isTest, isTrue);
      expect(delegator.platformName, 'Test');
      expect(delegator.defaultUsername, 'Test Listener');
      expect(delegator.appDataDirectory.existsSync(), isTrue);
      expect(delegator.cacheDirectory.existsSync(), isTrue);
    });

    test('Custom delegator can be injected and cleanly reset', () {
      final tempDir = Directory.systemTemp.createTempSync('attune_mock_test_');
      final dataDir = Directory('${tempDir.path}/data')..createSync();
      final cacheDir = Directory('${tempDir.path}/cache')..createSync();

      final mock = MockCustomEnvironmentDelegator(
        customDataDir: dataDir,
        customCacheDir: cacheDir,
      );

      AppEnvironmentDelegator.instance = mock;
      expect(AppEnvironmentDelegator.current, same(mock));
      expect(AppEnvironmentDelegator.current.platformName, 'MockCustom');
      expect(AppEnvironmentDelegator.current.defaultUsername, 'CustomMockListener');
      expect(AppEnvironmentDelegator.current.appDataDirectory.path, dataDir.path);
      expect(AppEnvironmentDelegator.current.cacheDirectory.path, cacheDir.path);

      final roots = AppEnvironmentDelegator.current.getMusicCandidateRoots(
        additionalRoots: ['/test/extra/folder'],
      );
      expect(roots, contains('/test/extra/folder'));
      expect(roots, contains('${dataDir.path}/custom_music'));

      AppEnvironmentDelegator.reset();
      expect(AppEnvironmentDelegator.current.platformName, 'Test');

      try {
        tempDir.deleteSync(recursive: true);
      } catch (_) {}
    });

    test('AndroidEnvironmentDelegator conforms to Android specifications and scoped storage', () {
      const android = AndroidEnvironmentDelegator();
      expect(android.isAndroid, isTrue);
      expect(android.platformName, 'Android');
      expect(android.defaultUsername, 'Attune Listener');

      // Writable data and cache directories
      final dataDir = android.appDataDirectory;
      expect(dataDir.existsSync(), isTrue);
      final cacheDir = android.cacheDirectory;
      expect(cacheDir.existsSync(), isTrue);

      final roots = android.getMusicCandidateRoots(additionalRoots: ['/custom/android/music']);
      expect(roots, contains('/custom/android/music'));
      expect(roots, contains('/storage/emulated/0/Music'));
      expect(roots, contains('/sdcard/Music'));
      expect(roots, contains('/storage/emulated/0/Download'));
      expect(roots, contains('/storage/emulated/0/Android/media/org.mizproject.attune'));

      // ensureValidWorkingDirectory does not throw
      expect(() => android.ensureValidWorkingDirectory(), returnsNormally);
    });

    test('LinuxEnvironmentDelegator conforms to XDG and POSIX home directory conventions', () {
      const linux = LinuxEnvironmentDelegator();
      expect(linux.isLinux, isTrue);
      expect(linux.platformName, 'Linux');
      expect(linux.defaultUsername, isNotEmpty);

      final dataDir = linux.appDataDirectory;
      expect(dataDir.existsSync(), isTrue);
      final cacheDir = linux.cacheDirectory;
      expect(cacheDir.existsSync(), isTrue);

      final roots = linux.getMusicCandidateRoots(additionalRoots: ['/opt/linux/music']);
      expect(roots, contains('/opt/linux/music'));

      final home = Platform.environment['HOME'];
      if (home != null && home.isNotEmpty) {
        expect(roots, contains('$home/Music'));
      }

      expect(() => linux.ensureValidWorkingDirectory(), returnsNormally);
    });

    test('WindowsEnvironmentDelegator conforms to Windows USERPROFILE conventions', () {
      const windows = WindowsEnvironmentDelegator();
      expect(windows.isWindows, isTrue);
      expect(windows.platformName, 'Windows');
      expect(windows.defaultUsername, isNotEmpty);

      final dataDir = windows.appDataDirectory;
      expect(dataDir.existsSync(), isTrue);
      final cacheDir = windows.cacheDirectory;
      expect(cacheDir.existsSync(), isTrue);

      final roots = windows.getMusicCandidateRoots(additionalRoots: [r'D:\Audio\Lossless']);
      expect(roots, contains(r'D:\Audio\Lossless'));

      expect(() => windows.ensureValidWorkingDirectory(), returnsNormally);
    });

    test('MacOsEnvironmentDelegator conforms to Application Support conventions', () {
      const macos = MacOsEnvironmentDelegator();
      expect(macos.isMacOS, isTrue);
      expect(macos.platformName, 'macOS');
      expect(macos.defaultUsername, isNotEmpty);

      final dataDir = macos.appDataDirectory;
      expect(dataDir.existsSync(), isTrue);
      final cacheDir = macos.cacheDirectory;
      expect(cacheDir.existsSync(), isTrue);

      final roots = macos.getMusicCandidateRoots(additionalRoots: ['/Volumes/External/Music']);
      expect(roots, contains('/Volumes/External/Music'));

      expect(() => macos.ensureValidWorkingDirectory(), returnsNormally);
    });

    test('TestEnvironmentDelegator supports custom temp directory injection', () {
      final customDir = Directory.systemTemp.createTempSync('attune_custom_test_env_');
      final testEnv = TestEnvironmentDelegator(customTestDir: customDir);

      expect(testEnv.isTest, isTrue);
      expect(testEnv.platformName, 'Test');
      expect(testEnv.appDataDirectory.path, customDir.path);
      expect(testEnv.cacheDirectory.existsSync(), isTrue);

      final roots = testEnv.getMusicCandidateRoots(additionalRoots: ['/test/root']);
      expect(roots, contains('/test/root'));

      try {
        customDir.deleteSync(recursive: true);
      } catch (_) {}
    });

    test('AudioStorageService transparently uses AppEnvironmentDelegator', () {
      final tempDir = Directory.systemTemp.createTempSync('attune_audio_storage_env_');
      final mock = MockCustomEnvironmentDelegator(
        customDataDir: tempDir,
        customCacheDir: tempDir,
      );

      AppEnvironmentDelegator.instance = mock;

      final appDataDir = AudioStorageService.instance.getAppDataDirectory();
      expect(appDataDir.path, tempDir.path);

      final cacheDir = AudioStorageService.instance.getCacheDirectory();
      expect(cacheDir.path, tempDir.path);

      final roots = AudioStorageService.instance.getCandidateRoots(
        additionalRoots: ['/delegated/extra'],
      );
      expect(roots, contains('/delegated/extra'));
      expect(roots, contains('${tempDir.path}/custom_music'));

      AppEnvironmentDelegator.reset();
      try {
        tempDir.deleteSync(recursive: true);
      } catch (_) {}
    });

    test('SettingsService utilizes AppEnvironmentDelegator defaults', () {
      final settings = SettingsService.instance;
      expect(settings.profileName, isNotEmpty);
    });
  });
}
