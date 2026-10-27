import 'dart:ui';
import 'package:flutter/material.dart';

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
        border: Border.all(
          width: 1.0,
          color: Colors.transparent, // Border painter handles diagonal specular gradient
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

    // Apply BackdropFilter blur
    content = ClipRRect(
      borderRadius: BorderRadius.circular(borderRadius),
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: blurSigma, sigmaY: blurSigma),
        child: content,
      ),
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
class AmbientMeshBackground extends StatefulWidget {
  final Widget child;
  final ColorScheme colorScheme;
  final bool isPlaying;

  const AmbientMeshBackground({
    super.key,
    required this.child,
    required this.colorScheme,
    this.isPlaying = true,
  });

  @override
  State<AmbientMeshBackground> createState() => _AmbientMeshBackgroundState();
}

class _AmbientMeshBackgroundState extends State<AmbientMeshBackground> with SingleTickerProviderStateMixin {
  late AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 12),
    )..repeat(reverse: true);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _controller,
      builder: (context, _) {
        final t = _controller.value;
        final primary = widget.colorScheme.primary;
        final secondary = widget.colorScheme.secondary;
        final tertiary = widget.colorScheme.tertiary;

        return Stack(
          children: [
            // Deep base background
            Container(color: widget.colorScheme.surface),

            // Diffused breathing radial color orbs
            Positioned(
              top: -80 + (t * 40),
              left: -80 + (t * 20),
              width: 360,
              height: 360,
              child: _Orb(color: primary.withValues(alpha: 0.28)),
            ),
            Positioned(
              bottom: 100 - (t * 30),
              right: -60 - (t * 20),
              width: 420,
              height: 420,
              child: _Orb(color: secondary.withValues(alpha: 0.22)),
            ),
            Positioned(
              top: 220 + (t * 50),
              right: 20 - (t * 30),
              width: 300,
              height: 300,
              child: _Orb(color: tertiary.withValues(alpha: 0.18)),
            ),

            // Child scaffold content on top
            widget.child,
          ],
        );
      },
    );
  }
}

class _Orb extends StatelessWidget {
  final Color color;

  const _Orb({required this.color});

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        gradient: RadialGradient(
          colors: [
            color,
            color.withValues(alpha: 0.0),
          ],
        ),
      ),
    );
  }
}
