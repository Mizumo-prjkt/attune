import 'package:flutter/material.dart';
import '../services/settings_service.dart';
import 'translation_registry.dart';

/// Reactive localization engine for Attune
class AppLocalizations extends ChangeNotifier {
  static final AppLocalizations instance = AppLocalizations._internal();

  AppLocalizations._internal() {
    SettingsService.instance.addListener(_onSettingsChanged);
  }

  String _currentCode = 'en';

  void _onSettingsChanged() {
    final newCode = SettingsService.instance.language;
    if (_currentCode != newCode) {
      _currentCode = newCode;
      notifyListeners();
    }
  }

  /// Active language code
  String get currentCode => SettingsService.instance.language;

  /// Active language metadata
  AppLanguage get currentLanguage => TranslationRegistry.getLanguage(currentCode);

  /// Translates a localized key with optional template parameters (e.g. `{count}`).
  /// Falls back to English if the key is missing in the active translation.
  String t(String key, {Map<String, dynamic>? params}) {
    final activeDict = currentLanguage.translations;
    var result = activeDict[key];

    // Fallback to English if missing
    if (result == null || result.isEmpty) {
      result = TranslationRegistry.defaultLanguage.translations[key];
    }

    // Fallback to key itself if not in English
    if (result == null || result.isEmpty) {
      return key;
    }

    if (params != null && params.isNotEmpty) {
      params.forEach((paramKey, paramValue) {
        result = result!.replaceAll('{$paramKey}', paramValue.toString());
      });
    }

    return result!;
  }
}

/// Convenience context extension for zero-boilerplate translation lookups:
/// `context.tr('nav_home')`
extension LocalizedContext on BuildContext {
  String tr(String key, [Map<String, dynamic>? params]) {
    return AppLocalizations.instance.t(key, params: params);
  }

  AppLanguage get currentLanguage => AppLocalizations.instance.currentLanguage;
}
