import 'dart:math' as math;
import 'package:flutter/material.dart';
import '../services/settings_service.dart';
import '../l10n/translation_core.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Interactive Visual Skin & Theme Picker stage.
/// Replaces plain switch flags with a dedicated visual interface showing live mockups
/// of each Now Playing skin and glass presentation style so listeners can make an informed choice.
class NowPlayingSkinPickerStage extends StatelessWidget {
  final VoidCallback? onDismiss;

  const NowPlayingSkinPickerStage({
    super.key,
    this.onDismiss,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final settings = SettingsService.instance;

    return ListenableBuilder(
      listenable: settings,
      builder: (context, _) {
        final currentSkin = settings.nowPlayingSkin;
        final is3dMirror = settings.pendingLiquidGlass3dMirror;

        return Scaffold(
          backgroundColor: Colors.transparent,
          body: SafeArea(
            child: ListView(
              physics: const BouncingScrollPhysics(),
              padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 16.0),
              children: [
                // Top Header with Back / Dismiss Button
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    GestureDetector(
                      onTap: () {
                        if (onDismiss != null) {
                          onDismiss!();
                        } else if (Navigator.canPop(context)) {
                          Navigator.of(context).pop();
                        }
                      },
                      child: LiquidGlassContainer(
                        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                        borderRadius: 16,
                        surfaceAlpha: 0.16,
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            const Icon(Icons.arrow_back_rounded, size: 18),
                            const SizedBox(width: 6),
                            Text(
                              context.tr('nav_settings'),
                              style: AttuneTypography.soraBadge(color: colorScheme.onSurface),
                            ),
                          ],
                        ),
                      ),
                    ),
                    LiquidGlassContainer(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
                      borderRadius: 16,
                      surfaceAlpha: 0.16,
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(Icons.palette_rounded, size: 16, color: colorScheme.primary),
                          const SizedBox(width: 6),
                          Text(
                            'SKIN LAB',
                            style: AttuneTypography.soraBadge(color: colorScheme.primary, fontSize: 10),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),

                const SizedBox(height: 20),

                // Main Title & Subtitle
                Text(
                  'Now Playing Visual Skins',
                  style: AttuneTypography.soraHeading(
                    fontSize: 22,
                    fontWeight: FontWeight.w700,
                    color: colorScheme.onSurface,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  'Select the visual presentation for hero artwork, lyrics choreography, and controls.',
                  style: AttuneTypography.jakartaBody(
                    color: colorScheme.onSurface.withValues(alpha: 0.65),
                    fontSize: 13,
                  ),
                ),

                const SizedBox(height: 24),

                // SECTION 1: NOW PLAYING SKINS
                ...NowPlayingSkin.values.map((skin) {
                  final isSelected = skin == currentSkin;
                  return Padding(
                    padding: const EdgeInsets.only(bottom: 16.0),
                    child: _buildSkinCard(
                      skin: skin,
                      isSelected: isSelected,
                      colorScheme: colorScheme,
                      onTap: () => settings.setNowPlayingSkin(skin),
                    ),
                  );
                }),

                const SizedBox(height: 20),

                // SECTION 2: GLASS PRESENTATION (3D Specular vs 2D Minimal)
                Text(
                  'GLASS MATERIAL PRESENTATION',
                  style: AttuneTypography.soraBadge(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  'Visual rendering mode for liquid glass depth and ambient reflections.',
                  style: AttuneTypography.jakartaBody(
                    color: colorScheme.onSurface.withValues(alpha: 0.60),
                    fontSize: 12,
                  ),
                ),

                const SizedBox(height: 14),

                Row(
                  children: [
                    Expanded(
                      child: _buildGlassModeCard(
                        title: '3D Specular Mirror',
                        subtitle: 'Multi-angle specular highlights with dynamic physics tilt',
                        is3D: true,
                        isSelected: is3dMirror,
                        colorScheme: colorScheme,
                        onTap: () => settings.setPendingLiquidGlass3dMirror(true),
                      ),
                    ),
                    const SizedBox(width: 14),
                    Expanded(
                      child: _buildGlassModeCard(
                        title: '2D Minimal Flat',
                        subtitle: 'Pure frosted matte backdrop blur with high performance',
                        is3D: false,
                        isSelected: !is3dMirror,
                        colorScheme: colorScheme,
                        onTap: () => settings.setPendingLiquidGlass3dMirror(false),
                      ),
                    ),
                  ],
                ),

                if (settings.restartRequiredForGlass) ...[
                  const SizedBox(height: 14),
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Colors.amber.withValues(alpha: 0.15),
                      borderRadius: BorderRadius.circular(14),
                      border: Border.all(color: Colors.amber.withValues(alpha: 0.3)),
                    ),
                    child: Row(
                      children: [
                        const Icon(Icons.restart_alt_rounded, color: Colors.amberAccent, size: 20),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Text(
                            context.tr('settings_restart_notice'),
                            style: AttuneTypography.jakartaBodySmall(color: Colors.amberAccent),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],

                const SizedBox(height: 40),
              ],
            ),
          ),
        );
      },
    );
  }

  Widget _buildSkinCard({
    required NowPlayingSkin skin,
    required bool isSelected,
    required ColorScheme colorScheme,
    required VoidCallback onTap,
  }) {
    return GestureDetector(
      onTap: onTap,
      child: LiquidGlassContainer(
        padding: const EdgeInsets.all(18),
        borderRadius: 24,
        surfaceAlpha: isSelected ? 0.24 : 0.10,
        borderColor: isSelected ? colorScheme.primary.withValues(alpha: 0.6) : null,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header Row with Title, Badge & Radio Check
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.all(8),
                        decoration: BoxDecoration(
                          color: isSelected
                              ? colorScheme.primary.withValues(alpha: 0.25)
                              : Colors.white.withValues(alpha: 0.08),
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: Icon(
                          _getSkinIcon(skin),
                          size: 20,
                          color: isSelected ? colorScheme.primary : Colors.white70,
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              skin.displayName,
                              style: AttuneTypography.soraSubheading(
                                fontSize: 16,
                                fontWeight: FontWeight.w700,
                                color: isSelected ? colorScheme.primary : colorScheme.onSurface,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
                Container(
                  width: 24,
                  height: 24,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: isSelected ? colorScheme.primary : Colors.transparent,
                    border: Border.all(
                      color: isSelected ? colorScheme.primary : Colors.white30,
                      width: 2,
                    ),
                  ),
                  child: isSelected
                      ? const Icon(Icons.check_rounded, size: 16, color: Colors.black)
                      : null,
                ),
              ],
            ),

            const SizedBox(height: 12),

            // Live Visual Mockup Preview
            Container(
              height: 130,
              width: double.infinity,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(18),
                color: Colors.black.withValues(alpha: 0.35),
                border: Border.all(color: Colors.white.withValues(alpha: 0.06)),
              ),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(18),
                child: _buildSkinMockup(skin, colorScheme),
              ),
            ),

            const SizedBox(height: 12),

            // Description
            Text(
              skin.description,
              style: AttuneTypography.jakartaBody(
                color: colorScheme.onSurface.withValues(alpha: 0.65),
                fontSize: 12.5,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildSkinMockup(NowPlayingSkin skin, ColorScheme colorScheme) {
    switch (skin) {
      case NowPlayingSkin.defaultSkin:
        // Mockup: Standard Liquid Glass Player
        return Stack(
          alignment: Alignment.center,
          children: [
            // Ambient glowing pulse
            Positioned(
              left: 40,
              top: 20,
              child: Container(
                width: 70,
                height: 70,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: colorScheme.primary.withValues(alpha: 0.35),
                ),
              ),
            ),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                // Squircle Cover
                Container(
                  width: 80,
                  height: 80,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(20),
                    gradient: LinearGradient(
                      colors: [colorScheme.primary, colorScheme.secondary],
                      begin: Alignment.topLeft,
                      end: Alignment.bottomRight,
                    ),
                    boxShadow: [
                      BoxShadow(
                        color: colorScheme.primary.withValues(alpha: 0.3),
                        blurRadius: 12,
                        offset: const Offset(0, 4),
                      ),
                    ],
                  ),
                  child: const Center(
                    child: Icon(Icons.music_note_rounded, color: Colors.white, size: 32),
                  ),
                ),
                const SizedBox(width: 20),
                // Compact Controls Preview
                Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Container(width: 85, height: 8, decoration: BoxDecoration(color: Colors.white, borderRadius: BorderRadius.circular(4))),
                    const SizedBox(height: 6),
                    Container(width: 55, height: 6, decoration: BoxDecoration(color: Colors.white38, borderRadius: BorderRadius.circular(3))),
                    const SizedBox(height: 12),
                    Row(
                      children: [
                        Icon(Icons.skip_previous_rounded, size: 16, color: colorScheme.onSurface),
                        const SizedBox(width: 8),
                        Container(
                          padding: const EdgeInsets.all(6),
                          decoration: BoxDecoration(
                            shape: BoxShape.circle,
                            color: colorScheme.primary,
                          ),
                          child: const Icon(Icons.play_arrow_rounded, size: 14, color: Colors.black),
                        ),
                        const SizedBox(width: 8),
                        Icon(Icons.skip_next_rounded, size: 16, color: colorScheme.onSurface),
                      ],
                    ),
                  ],
                ),
              ],
            ),
          ],
        );

      case NowPlayingSkin.materialYouFreeshapes:
        // Mockup: Organic M3 Shapes morphing with lyrics silhouette
        return Stack(
          alignment: Alignment.center,
          children: [
            // Faint silhouette backdrop
            Positioned(
              right: 30,
              child: Opacity(
                opacity: 0.15,
                child: Text(
                  '♪ I will always love you ♪\n♪ Oh baby sweet fantasy ♪',
                  style: AttuneTypography.soraTitleLarge(color: Colors.white, fontSize: 13),
                  textAlign: TextAlign.right,
                ),
              ),
            ),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                // 8-petal scalloped flower shape
                CustomPaint(
                  size: const Size(82, 82),
                  painter: _MiniFreeshapePainter(
                    color1: const Color(0xFFFF6584),
                    color2: const Color(0xFF6C63FF),
                    petals: 8,
                  ),
                ),
                const SizedBox(width: 14),
                // 4-leaf organic clover
                CustomPaint(
                  size: const Size(60, 60),
                  painter: _MiniFreeshapePainter(
                    color1: const Color(0xFF43E97B),
                    color2: const Color(0xFF38F9D7),
                    petals: 4,
                  ),
                ),
                const SizedBox(width: 14),
                // Morphing badge pill
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                  decoration: BoxDecoration(
                    color: colorScheme.primary.withValues(alpha: 0.20),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: colorScheme.primary.withValues(alpha: 0.3)),
                  ),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.auto_awesome_rounded, size: 14, color: colorScheme.primary),
                      const SizedBox(height: 4),
                      Text(
                        'Morphing',
                        style: AttuneTypography.soraBadge(fontSize: 9, color: colorScheme.primary),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ],
        );

      case NowPlayingSkin.normalSquareBig:
        // Mockup: Expansive square artwork with side subtitles pill
        return Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            // Edge-to-edge square artwork
            Container(
              width: 95,
              height: 95,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(16),
                gradient: const LinearGradient(
                  colors: [Color(0xFF2C3E50), Color(0xFF4CA1AF)],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
                boxShadow: const [
                  BoxShadow(
                    color: Colors.black45,
                    blurRadius: 10,
                    offset: Offset(0, 4),
                  ),
                ],
              ),
              child: Stack(
                children: [
                  const Center(
                    child: Icon(Icons.album_rounded, size: 40, color: Colors.white70),
                  ),
                  Positioned(
                    right: 6,
                    bottom: 6,
                    child: Container(
                      padding: const EdgeInsets.all(4),
                      decoration: BoxDecoration(
                        color: Colors.black54,
                        borderRadius: BorderRadius.circular(6),
                      ),
                      child: const Icon(Icons.subtitles_rounded, size: 12, color: Colors.white),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 18),
            // Clean side typography & subtitle access
            Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Container(width: 90, height: 8, decoration: BoxDecoration(color: Colors.white, borderRadius: BorderRadius.circular(4))),
                const SizedBox(height: 6),
                Container(width: 60, height: 6, decoration: BoxDecoration(color: Colors.white38, borderRadius: BorderRadius.circular(3))),
                const SizedBox(height: 10),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: Colors.white.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.subtitles_rounded, size: 12, color: colorScheme.primary),
                      const SizedBox(width: 4),
                      Text('Subtitles', style: AttuneTypography.soraBadge(fontSize: 9, color: colorScheme.onSurface)),
                    ],
                  ),
                ),
              ],
            ),
          ],
        );
    }
  }

  Widget _buildGlassModeCard({
    required String title,
    required String subtitle,
    required bool is3D,
    required bool isSelected,
    required ColorScheme colorScheme,
    required VoidCallback onTap,
  }) {
    return GestureDetector(
      onTap: onTap,
      child: LiquidGlassContainer(
        padding: const EdgeInsets.all(14),
        borderRadius: 18,
        surfaceAlpha: isSelected ? 0.22 : 0.08,
        borderColor: isSelected ? colorScheme.primary.withValues(alpha: 0.6) : null,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Icon(
                  is3D ? Icons.view_in_ar_rounded : Icons.layers_outlined,
                  size: 22,
                  color: isSelected ? colorScheme.primary : Colors.white70,
                ),
                Container(
                  width: 20,
                  height: 20,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: isSelected ? colorScheme.primary : Colors.transparent,
                    border: Border.all(
                      color: isSelected ? colorScheme.primary : Colors.white30,
                      width: 2,
                    ),
                  ),
                  child: isSelected
                      ? const Icon(Icons.check_rounded, size: 14, color: Colors.black)
                      : null,
                ),
              ],
            ),
            const SizedBox(height: 10),
            Text(
              title,
              style: AttuneTypography.soraSubheading(
                fontSize: 13,
                fontWeight: FontWeight.w700,
                color: isSelected ? colorScheme.primary : colorScheme.onSurface,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              subtitle,
              style: AttuneTypography.jakartaBodySmall(
                color: colorScheme.onSurface.withValues(alpha: 0.6),
                fontSize: 11,
              ),
            ),
          ],
        ),
      ),
    );
  }

  IconData _getSkinIcon(NowPlayingSkin skin) {
    switch (skin) {
      case NowPlayingSkin.defaultSkin:
        return Icons.water_drop_rounded;
      case NowPlayingSkin.materialYouFreeshapes:
        return Icons.auto_awesome_rounded;
      case NowPlayingSkin.normalSquareBig:
        return Icons.crop_square_rounded;
    }
  }
}

/// Miniature custom painter to draw sample M3 expressive free-shapes in the picker mockup
class _MiniFreeshapePainter extends CustomPainter {
  final Color color1;
  final Color color2;
  final int petals;

  _MiniFreeshapePainter({
    required this.color1,
    required this.color2,
    required this.petals,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = size.width / 2;
    final path = Path();

    const steps = 120;
    for (int i = 0; i <= steps; i++) {
      final theta = (i / steps) * 2 * math.pi;
      final mod = (petals == 8)
          ? (0.84 + 0.16 * math.cos(petals * theta))
          : (0.78 + 0.22 * math.cos(petals * theta));
      final r = radius * mod;
      final x = center.dx + r * math.cos(theta);
      final y = center.dy + r * math.sin(theta);
      if (i == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }
    path.close();

    final paint = Paint()
      ..shader = LinearGradient(
        colors: [color1, color2],
        begin: Alignment.topLeft,
        end: Alignment.bottomRight,
      ).createShader(Rect.fromLTWH(0, 0, size.width, size.height))
      ..style = PaintingStyle.fill;

    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(covariant _MiniFreeshapePainter oldDelegate) =>
      oldDelegate.color1 != color1 || oldDelegate.color2 != color2 || oldDelegate.petals != petals;
}
