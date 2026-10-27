import 'package:flutter/material.dart';
import '../services/platform_navigation.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Subtle Inline Transition Connector Pill adhering to Section 3.2 of 05_UI_DESIGN_SPEC.md:
/// Replaces the bulky "Smart Transition AI" box with a sleek translucent pill that
/// expands into the Dual-Deck Transition Inspector upon tapping.
class TransitionConnectorPill extends StatelessWidget {
  final String archetype;
  final double durationSeconds;
  final double confidenceScore;
  final bool isCompact;

  const TransitionConnectorPill({
    super.key,
    required this.archetype,
    required this.durationSeconds,
    this.confidenceScore = 0.91,
    this.isCompact = false,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return LiquidGlassContainer(
      padding: EdgeInsets.symmetric(
        horizontal: isCompact ? 10 : 14,
        vertical: isCompact ? 6 : 8,
      ),
      borderRadius: 16,
      surfaceAlpha: 0.16,
      tintOverride: colorScheme.primary.withValues(alpha: 0.14),
      onTap: () => PlatformNavigation.openTransitionInspector(context),
      child: FittedBox(
        fit: BoxFit.scaleDown,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Dual-wave micro-icon
            Icon(
              Icons.graphic_eq_rounded,
              size: isCompact ? 14 : 16,
              color: colorScheme.primary,
            ),
            const SizedBox(width: 8),
            Text(
              '$archetype • ${durationSeconds.toStringAsFixed(1)}s',
              style: AttuneTypography.soraBadge(color: colorScheme.onSurface),
            ),
            const SizedBox(width: 6),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1.5),
              decoration: BoxDecoration(
                color: colorScheme.primary.withValues(alpha: 0.20),
                borderRadius: BorderRadius.circular(6),
              ),
              child: Text(
                '${(confidenceScore * 100).toInt()}% match',
                style: AttuneTypography.soraMetric(
                  fontSize: 9,
                  weight: FontWeight.w700,
                  color: colorScheme.primary,
                ),
              ),
            ),
            const SizedBox(width: 6),
            Icon(
              Icons.chevron_right_rounded,
              size: 14,
              color: colorScheme.onSurface.withValues(alpha: 0.5),
            ),
          ],
        ),
      ),
    );
  }
}
