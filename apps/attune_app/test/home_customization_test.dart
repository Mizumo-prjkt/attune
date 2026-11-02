import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/settings_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('Home Layout Customization & 3D Mirror Sheen Tests', () {
    test('Default home sections are loaded and non-empty', () {
      final settings = SettingsService.instance;
      expect(settings.homeSections.isNotEmpty, true);
      expect(settings.homeSections.length, 6);

      final sectionIds = settings.homeSections.map((s) => s.id).toList();
      expect(sectionIds.contains('search_bar'), true);
      expect(sectionIds.contains('for_you_mixes'), true);
      expect(sectionIds.contains('genre_chips'), true);
      expect(sectionIds.contains('quick_picks'), true);
      expect(sectionIds.contains('recent_tracks'), true);
      expect(sectionIds.contains('all_tracks'), true);
    });

    test('Toggles section visibility without losing ordering', () {
      final settings = SettingsService.instance;
      final initialEnabled = settings.homeSections.firstWhere((s) => s.id == 'for_you_mixes').enabled;

      settings.toggleHomeSection('for_you_mixes', !initialEnabled);
      expect(settings.homeSections.firstWhere((s) => s.id == 'for_you_mixes').enabled, !initialEnabled);

      // Restore
      settings.toggleHomeSection('for_you_mixes', initialEnabled);
      expect(settings.homeSections.firstWhere((s) => s.id == 'for_you_mixes').enabled, initialEnabled);
    });

    test('Reorders sections correctly and gracefully handles bounds', () {
      final settings = SettingsService.instance;
      final originalFirst = settings.homeSections[0].id;
      final originalSecond = settings.homeSections[1].id;

      // Move index 0 down to index 2
      settings.reorderHomeSections(0, 2);
      expect(settings.homeSections[0].id, originalSecond);

      // Out of bounds check (should be a no-op, preventing segfaults/exceptions)
      settings.reorderHomeSections(-1, 5);
      settings.reorderHomeSections(0, 999);

      // Reset to default
      settings.resetHomeSectionsToDefault();
      expect(settings.homeSections[0].id, originalFirst);
    });

    test('3D Mirror Sheen requires app restart when toggled', () {
      final settings = SettingsService.instance;
      final initialActive = settings.liquidGlass3dMirror;
      expect(settings.restartRequiredForGlass, false);

      // Toggle pending value
      settings.setPendingLiquidGlass3dMirror(!initialActive);
      expect(settings.pendingLiquidGlass3dMirror, !initialActive);
      expect(settings.liquidGlass3dMirror, initialActive); // Active remains unchanged until reboot
      expect(settings.restartRequiredForGlass, true);

      // Revert pending value
      settings.setPendingLiquidGlass3dMirror(initialActive);
      expect(settings.restartRequiredForGlass, false);
    });
  });
}
