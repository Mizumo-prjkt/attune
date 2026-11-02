import 'en.dart';
import 'fil.dart';

/// Metadata and dictionary bundle for a supported language in Attune
class AppLanguage {
  final String code;
  final String name;
  final String nativeName;
  final Map<String, String> translations;

  const AppLanguage({
    required this.code,
    required this.name,
    required this.nativeName,
    required this.translations,
  });
}

/// Central registry indexing all available translations in Attune.
/// To register a new language:
/// 1. Create a dictionary map in `lib/l10n/<code>.dart`
/// 2. Add an AppLanguage entry to [supportedLanguages] below.
class TranslationRegistry {
  static const AppLanguage english = AppLanguage(
    code: 'en',
    name: 'English',
    nativeName: 'English',
    translations: enTranslations,
  );

  static const AppLanguage filipino = AppLanguage(
    code: 'fil',
    name: 'Filipino',
    nativeName: 'Wikang Filipino',
    translations: filTranslations,
  );

  /// All registered languages available for in-app selection
  static final List<AppLanguage> supportedLanguages = [
    english,
    filipino,
  ];

  /// Fallback language when a key is missing or language code is unrecognized
  static const AppLanguage defaultLanguage = english;

  /// Resolves an [AppLanguage] by code, falling back to English if not found
  static AppLanguage getLanguage(String code) {
    return supportedLanguages.firstWhere(
      (lang) => lang.code.toLowerCase() == code.toLowerCase(),
      orElse: () => defaultLanguage,
    );
  }

  /// Checks if a language code is supported
  static bool isSupported(String code) {
    return supportedLanguages.any(
      (lang) => lang.code.toLowerCase() == code.toLowerCase(),
    );
  }
}
