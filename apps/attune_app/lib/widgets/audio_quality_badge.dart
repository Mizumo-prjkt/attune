import 'package:flutter/material.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Human-Friendly Audio Quality Badge adhering to Section 3.1 of 05_UI_DESIGN_SPEC.md:
/// Strictly avoids "Dolby Atmos", "HI-RES MQA", or "Spatial 3D".
class AudioQualityBadge extends StatelessWidget {
  final String format;
  final int sampleRate;
  final int bitDepth;
  final bool isProprietary;

  const AudioQualityBadge({
    super.key,
    required this.format,
    required this.sampleRate,
    this.bitDepth = 16,
    this.isProprietary = false,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    String badgeText;
    Color containerTint;
    Color textColor;

    final fmt = format.toUpperCase();
    final isLossless = fmt == 'FLAC' || fmt == 'WAV' || fmt == 'ALAC' || fmt == 'AIFF';

    if (isProprietary) {
      badgeText = 'Proprietary Audio Codec';
      containerTint = colorScheme.tertiaryContainer.withValues(alpha: 0.24);
      textColor = colorScheme.onTertiaryContainer;
    } else if (isLossless && sampleRate >= 44100) {
      badgeText = 'Very Good Quality Audio';
      containerTint = colorScheme.primaryContainer.withValues(alpha: 0.22);
      textColor = colorScheme.primary;
    } else {
      badgeText = 'Good Quality Audio';
      containerTint = colorScheme.secondaryContainer.withValues(alpha: 0.20);
      textColor = colorScheme.secondary;
    }

    final IconData iconData;
    if (isProprietary) {
      iconData = Icons.lock_outline_rounded;
    } else if (isLossless && sampleRate >= 44100) {
      iconData = Icons.diamond_rounded;
    } else {
      iconData = Icons.music_note_rounded;
    }

    return LiquidGlassContainer(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      borderRadius: 12,
      surfaceAlpha: 0.12,
      tintOverride: containerTint,
      child: FittedBox(
        fit: BoxFit.scaleDown,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              iconData,
              size: 13,
              color: textColor,
            ),
            const SizedBox(width: 5),
            Text(
              badgeText,
              style: AttuneTypography.soraBadge(color: textColor),
            ),
            const SizedBox(width: 6),
            Text(
              '$bitDepth-bit / ${(sampleRate / 1000).toStringAsFixed(1)} kHz $fmt',
              style: AttuneTypography.soraMetric(
                fontSize: 10,
                weight: FontWeight.w500,
                color: textColor.withValues(alpha: 0.8),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
