import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../services/settings_service.dart';
import '../services/tilt_service.dart';

/// The 4-Layer Liquid Material Glass Container adhering to Section 2.2 of 05_UI_DESIGN_SPEC.md:
/// 1. Frosted Backdrop Blur / Clean Translucent Surface
/// 2. Tonal Glass Tint Fill (surfaceContainerHigh + surfaceTint)
/// 3. Diagonal Specular Rim Lighting (1.0px gradient border from top-left catch light to shadow edge)
/// 4. Dynamic 3D Parallax Tilt (Hardware accelerometer on Android, mouse tracking on Desktop)
class LiquidGlassContainer extends StatefulWidget {
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
  final bool enableTilt;

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
    this.enableTilt = false,
  });

  @override
  State<LiquidGlassContainer> createState() => _LiquidGlassContainerState();
}

class _LiquidGlassContainerState extends State<LiquidGlassContainer> {
  @override
  void initState() {
    super.initState();
    if (widget.enableTilt) {
      TiltService.instance.registerViewer();
    }
  }

  @override
  void didUpdateWidget(covariant LiquidGlassContainer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.enableTilt != oldWidget.enableTilt) {
      if (widget.enableTilt) {
        TiltService.instance.registerViewer();
      } else {
        TiltService.instance.unregisterViewer();
      }
    }
  }

  @override
  void dispose() {
    if (widget.enableTilt) {
      TiltService.instance.unregisterViewer();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final is3dMirror = SettingsService.instance.liquidGlass3dMirror;

    Widget buildGlassBody({double tiltX = 0.0, double tiltY = 0.0}) {
      Widget content = Container(
        width: widget.width,
        height: widget.height,
        padding: widget.padding,
        decoration: BoxDecoration(
          color: widget.tintOverride ??
              colorScheme.surfaceContainerHigh.withValues(alpha: widget.surfaceAlpha),
          borderRadius: BorderRadius.circular(widget.borderRadius),
          boxShadow: widget.boxShadow,
          border: Border.all(
            width: 1.0,
            color: widget.borderColor ?? Colors.transparent,
          ),
        ),
        child: widget.child,
      );

      // Apply diagonal specular rim lighting gradient and optional 3D mirror sheen
      content = CustomPaint(
        foregroundPainter: _SpecularRimPainter(
          borderRadius: widget.borderRadius,
          primaryTint: colorScheme.primary,
          is3dMirror: is3dMirror,
          tiltX: tiltX,
          tiltY: tiltY,
        ),
        child: content,
      );

      // Clip to rounded border with clean, high-performance translucent tonal surface tint
      content = ClipRRect(
        borderRadius: BorderRadius.circular(widget.borderRadius),
        child: content,
      );

      if (widget.onTap != null) {
        content = GestureDetector(
          onTap: widget.onTap,
          behavior: HitTestBehavior.opaque,
          child: content,
        );
      }

      if (widget.margin != EdgeInsets.zero) {
        content = Padding(
          padding: widget.margin,
          child: content,
        );
      }

      return content;
    }

    if (!widget.enableTilt || !is3dMirror) {
      return buildGlassBody();
    }

    // 3D Parallax Tilt Active:
    // Listens to accelerometer on Android (auto-suspended when out of view) or cursor on Desktop
    return ListenableBuilder(
      listenable: TiltService.instance,
      builder: (ctx, _) {
        final tiltX = TiltService.instance.tiltX;
        final tiltY = TiltService.instance.tiltY;

        Widget tilted = Transform(
          transform: Matrix4.identity()
            ..setEntry(3, 2, 0.0012) // Perspective depth
            ..rotateX(-tiltY * 0.12)  // Tilt on X axis from Y pitch
            ..rotateY(tiltX * 0.12),  // Tilt on Y axis from X roll
          alignment: Alignment.center,
          child: buildGlassBody(tiltX: tiltX, tiltY: tiltY),
        );

        // Desktop mouse tracking
        return MouseRegion(
          onHover: (event) {
            final box = ctx.findRenderObject() as RenderBox?;
            if (box != null && box.hasSize) {
              TiltService.instance.updatePointer(event.localPosition, box.size);
            }
          },
          onExit: (_) => TiltService.instance.resetPointer(),
          child: tilted,
        );
      },
    );
  }
}

/// CustomPainter for the 1.0px diagonal specular rim gradient border and optional 3D mirror sheen
class _SpecularRimPainter extends CustomPainter {
  final double borderRadius;
  final Color primaryTint;
  final bool is3dMirror;
  final double tiltX;
  final double tiltY;

  _SpecularRimPainter({
    required this.borderRadius,
    required this.primaryTint,
    this.is3dMirror = true,
    this.tiltX = 0.0,
    this.tiltY = 0.0,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Rect.fromLTWH(0.5, 0.5, size.width - 1.0, size.height - 1.0);
    final rrect = RRect.fromRectAndRadius(rect, Radius.circular(borderRadius));

    // Optional 3D Mirror Sheen & Depth Bevel
    if (is3dMirror) {
      // 1. Diagonal mirror reflection sheen across upper face (shifts with tilt)
      final sheenStart = Alignment(-1.0 + (tiltX * 0.4), -1.0 + (tiltY * 0.4));
      final sheenEnd = Alignment(1.0 + (tiltX * 0.4), 1.0 + (tiltY * 0.4));

      final sheenPaint = Paint()
        ..style = PaintingStyle.fill
        ..shader = LinearGradient(
          begin: sheenStart,
          end: sheenEnd,
          colors: [
            Colors.white.withValues(alpha: 0.14),
            Colors.white.withValues(alpha: 0.04),
            Colors.transparent,
          ],
          stops: const [0.0, 0.32, 0.70],
        ).createShader(rect);
      canvas.drawRRect(rrect, sheenPaint);

      // 2. Top bevel inner highlight
      final bevelHighlightPaint = Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.0
        ..shader = LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [
            Colors.white.withValues(alpha: 0.22),
            Colors.transparent,
          ],
          stops: const [0.0, 0.20],
        ).createShader(rect);
      canvas.drawRRect(rrect, bevelHighlightPaint);

      // 3. Bottom inner depth shadow ledge
      final shadowLedgePaint = Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.2
        ..shader = LinearGradient(
          begin: Alignment.bottomCenter,
          end: Alignment.topCenter,
          colors: [
            Colors.black.withValues(alpha: 0.20),
            Colors.transparent,
          ],
          stops: const [0.0, 0.18],
        ).createShader(rect);
      canvas.drawRRect(rrect, shadowLedgePaint);
    }

    // Specular Rim Border
    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.0
      ..shader = LinearGradient(
        begin: Alignment(-1.0 + (tiltX * 0.2), -1.0 + (tiltY * 0.2)),
        end: Alignment(1.0 + (tiltX * 0.2), 1.0 + (tiltY * 0.2)),
        colors: [
          Colors.white.withValues(alpha: is3dMirror ? 0.24 : 0.14),
          primaryTint.withValues(alpha: is3dMirror ? 0.12 : 0.06),
          Colors.white.withValues(alpha: is3dMirror ? 0.04 : 0.02),
        ],
        stops: const [0.0, 0.45, 1.0],
      ).createShader(rect);

    canvas.drawRRect(rrect, paint);
  }

  @override
  bool shouldRepaint(covariant _SpecularRimPainter oldDelegate) {
    return oldDelegate.borderRadius != borderRadius ||
        oldDelegate.primaryTint != primaryTint ||
        oldDelegate.is3dMirror != is3dMirror ||
        oldDelegate.tiltX != tiltX ||
        oldDelegate.tiltY != tiltY;
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
        final scheme = colorScheme ?? Theme.of(context).colorScheme;

        return Stack(
          fit: StackFit.expand,
          children: [
            Container(color: scheme.surface),
            ?child,
          ],
        );
      },
    );
  }
}
