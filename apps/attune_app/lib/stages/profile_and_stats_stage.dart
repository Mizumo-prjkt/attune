import 'package:flutter/material.dart';
import '../services/audio_storage_service.dart';
import '../services/engine_bridge.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// ProfileAndStatsStage: Replaces the old library slot with user profile,
/// listening usage statistics, app settings, and about / licenses.
class ProfileAndStatsStage extends StatelessWidget {
  final VoidCallback? onRerunSetup;

  const ProfileAndStatsStage({super.key, this.onRerunSetup});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final tracks = controller.libraryTracks;

        // Calculate statistics
        final totalTracks = tracks.length;
        final totalSeconds = tracks.fold<int>(0, (sum, t) => sum + t.duration.inSeconds);
        final totalHours = (totalSeconds / 3600).toStringAsFixed(1);
        final rockCount = tracks.where((t) => t.genre == 'Rock').length;
        final popCount = tracks.where((t) => t.genre == 'Pop').length;
        final electronicCount = tracks.where((t) => t.genre == 'Electronic').length;
        final losslessCount = tracks.where((t) => t.isLossless).length;

        final engineVersion = AttuneEngineBridge.instance.version;

        return ListView(
          physics: const BouncingScrollPhysics(),
          padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 16.0),
          children: [
            // 1. Profile Header Card
            LiquidGlassContainer(
              padding: const EdgeInsets.all(20),
              borderRadius: 24,
              surfaceAlpha: 0.16,
              child: Row(
                children: [
                  Container(
                    width: 64,
                    height: 64,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      gradient: LinearGradient(
                        colors: [colorScheme.primary, colorScheme.tertiary],
                        begin: Alignment.topLeft,
                        end: Alignment.bottomRight,
                      ),
                      boxShadow: [
                        BoxShadow(
                          color: colorScheme.primary.withValues(alpha: 0.4),
                          blurRadius: 16,
                          spreadRadius: 1,
                        ),
                      ],
                    ),
                    child: Center(
                      child: Text(
                        'M',
                        style: AttuneTypography.soraHeading(
                          fontSize: 28,
                          fontWeight: FontWeight.bold,
                          color: Colors.white,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Miku (Audiophile)',
                          style: AttuneTypography.soraHeading(
                            fontSize: 18,
                            fontWeight: FontWeight.w700,
                            color: colorScheme.onSurface,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Row(
                          children: [
                            Container(
                              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                              decoration: BoxDecoration(
                                color: colorScheme.primary.withValues(alpha: 0.2),
                                borderRadius: BorderRadius.circular(8),
                              ),
                              child: Text(
                                'Attune Hi-Fi Engine',
                                style: AttuneTypography.soraBadge(
                                  color: colorScheme.primary,
                                  fontSize: 10,
                                ),
                              ),
                            ),
                            const SizedBox(width: 6),
                            Container(
                              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                              decoration: BoxDecoration(
                                color: colorScheme.tertiary.withValues(alpha: 0.2),
                                borderRadius: BorderRadius.circular(8),
                              ),
                              child: Text(
                                '$losslessCount Lossless',
                                style: AttuneTypography.soraBadge(
                                  color: colorScheme.tertiary,
                                  fontSize: 10,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),

            const SizedBox(height: 24),

            // 2. Statistics & Usage Metrics
            Text(
              'LISTENING USAGE & STATISTICS',
              style: AttuneTypography.soraBadge(color: colorScheme.primary),
            ),
            const SizedBox(height: 12),

            Row(
              children: [
                Expanded(
                  child: _buildMetricTile(
                    label: 'TOTAL TRACKS',
                    value: '$totalTracks',
                    subtitle: 'Ingested locally',
                    colorScheme: colorScheme,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: _buildMetricTile(
                    label: 'PLAYTIME',
                    value: '$totalHours hrs',
                    subtitle: 'Audio library',
                    colorScheme: colorScheme,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: _buildMetricTile(
                    label: 'TRANSITION ENGINE',
                    value: '1.4s 4-Beat',
                    subtitle: 'Riff Handoff locked',
                    colorScheme: colorScheme,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: _buildMetricTile(
                    label: 'DAC ADAPTATION',
                    value: '1.000x Hermite',
                    subtitle: '44.1k -> 48k native',
                    colorScheme: colorScheme,
                  ),
                ),
              ],
            ),

            const SizedBox(height: 16),

            // Genre Distribution Chips
            LiquidGlassContainer(
              padding: const EdgeInsets.all(16),
              borderRadius: 20,
              surfaceAlpha: 0.12,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'GENRE DISTRIBUTION',
                    style: AttuneTypography.soraBadge(
                      color: colorScheme.onSurface.withValues(alpha: 0.6),
                      fontSize: 11,
                    ),
                  ),
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      _buildGenreChip('Rock / Punk', rockCount, colorScheme.primary),
                      _buildGenreChip('Pop / J-Pop', popCount, colorScheme.tertiary),
                      _buildGenreChip('Electronic / Dance', electronicCount, const Color(0xFF14B8A6)),
                      _buildGenreChip('Other / Soulseek', totalTracks - (rockCount + popCount + electronicCount), Colors.purpleAccent),
                    ],
                  ),
                ],
              ),
            ),

            const SizedBox(height: 24),

            // 3. Settings & Library Management
            Text(
              'SETTINGS & ACTIONS',
              style: AttuneTypography.soraBadge(color: colorScheme.primary),
            ),
            const SizedBox(height: 12),

            LiquidGlassContainer(
              padding: const EdgeInsets.symmetric(vertical: 6),
              borderRadius: 20,
              surfaceAlpha: 0.12,
              child: Column(
                children: [
                  ListTile(
                    leading: Icon(Icons.tune_rounded, color: colorScheme.primary),
                    title: Text(
                      'Audio & DSP Settings',
                      style: AttuneTypography.soraSubheading(fontSize: 14, color: colorScheme.onSurface),
                    ),
                    subtitle: Text(
                      'Configure JIT Mode, EQ curves, and reverb wash',
                      style: AttuneTypography.jakartaBody(fontSize: 12, color: colorScheme.onSurface.withValues(alpha: 0.5)),
                    ),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () => PlatformNavigation.openSettings(context),
                  ),
                  Divider(color: Colors.white.withValues(alpha: 0.08), height: 1),
                  ListTile(
                    leading: Icon(Icons.refresh_rounded, color: colorScheme.primary),
                    title: Text(
                      'Refresh Music Library',
                      style: AttuneTypography.soraSubheading(fontSize: 14, color: colorScheme.onSurface),
                    ),
                    subtitle: Text(
                      'Re-scan local storage and update library cache',
                      style: AttuneTypography.jakartaBody(fontSize: 12, color: colorScheme.onSurface.withValues(alpha: 0.5)),
                    ),
                    trailing: controller.isScanning
                        ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
                        : const Icon(Icons.chevron_right_rounded),
                    onTap: controller.isScanning
                        ? null
                        : () => controller.scanLocalAudioDirectories(forceRefresh: true),
                  ),
                  Divider(color: Colors.white.withValues(alpha: 0.08), height: 1),
                  ListTile(
                    leading: Icon(Icons.restart_alt_rounded, color: colorScheme.primary),
                    title: Text(
                      'Startup Setup Wizard',
                      style: AttuneTypography.soraSubheading(fontSize: 14, color: colorScheme.onSurface),
                    ),
                    subtitle: Text(
                      'Re-open the directory configuration welcome page',
                      style: AttuneTypography.jakartaBody(fontSize: 12, color: colorScheme.onSurface.withValues(alpha: 0.5)),
                    ),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () {
                      AudioStorageService.instance.setSetupCompleted(false);
                      onRerunSetup?.call();
                    },
                  ),
                ],
              ),
            ),

            const SizedBox(height: 24),

            // 4. About & Open-Source Licenses
            Text(
              'ABOUT & LEGAL',
              style: AttuneTypography.soraBadge(color: colorScheme.primary),
            ),
            const SizedBox(height: 12),

            LiquidGlassContainer(
              padding: const EdgeInsets.symmetric(vertical: 6),
              borderRadius: 20,
              surfaceAlpha: 0.12,
              child: Column(
                children: [
                  ListTile(
                    leading: Icon(Icons.info_outline_rounded, color: colorScheme.primary),
                    title: Text(
                      'About Attune',
                      style: AttuneTypography.soraSubheading(fontSize: 14, color: colorScheme.onSurface),
                    ),
                    subtitle: Text(
                      'Version 0.1.0 (Native Core v$engineVersion)',
                      style: AttuneTypography.jakartaBody(fontSize: 12, color: colorScheme.onSurface.withValues(alpha: 0.5)),
                    ),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () {
                      showAboutDialog(
                        context: context,
                        applicationName: 'Attune',
                        applicationVersion: 'v0.1.0',
                        applicationIcon: Container(
                          width: 48,
                          height: 48,
                          padding: const EdgeInsets.all(6),
                          decoration: BoxDecoration(color: colorScheme.primary.withValues(alpha: 0.15), shape: BoxShape.circle),
                          child: Image.asset(
                            'assets/icon/icon_transparent.png',
                            fit: BoxFit.contain,
                          ),
                        ),
                        children: [
                          const SizedBox(height: 12),
                          const Text(
                            'Attune is an open-source, intelligent music player with a DJ-grade, audio-profile-driven transition and remixing engine.\n\nBuilt with Rust, Signalsmith Stretch, Symphonia, Hound, Cpal, and Flutter Liquid Material Glass.',
                          ),
                        ],
                      );
                    },
                  ),
                  Divider(color: Colors.white.withValues(alpha: 0.08), height: 1),
                  ListTile(
                    leading: Icon(Icons.description_outlined, color: colorScheme.primary),
                    title: Text(
                      'Open-Source Licenses',
                      style: AttuneTypography.soraSubheading(fontSize: 14, color: colorScheme.onSurface),
                    ),
                    subtitle: Text(
                      'View third-party software notices and licenses',
                      style: AttuneTypography.jakartaBody(fontSize: 12, color: colorScheme.onSurface.withValues(alpha: 0.5)),
                    ),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () {
                      showLicensePage(
                        context: context,
                        applicationName: 'Attune',
                        applicationVersion: 'v0.1.0',
                        applicationIcon: Container(
                          width: 48,
                          height: 48,
                          padding: const EdgeInsets.all(6),
                          decoration: BoxDecoration(color: colorScheme.primary.withValues(alpha: 0.15), shape: BoxShape.circle),
                          child: Image.asset(
                            'assets/icon/icon_transparent.png',
                            fit: BoxFit.contain,
                          ),
                        ),
                      );
                    },
                  ),
                ],
              ),
            ),

            const SizedBox(height: 90),
          ],
        );
      },
    );
  }

  Widget _buildMetricTile({
    required String label,
    required String value,
    required String subtitle,
    required ColorScheme colorScheme,
  }) {
    return LiquidGlassContainer(
      padding: const EdgeInsets.all(16),
      borderRadius: 18,
      surfaceAlpha: 0.12,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: AttuneTypography.soraBadge(
              fontSize: 10,
              color: colorScheme.onSurface.withValues(alpha: 0.55),
            ),
          ),
          const SizedBox(height: 8),
          Text(
            value,
            style: AttuneTypography.soraHeading(
              fontSize: 18,
              fontWeight: FontWeight.w700,
              color: colorScheme.primary,
            ),
          ),
          const SizedBox(height: 2),
          Text(
            subtitle,
            style: AttuneTypography.jakartaBody(
              fontSize: 11,
              color: colorScheme.onSurface.withValues(alpha: 0.45),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildGenreChip(String label, int count, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.18),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.4), width: 1),
      ),
      child: Text(
        '$label: $count',
        style: AttuneTypography.soraBadge(
          color: color,
          fontSize: 11,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}
