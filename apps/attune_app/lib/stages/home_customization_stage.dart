import 'package:flutter/material.dart';
import '../services/settings_service.dart';
import '../l10n/translation_core.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Non-popup, dedicated settings component allowing users to customize and reorder
/// elements on the Attune Home screen. Adheres strictly to zero-hardcoding and zero-popup rules.
class HomeCustomizationStage extends StatelessWidget {
  const HomeCustomizationStage({super.key});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      backgroundColor: colorScheme.surface,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        title: Text(
          context.tr('home_customize_title'),
          style: AttuneTypography.soraTitleMedium(color: colorScheme.onSurface),
        ),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back_rounded),
          onPressed: () => Navigator.of(context).pop(),
        ),
        actions: [
          TextButton.icon(
            icon: const Icon(Icons.restart_alt_rounded, size: 18),
            label: Text(context.tr('common_reset')),
            onPressed: () => SettingsService.instance.resetHomeSectionsToDefault(),
          ),
          const SizedBox(width: 8),
        ],
      ),
      body: ListenableBuilder(
        listenable: Listenable.merge([SettingsService.instance, AppLocalizations.instance]),
        builder: (context, _) {
          final sections = SettingsService.instance.homeSections;

          return ListView(
            padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
            children: [
              // Informative banner explaining drag-to-reorder and visibility toggles
              LiquidGlassContainer(
                padding: const EdgeInsets.all(16),
                borderRadius: 22,
                surfaceAlpha: 0.12,
                child: Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.all(10),
                      decoration: BoxDecoration(
                        color: colorScheme.primary.withValues(alpha: 0.15),
                        shape: BoxShape.circle,
                      ),
                      child: Icon(Icons.tune_rounded, color: colorScheme.primary, size: 22),
                    ),
                    const SizedBox(width: 14),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            context.tr('home_customize_subtitle'),
                            style: AttuneTypography.jakartaBodySmall(
                              color: colorScheme.onSurface.withValues(alpha: 0.8),
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            'Hold & drag handles to reorder. Toggle switches to show or hide sections.',
                            style: AttuneTypography.jakartaBodySmall(
                              color: colorScheme.onSurface.withValues(alpha: 0.5),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),

              const SizedBox(height: 16),

              // Reorderable list of sections (No popups)
              ReorderableListView.builder(
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                itemCount: sections.length,
                // ignore: deprecated_member_use
                onReorder: (oldIndex, newIndex) {
                  SettingsService.instance.reorderHomeSections(oldIndex, newIndex);
                },
                itemBuilder: (context, index) {
                  final section = sections[index];

                  return Padding(
                    key: ValueKey(section.id),
                    padding: const EdgeInsets.only(bottom: 10),
                    child: LiquidGlassContainer(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                      borderRadius: 20,
                      surfaceAlpha: section.enabled ? 0.14 : 0.05,
                      child: Row(
                        children: [
                          // Reorder Drag Handle
                          ReorderableDragStartListener(
                            index: index,
                            child: Padding(
                              padding: const EdgeInsets.only(right: 12.0),
                              child: Icon(
                                Icons.drag_indicator_rounded,
                                color: colorScheme.onSurface.withValues(alpha: 0.4),
                                size: 22,
                              ),
                            ),
                          ),

                          // Section Icon
                          Container(
                            padding: const EdgeInsets.all(8),
                            decoration: BoxDecoration(
                              color: section.enabled
                                  ? colorScheme.primary.withValues(alpha: 0.15)
                                  : colorScheme.onSurface.withValues(alpha: 0.05),
                              borderRadius: BorderRadius.circular(12),
                            ),
                            child: Icon(
                              section.icon,
                              size: 18,
                              color: section.enabled ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.4),
                            ),
                          ),

                          const SizedBox(width: 14),

                          // Title & Description
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  context.tr(section.titleKey),
                                  style: AttuneTypography.soraSubheading(
                                    fontSize: 14,
                                    fontWeight: FontWeight.w600,
                                    color: section.enabled
                                        ? colorScheme.onSurface
                                        : colorScheme.onSurface.withValues(alpha: 0.4),
                                  ),
                                ),
                                const SizedBox(height: 2),
                                Text(
                                  context.tr(section.descriptionKey),
                                  style: AttuneTypography.jakartaBodySmall(
                                    color: colorScheme.onSurface.withValues(alpha: 0.5),
                                    fontSize: 11,
                                  ),
                                ),
                              ],
                            ),
                          ),

                          // Visibility Switch Toggle
                          Switch(
                            value: section.enabled,
                            activeThumbColor: colorScheme.primary,
                            onChanged: (val) {
                              SettingsService.instance.toggleHomeSection(section.id, val);
                            },
                          ),
                        ],
                      ),
                    ),
                  );
                },
              ),

              const SizedBox(height: 16),

              // Reset Button
              Center(
                child: OutlinedButton.icon(
                  style: OutlinedButton.styleFrom(
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                    side: BorderSide(color: colorScheme.outline.withValues(alpha: 0.3)),
                    padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                  ),
                  icon: const Icon(Icons.restart_alt_rounded, size: 18),
                  label: Text(
                    context.tr('settings_reset_default'),
                    style: AttuneTypography.soraTitleSmall(color: colorScheme.onSurface, fontSize: 13),
                  ),
                  onPressed: () => SettingsService.instance.resetHomeSectionsToDefault(),
                ),
              ),

              const SizedBox(height: 40),
            ],
          );
        },
      ),
    );
  }
}
