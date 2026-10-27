import 'dart:math' as math;
import 'dart:ui';
import 'package:flutter/material.dart';
import '../services/playback_controller.dart';

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
/// Dynamically sways and breathes in lockstep with the active track's BPM and musical rhythm.
class AmbientMeshBackground extends StatefulWidget {
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
  State<AmbientMeshBackground> createState() => _AmbientMeshBackgroundState();
}

class _AmbientMeshBackgroundState extends State<AmbientMeshBackground> with TickerProviderStateMixin {
  late AnimationController _driftController;
  late AnimationController _beatController;
  double _lastBpm = 120.0;

  @override
  void initState() {
    super.initState();
    _driftController = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 14),
    )..repeat(reverse: true);

    _beatController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 500), // 120 BPM default
    )..repeat(reverse: true);
  }

  void _updateBeatTempo(double rawBpm, bool isPlaying) {
    if (!isPlaying) {
      if (_beatController.isAnimating) {
        _beatController.stop();
      }
      return;
    }

    final effectiveBpm = rawBpm > 40.0 && rawBpm < 260.0 ? rawBpm : 120.0;
    if ((effectiveBpm - _lastBpm).abs() > 1.0 || !_beatController.isAnimating) {
      _lastBpm = effectiveBpm;
      final beatMs = (60000.0 / effectiveBpm).round().clamp(250, 1500);
      _beatController.duration = Duration(milliseconds: beatMs);
      if (!_beatController.isAnimating) {
        _beatController.repeat(reverse: true);
      }
    }
  }

  @override
  void dispose() {
    _driftController.dispose();
    _beatController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final bpm = controller.currentTrack?.bpm ?? 120.0;
        final isPlaying = controller.isPlaying;

        _updateBeatTempo(bpm, isPlaying);

        return AnimatedBuilder(
          animation: Listenable.merge([_driftController, _beatController]),
          builder: (context, _) {
            final t = _driftController.value;
            final beatPulse = isPlaying ? Curves.easeInOutSine.transform(_beatController.value) : 0.0;
            final beatScale = 1.0 + (0.12 * beatPulse);
            final beatAlphaMult = 0.85 + (0.35 * beatPulse);

            final scheme = widget.colorScheme ?? Theme.of(context).colorScheme;
            final primary = scheme.primary;
            final secondary = scheme.secondary;
            final tertiary = scheme.tertiary;

            final swayX = math.sin(t * math.pi * 2) * 40.0;
            final swayY = math.cos(t * math.pi * 2) * 30.0;

            return Stack(
              children: [
                // Deep base surface
                Container(color: scheme.surface),

                // 1. Primary Hue Orb (Top-Left / Center sway with beat)
                Positioned(
                  top: -60 + swayY,
                  left: -60 + swayX,
                  width: 440 * beatScale,
                  height: 440 * beatScale,
                  child: _Orb(color: primary.withValues(alpha: 0.32 * beatAlphaMult)),
                ),

                // 2. Secondary Hue Orb (Bottom-Right beat wash)
                Positioned(
                  bottom: 60 - swayY,
                  right: -50 - swayX,
                  width: 480 * beatScale,
                  height: 480 * beatScale,
                  child: _Orb(color: secondary.withValues(alpha: 0.28 * beatAlphaMult)),
                ),

                // 3. Tertiary Hue Orb (Center-Right rhythmic accent)
                Positioned(
                  top: 200 + (swayY * 1.5),
                  right: 40 + (swayX * 1.2),
                  width: 380 * (1.0 + 0.15 * (1.0 - beatPulse)),
                  height: 380 * (1.0 + 0.15 * (1.0 - beatPulse)),
                  child: _Orb(color: tertiary.withValues(alpha: 0.22 * beatAlphaMult)),
                ),

                // 4. Center Ambient Diffuse Beat Halo
                Positioned.fill(
                  child: IgnorePointer(
                    child: Container(
                      decoration: BoxDecoration(
                        gradient: RadialGradient(
                          center: Alignment(swayX / 100.0, swayY / 100.0),
                          radius: 1.25 * beatScale,
                          colors: [
                            primary.withValues(alpha: 0.08 * beatAlphaMult),
                            secondary.withValues(alpha: 0.04 * beatAlphaMult),
                            Colors.transparent,
                          ],
                          stops: const [0.0, 0.55, 1.0],
                        ),
                      ),
                    ),
                  ),
                ),

                // Child content on top if provided
                if (widget.child != null) widget.child!,
              ],
            );
          },
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
