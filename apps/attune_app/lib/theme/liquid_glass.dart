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
/// Dynamically sways and breathes in lockstep with the active track's BPM and musical rhythm,
/// featuring organic fluid lavalamp liquid light globs responding to beat kicks.
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

  void _updateBeatTempo(double rawBpm, bool isPlaying, bool isEnabled) {
    if (!isEnabled || !isPlaying) {
      if (_beatController.isAnimating) {
        _beatController.stop();
      }
      if (!isEnabled && _driftController.isAnimating) {
        _driftController.stop();
      }
      return;
    }

    if (!_driftController.isAnimating) {
      _driftController.repeat(reverse: true);
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
        final isEnabled = controller.livingBeatMeshEnabled;
        final bpm = controller.currentTrack?.bpm ?? 120.0;
        final isPlaying = controller.isPlaying;
        final scheme = widget.colorScheme ?? Theme.of(context).colorScheme;

        _updateBeatTempo(bpm, isPlaying, isEnabled);

        if (!isEnabled) {
          // Disabled state: Calm, sleek, non-distracting dark frosted surface
          return Stack(
            fit: StackFit.expand,
            children: [
              Container(color: scheme.surface),
              Positioned.fill(
                child: Container(
                  decoration: BoxDecoration(
                    gradient: RadialGradient(
                      center: Alignment.center,
                      radius: 1.4,
                      colors: [
                        scheme.primary.withValues(alpha: 0.03),
                        Colors.transparent,
                      ],
                    ),
                  ),
                ),
              ),
              if (widget.child != null) widget.child!,
            ],
          );
        }

        return AnimatedBuilder(
          animation: Listenable.merge([_driftController, _beatController]),
          builder: (context, _) {
            final t = _driftController.value;
            final beatPulse = isPlaying ? Curves.easeInOutSine.transform(_beatController.value) : 0.0;

            return Stack(
              fit: StackFit.expand,
              children: [
                // Deep base surface
                Container(color: scheme.surface),

                // Fluid Lavalamp Liquid Light Globs
                CustomPaint(
                  painter: _LavalampGlobsPainter(
                    t: t,
                    beatPulse: beatPulse,
                    primary: scheme.primary,
                    secondary: scheme.secondary,
                    tertiary: scheme.tertiary,
                  ),
                  size: Size.infinite,
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

class _LavalampGlobsPainter extends CustomPainter {
  final double t;
  final double beatPulse;
  final Color primary;
  final Color secondary;
  final Color tertiary;

  _LavalampGlobsPainter({
    required this.t,
    required this.beatPulse,
    required this.primary,
    required this.secondary,
    required this.tertiary,
  });

  @override
  void paint(Canvas canvas, Size size) {
    if (size.width <= 0 || size.height <= 0) return;

    final w = size.width;
    final h = size.height;

    // Fluid lavalamp phase variables
    final phi = t * math.pi * 2;
    final beatScale = 1.0 + (0.16 * beatPulse);
    final beatAlpha = 1.0 + (0.35 * beatPulse);
    // Squash on kick: slightly wider, slightly shorter
    final squashX = 1.0 + (0.08 * beatPulse);
    final squashY = 1.0 - (0.05 * beatPulse);

    // 1. Bottom Heat Reservoir (Molten pool of liquid wax at base)
    final poolH = h * 0.32;
    final poolY = h - (poolH * 0.35);
    final poolSway = math.sin(phi * 0.5) * (w * 0.05);
    _drawGlob(
      canvas: canvas,
      center: Offset((w * 0.5) + poolSway, poolY),
      radiusX: (w * 0.75) * squashX * beatScale,
      radiusY: (poolH * 0.8) * squashY * beatScale,
      color: primary,
      alphaCore: 0.35 * beatAlpha,
      alphaMid: 0.18 * beatAlpha,
      alphaEdge: 0.05,
    );

    // 2. Glob 1: Large Primary Rising Buoyant Droplet
    // Vertical non-linear ascent & descent
    final g1Y = h * (0.50 + 0.34 * math.sin(phi + 0.25 * math.cos(phi * 2)));
    final g1X = w * (0.32 + 0.14 * math.cos(phi * 0.8));
    final g1VelY = (math.cos(phi) * 0.3).abs();
    final g1StretchY = (1.0 + g1VelY * 0.6) * squashY;
    final g1StretchX = (1.0 / math.sqrt(1.0 + g1VelY * 0.6)) * squashX;
    _drawGlob(
      canvas: canvas,
      center: Offset(g1X, g1Y),
      radiusX: (w * 0.30) * g1StretchX * beatScale,
      radiusY: (w * 0.30) * g1StretchY * beatScale,
      color: primary,
      alphaCore: 0.40 * beatAlpha,
      alphaMid: 0.22 * beatAlpha,
      alphaEdge: 0.06,
    );

    // 3. Glob 2: Vibrant Secondary Sinking / Drifting Droplet
    final g2Y = h * (0.45 - 0.32 * math.sin(phi * 1.25 + 1.2));
    final g2X = w * (0.68 + 0.16 * math.sin(phi * 0.95 + 0.4));
    final g2VelY = (math.cos(phi * 1.25 + 1.2) * 0.3).abs();
    final g2StretchY = (1.0 + g2VelY * 0.5) * squashY;
    final g2StretchX = (1.0 / math.sqrt(1.0 + g2VelY * 0.5)) * squashX;
    _drawGlob(
      canvas: canvas,
      center: Offset(g2X, g2Y),
      radiusX: (w * 0.32) * g2StretchX * beatScale,
      radiusY: (w * 0.32) * g2StretchY * beatScale,
      color: secondary,
      alphaCore: 0.36 * beatAlpha,
      alphaMid: 0.20 * beatAlpha,
      alphaEdge: 0.05,
    );

    // 4. Glob 3: Floating Mid-Tone Tertiary Droplet (Center right harmonic accent)
    final g3Y = h * (0.35 + 0.28 * math.cos(phi * 0.7 + 2.0));
    final g3X = w * (0.48 + 0.22 * math.sin(phi * 1.4));
    _drawGlob(
      canvas: canvas,
      center: Offset(g3X, g3Y),
      radiusX: (w * 0.22) * squashX * beatScale,
      radiusY: (w * 0.25) * squashY * beatScale,
      color: tertiary,
      alphaCore: 0.30 * beatAlpha,
      alphaMid: 0.15 * beatAlpha,
      alphaEdge: 0.04,
    );

    // 5. Glob 4: Tiny Satellite Ascending Droplet (Pinching off and rising)
    final g4Y = h * (0.28 + 0.25 * math.sin(phi * 1.6 + 3.0));
    final g4X = w * (0.25 + 0.18 * math.cos(phi * 1.1 + 1.0));
    _drawGlob(
      canvas: canvas,
      center: Offset(g4X, g4Y),
      radiusX: (w * 0.14) * squashX * beatScale,
      radiusY: (w * 0.16) * squashY * beatScale,
      color: primary,
      alphaCore: 0.32 * beatAlpha,
      alphaMid: 0.16 * beatAlpha,
      alphaEdge: 0.04,
    );

    // 6. Top Condenser Pool (Cool liquid collecting at top)
    final topPoolH = h * 0.22;
    final topPoolY = -(topPoolH * 0.2);
    final topSway = math.cos(phi * 0.6) * (w * 0.06);
    _drawGlob(
      canvas: canvas,
      center: Offset((w * 0.5) + topSway, topPoolY),
      radiusX: (w * 0.70) * squashX * beatScale,
      radiusY: (topPoolH * 0.75) * squashY * beatScale,
      color: secondary,
      alphaCore: 0.25 * beatAlpha,
      alphaMid: 0.12 * beatAlpha,
      alphaEdge: 0.03,
    );
  }

  void _drawGlob({
    required Canvas canvas,
    required Offset center,
    required double radiusX,
    required double radiusY,
    required Color color,
    required double alphaCore,
    required double alphaMid,
    required double alphaEdge,
  }) {
    if (radiusX <= 0 || radiusY <= 0) return;

    final rect = Rect.fromCenter(
      center: center,
      width: radiusX * 2,
      height: radiusY * 2,
    );

    final paint = Paint()
      ..shader = RadialGradient(
        colors: [
          color.withValues(alpha: alphaCore.clamp(0.0, 1.0)),
          color.withValues(alpha: alphaMid.clamp(0.0, 1.0)),
          color.withValues(alpha: alphaEdge.clamp(0.0, 1.0)),
          Colors.transparent,
        ],
        stops: const [0.0, 0.40, 0.75, 1.0],
      ).createShader(rect);

    canvas.drawOval(rect, paint);
  }

  @override
  bool shouldRepaint(covariant _LavalampGlobsPainter oldDelegate) {
    return oldDelegate.t != t ||
        oldDelegate.beatPulse != beatPulse ||
        oldDelegate.primary != primary ||
        oldDelegate.secondary != secondary ||
        oldDelegate.tertiary != tertiary;
  }
}
