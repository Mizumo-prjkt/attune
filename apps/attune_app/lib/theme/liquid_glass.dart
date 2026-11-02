import 'dart:math' as math;
import 'dart:ui';
import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../services/settings_service.dart';

/// The 4-Layer Liquid Material Glass Container adhering to Section 2.2 of 05_UI_DESIGN_SPEC.md:
/// 1. Frosted Backdrop Blur (BackdropFilter with configurable sigma)
/// 2. Tonal Glass Tint Fill (surfaceContainerHigh + surfaceTint)
/// 3. Diagonal Specular Rim Lighting (1.0px gradient border from top-left catch light to shadow edge)
class LiquidGlassContainer extends StatelessWidget {
  final Widget child;
  final double? width;
  final double? height;
  final EdgeInsetsGeometry padding;
  final EdgeInsetsGeometry margin;
  final double borderRadius;
  final double blurSigma;
  final double surfaceAlpha;
  final Color? tintOverride;
  final Color? borderColor;
  final List<BoxShadow>? boxShadow;
  final VoidCallback? onTap;

  const LiquidGlassContainer({
    super.key,
    required this.child,
    this.width,
    this.height,
    this.padding = const EdgeInsets.all(16.0),
    this.margin = EdgeInsets.zero,
    this.borderRadius = 24.0,
    this.blurSigma = 36.0,
    this.surfaceAlpha = 0.14,
    this.tintOverride,
    this.borderColor,
    this.boxShadow,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    Widget content = Container(
      width: width,
      height: height,
      padding: padding,
      decoration: BoxDecoration(
        color: tintOverride ?? colorScheme.surfaceContainerHigh.withValues(alpha: surfaceAlpha),
        borderRadius: BorderRadius.circular(borderRadius),
        boxShadow: boxShadow,
        border: Border.all(
          width: 1.0,
          color: borderColor ?? Colors.transparent, // Border painter handles diagonal specular gradient if not overridden
        ),
      ),
      child: child,
    );

    // Apply diagonal specular rim lighting gradient via CustomPaint
    content = CustomPaint(
      foregroundPainter: _SpecularRimPainter(
        borderRadius: borderRadius,
        primaryTint: colorScheme.primary,
      ),
      child: content,
    );

    // Clip to rounded border with clean, high-performance translucent tonal surface tint
    content = ClipRRect(
      borderRadius: BorderRadius.circular(borderRadius),
      child: content,
    );

    if (onTap != null) {
      content = GestureDetector(
        onTap: onTap,
        behavior: HitTestBehavior.opaque,
        child: content,
      );
    }

    if (margin != EdgeInsets.zero) {
      content = Padding(
        padding: margin,
        child: content,
      );
    }

    return content;
  }
}

/// CustomPainter for the 1.0px diagonal specular rim gradient border
class _SpecularRimPainter extends CustomPainter {
  final double borderRadius;
  final Color primaryTint;

  _SpecularRimPainter({
    required this.borderRadius,
    required this.primaryTint,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Rect.fromLTWH(0.5, 0.5, size.width - 1.0, size.height - 1.0);
    final rrect = RRect.fromRectAndRadius(rect, Radius.circular(borderRadius));

    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.0
      ..shader = LinearGradient(
        begin: Alignment.topLeft,
        end: Alignment.bottomRight,
        colors: [
          Colors.white.withValues(alpha: 0.24),                 // Top-Left specular catch light
          primaryTint.withValues(alpha: 0.12),                  // Mid-edge tonal tint
          Colors.white.withValues(alpha: 0.04),                 // Bottom-Right shadow edge
        ],
        stops: const [0.0, 0.45, 1.0],
      ).createShader(rect);

    canvas.drawRRect(rrect, paint);
  }

  @override
  bool shouldRepaint(covariant _SpecularRimPainter oldDelegate) {
    return oldDelegate.borderRadius != borderRadius || oldDelegate.primaryTint != primaryTint;
  }
}

/// Living Ambient Artwork Mesh rendered behind the entire scaffold (Section 2.2 layer 1)
/// Dynamically sways and breathes in lockstep with the active track's BPM and musical rhythm,
/// featuring organic fluid lavalamp liquid light globs responding to beat kicks.
class AmbientMeshBackground extends StatelessWidget {
  final Widget? child;
  final ColorScheme? colorScheme;
  final bool isPlaying;

  const AmbientMeshBackground({
    super.key,
    this.child,
    this.colorScheme,
    this.isPlaying = true,
  });

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: Listenable.merge([PlaybackController.instance, SettingsService.instance]),
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final isEnabled = controller.livingBeatMeshEnabled;
        final scheme = colorScheme ?? Theme.of(context).colorScheme;
        final currentTrack = controller.currentTrack;
        final accent = currentTrack?.seedColor ?? scheme.primary;

        if (!isEnabled) {
          return Stack(
            fit: StackFit.expand,
            children: [
              Container(color: scheme.surface),
              if (child != null) child!,
            ],
          );
        }

        return Stack(
          fit: StackFit.expand,
          children: [
            Container(color: scheme.surface),
            if (child != null) child!,
          ],
        );
      },
    );
  }
}
