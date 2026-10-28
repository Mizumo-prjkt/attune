import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/update_service.dart';

void main() {
  group('UpdateService Semantic Version Comparison Tests', () {
    test('Identifies newer versions correctly', () {
      expect(UpdateService.compareSemanticVersions('0.1.0', '0.2.0'), -1);
      expect(UpdateService.compareSemanticVersions('0.1.0', '1.0.0'), -1);
      expect(UpdateService.compareSemanticVersions('0.1.0', '0.1.1'), -1);
      expect(UpdateService.compareSemanticVersions('v0.1.0', 'v0.2.0'), -1);
      expect(UpdateService.compareSemanticVersions('0.1.0-alpha', '0.2.0'), -1);
    });

    test('Identifies older or equal versions correctly', () {
      expect(UpdateService.compareSemanticVersions('0.2.0', '0.1.0'), 1);
      expect(UpdateService.compareSemanticVersions('1.0.0', '0.9.9'), 1);
      expect(UpdateService.compareSemanticVersions('0.1.0', '0.1.0'), 0);
      expect(UpdateService.compareSemanticVersions('v0.1.0', '0.1.0'), 0);
    });
  });

  group('ReleaseAsset Parsing Tests', () {
    test('Parses Linux AppImage asset and recognizes debug vs release', () {
      final jsonRelease = {
        'name': 'Attune-0.1.0-x86_64-release.AppImage',
        'browser_download_url': 'https://github.com/Mizumo-prjkt/attune/releases/download/v0.1.0/Attune-0.1.0-x86_64-release.AppImage',
        'size': 18874368,
        'content_type': 'application/x-executable',
      };
      final asset = ReleaseAsset.fromJson(jsonRelease);
      expect(asset.name, 'Attune-0.1.0-x86_64-release.AppImage');
      expect(asset.platformCategory, 'linux');
      expect(asset.isDebug, false);
      expect(asset.formattedSize, '18.0 MB');

      final jsonDebug = {
        'name': 'Attune-0.1.0-x86_64-debug.AppImage',
        'browser_download_url': 'https://github.com/Mizumo-prjkt/attune/releases/download/v0.1.0/Attune-0.1.0-x86_64-debug.AppImage',
        'size': 73400320,
        'content_type': 'application/x-executable',
      };
      final assetDbg = ReleaseAsset.fromJson(jsonDebug);
      expect(assetDbg.name, 'Attune-0.1.0-x86_64-debug.AppImage');
      expect(assetDbg.platformCategory, 'linux');
      expect(assetDbg.isDebug, true);
      expect(assetDbg.formattedSize, '70.0 MB');
    });

    test('Parses Windows and Android assets correctly', () {
      final jsonWin = {
        'name': 'Attune-0.1.0-Setup.exe',
        'browser_download_url': 'https://example.com/Attune-0.1.0-Setup.exe',
        'size': 35651584,
        'content_type': 'application/octet-stream',
      };
      final assetWin = ReleaseAsset.fromJson(jsonWin);
      expect(assetWin.platformCategory, 'windows');
      expect(assetWin.isDebug, false);

      final jsonApk = {
        'name': 'Attune-0.1.0-Android-Debug.apk',
        'browser_download_url': 'https://example.com/app-debug.apk',
        'size': 211812352,
        'content_type': 'application/vnd.android.package-archive',
      };
      final assetApk = ReleaseAsset.fromJson(jsonApk);
      expect(assetApk.platformCategory, 'android');
      expect(assetApk.isDebug, true);
    });
  });

  group('UpdateInfo JSON Parsing', () {
    test('Constructs UpdateInfo with notes, assets, and isNewer flag', () {
      final mockJson = {
        'tag_name': 'v0.2.0',
        'name': 'Attune v0.2.0 (Dual-Deck & Updates)',
        'body': '### Patch Notes\n* Added in-app update checker.\n* Built dual release and debug targets.',
        'html_url': 'https://github.com/Mizumo-prjkt/attune/releases/tag/v0.2.0',
        'published_at': '2026-10-28T09:00:00Z',
        'prerelease': false,
        'assets': [
          {
            'name': 'Attune-0.2.0-x86_64-release.AppImage',
            'browser_download_url': 'https://example.com/appimage',
            'size': 18000000,
          },
          {
            'name': 'Attune-0.2.0-Android-Debug.apk',
            'browser_download_url': 'https://example.com/apk',
            'size': 200000000,
          }
        ],
      };

      final info = UpdateInfo.fromJson(mockJson, isNewer: true);
      expect(info.tagName, 'v0.2.0');
      expect(info.isNewer, true);
      expect(info.assets.length, 2);
      expect(info.releaseNotes, contains('in-app update checker'));
    });
  });
}
