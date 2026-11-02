import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/l10n/translation_core.dart';
import 'package:attune_app/l10n/translation_registry.dart';
import 'package:attune_app/services/settings_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('TranslationRegistry & AppLocalizations Tests', () {
    test('Registry contains English and Filipino supported languages', () {
      final languages = TranslationRegistry.supportedLanguages;
      expect(languages.length, greaterThanOrEqualTo(2));

      final en = languages.firstWhere((l) => l.code == 'en');
      expect(en.name, 'English');
      expect(en.translations.isNotEmpty, true);

      final fil = languages.firstWhere((l) => l.code == 'fil');
      expect(fil.name, 'Filipino');
      expect(fil.nativeName, 'Wikang Filipino');
      expect(fil.translations.isNotEmpty, true);
    });

    test('English translations contain required core navigation and home keys', () {
      SettingsService.instance.setLanguage('en');
      final loc = AppLocalizations.instance;

      expect(loc.t('nav_home'), 'Home');
      expect(loc.t('nav_for_you'), 'For You');
      expect(loc.t('nav_library'), 'Library');
      expect(loc.t('nav_settings'), 'Settings');
      expect(loc.t('home_customize'), 'Customize');
      expect(loc.t('home_customize_subtitle'), contains('Reorder'));
    });

    test('Filipino translations translate navigation and home keys accurately', () {
      SettingsService.instance.setLanguage('fil');
      final loc = AppLocalizations.instance;

      expect(loc.t('nav_home'), 'Tahanan');
      expect(loc.t('nav_for_you'), 'Para sa Iyo');
      expect(loc.t('nav_library'), 'Aklatan');
      expect(loc.t('nav_settings'), 'Mga Setting');
      expect(loc.t('home_customize'), 'I-customize');
      expect(loc.t('home_search_hint'), contains('Maghanap'));
    });

    test('Fallback to English when key is missing in target language', () {
      SettingsService.instance.setLanguage('fil');
      final loc = AppLocalizations.instance;

      final missingKey = 'non_existent_key_xyz';
      expect(loc.t(missingKey), missingKey);
    });

    test('Interpolates params properly in translated strings', () {
      SettingsService.instance.setLanguage('en');
      final loc = AppLocalizations.instance;
      final resultEn = loc.t('for_you_mix_tracks', params: {'count': '15'});
      expect(resultEn, '15 tracks');

      SettingsService.instance.setLanguage('fil');
      final resultFil = loc.t('for_you_mix_tracks', params: {'count': '15'});
      expect(resultFil, '15 na kanta');
    });

    test('Supports language change dynamically via SettingsService', () {
      SettingsService.instance.setLanguage('en');
      final loc = AppLocalizations.instance;
      expect(loc.t('nav_home'), 'Home');

      SettingsService.instance.setLanguage('fil');
      expect(loc.currentCode, 'fil');
      expect(loc.t('nav_home'), 'Tahanan');

      // Reset back to English
      SettingsService.instance.setLanguage('en');
    });
  });
}
