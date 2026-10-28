import 'package:flutter/material.dart';
import 'about_stage.dart';
import '../models/track_item.dart';
import '../services/audio_storage_service.dart';
import '../services/engine_bridge.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../services/settings_service.dart';
import '../services/update_service.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// ProfileAndStatsStage: Displays real user profile, dynamically estimated
/// listening usage statistics, real hardware/DSP telemetry, genre distribution,
/// audio fidelity breakdown, app settings, and about / licenses.
class ProfileAndStatsStage extends StatefulWidget {
  final VoidCallback? onRerunSetup;

  const ProfileAndStatsStage({super.key, this.onRerunSetup});

  @override
  State<ProfileAndStatsStage> createState() => _ProfileAndStatsStageState();
}

class _ProfileAndStatsStageState extends State<ProfileAndStatsStage> {
  String? _appVersion;

  @override
  void initState() {
    super.initState();
    _loadVersion();
  }

  Future<void> _loadVersion() async {
    final v = await UpdateService.resolveCurrentVersion();
    if (mounted && v != null) {
      setState(() {
        _appVersion = v;
      });
    }
  }

  String _computeListenerPersona(List<TrackItem> tracks) {
    if (tracks.isEmpty) return 'New Listener';
    final total = tracks.length;
    final hiResCount = tracks.where((t) => t.sampleRate >= 88200 || t.bitDepth > 16).length;
    final losslessCount = tracks.where((t) => t.isLossless).length;

    if (hiResCount > 0 && (hiResCount / total) >= 0.3) {
      return 'Hi-Res Purist';
    }
    if ((losslessCount / total) >= 0.6) {
      return 'Lossless Collector';
    }

    final Map<String, int> genreCounts = {};
    for (final t in tracks) {
      final g = t.genre.trim();
      if (g.isNotEmpty && g.toLowerCase() != 'unknown' && g.toLowerCase() != 'uncategorized') {
        genreCounts[g] = (genreCounts[g] ?? 0) + 1;
      }
    }
    if (genreCounts.isNotEmpty) {
      final sorted = genreCounts.entries.toList()..sort((a, b) => b.value.compareTo(a.value));
      return '${sorted.first.key} Connoisseur';
    }
    return 'Audiophile Explorer';
  }

  void _showEditProfileDialog(BuildContext context, String currentName) {
    final controller = TextEditingController(text: currentName);
    final colorScheme = Theme.of(context).colorScheme;

    showDialog(
      context: context,
      builder: (ctx) {
        return AlertDialog(
          backgroundColor: const Color(0xFF131722),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(20),
            side: BorderSide(color: Colors.white.withValues(alpha: 0.12)),
          ),
          title: Text(
            'Edit Profile Name',
            style: AttuneTypography.soraHeading(fontSize: 18, color: colorScheme.onSurface),
          ),
          content: TextField(
            controller: controller,
            autofocus: true,
            style: AttuneTypography.jakartaBody(fontSize: 14, color: colorScheme.onSurface),
            decoration: InputDecoration(
              hintText: 'Enter listener name',
              hintStyle: AttuneTypography.jakartaBody(fontSize: 14, color: Colors.white38),
              filled: true,
              fillColor: Colors.white.withValues(alpha: 0.05),
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(12),
                borderSide: BorderSide(color: colorScheme.primary.withValues(alpha: 0.4)),
              ),
              focusedBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(12),
                borderSide: BorderSide(color: colorScheme.primary, width: 1.5),
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: Text(
                'Cancel',
                style: AttuneTypography.soraBadge(color: Colors.white60),
              ),
            ),
            FilledButton(
              style: FilledButton.styleFrom(
                backgroundColor: colorScheme.primary,
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
              ),
              onPressed: () {
                final newName = controller.text.trim();
                if (newName.isNotEmpty) {
                  SettingsService.instance.setProfileName(newName);
                }
                Navigator.of(ctx).pop();
              },
              child: Text(
                'Save',
                style: AttuneTypography.soraBadge(color: Colors.white),
              ),
            ),
          ],
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListenableBuilder(
      listenable: Listenable.merge([
        PlaybackController.instance,
        SettingsService.instance,
      ]),
      builder: (context, _) {
        final playback = PlaybackController.instance;
        final settings = SettingsService.instance;
        final tracks = playback.libraryTracks;

        // 1. Genuine Library Statistics
        final totalTracks = tracks.length;
        final totalSeconds = tracks.fold<int>(0, (sum, t) => sum + t.duration.inSeconds);
        final totalHours = (totalSeconds / 3600).toStringAsFixed(1);
        final hiResCount = tracks.where((t) => t.sampleRate >= 88200 || t.bitDepth > 16).length;
        final losslessCount = tracks.where((t) => t.isLossless).length;
        final compressedCount = (totalTracks - losslessCount).clamp(0, totalTracks);

        // Persona & User Name
        final profileName = settings.profileName;
        final initial = profileName.isNotEmpty ? profileName[0].toUpperCase() : 'A';
        final persona = _computeListenerPersona(tracks);

        // Dynamic Transition Metric
        String transitionValue;
        String transitionSubtitle;
        if (playback.transitionsCompletedCount > 0) {
          transitionValue = '${playback.transitionsCompletedCount} Mixed';
          final lastArch = playback.lastTransitionArchetype ?? 'DSP Mixer';
          final lastDur = (playback.lastTransitionDurationSec ?? 1.4).toStringAsFixed(1);
          transitionSubtitle = '$lastArch (${lastDur}s)';
        } else {
          final validBpms = tracks.map((t) => t.bpm).where((b) => b > 40 && b < 240).toList();
          final avgBpm = validBpms.isNotEmpty
              ? validBpms.reduce((a, b) => a + b) / validBpms.length
              : 120.0;
          final phraseSec = (4.0 * 60.0 / avgBpm);
          transitionValue = '${phraseSec.toStringAsFixed(1)}s 4-Beat';
          transitionSubtitle = 'Avg ${avgBpm.toStringAsFixed(0)} BPM • JIT Ready';
        }

        // Dynamic DAC Adaptation Metric
        final hwRate = playback.hardwareSampleRate > 0 ? playback.hardwareSampleRate : 44100;
        final curTrack = playback.currentTrack;
        final trackRate = (curTrack != null && curTrack.sampleRate > 0)
            ? curTrack.sampleRate
            : (tracks.isNotEmpty && tracks.first.sampleRate > 0 ? tracks.first.sampleRate : hwRate);

        String dacValue;
        String dacSubtitle;
        if (trackRate == hwRate) {
          dacValue = '1.000x Bit-Perfect';
          dacSubtitle = '${(hwRate / 1000).toStringAsFixed(1)}kHz Native DAC';
        } else {
          final ratio = hwRate / (trackRate > 0 ? trackRate : hwRate);
          dacValue = '${ratio.toStringAsFixed(3)}x Hermite';
          dacSubtitle = '${(trackRate / 1000).toStringAsFixed(1)}k → ${(hwRate / 1000).toStringAsFixed(1)}k Native';
        }

        // Dynamic Genre Distribution Map
        final Map<String, int> genreMap = {};
        for (final t in tracks) {
          final g = t.genre.trim();
          final key = g.isEmpty ? 'Uncategorized' : g;
          genreMap[key] = (genreMap[key] ?? 0) + 1;
        }
        final sortedGenres = genreMap.entries.toList()
          ..sort((a, b) => b.value.compareTo(a.value));

        final engineVersion = AttuneEngineBridge.instance.version;
        final currentVersionStr = _appVersion ?? '0.1.0';

        // Genre color palette
        final palette = [
          colorScheme.primary,
          colorScheme.tertiary,
          const Color(0xFF14B8A6),
          Colors.purpleAccent,
          Colors.amberAccent,
          Colors.cyanAccent,
          Colors.pinkAccent,
          Colors.tealAccent,
        ];

        return ListView(
          physics: const BouncingScrollPhysics(),
          padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 16.0),
          children: [
            // 1. Profile Header Card
            LiquidGlassContainer(
              padding: const EdgeInsets.all(20),
              borderRadius: 24,
              surfaceAlpha: 0.16,
              child: InkWell(
                borderRadius: BorderRadius.circular(20),
                onTap: () => _showEditProfileDialog(context, profileName),
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
                          initial,
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
                          Row(
                            children: [
                              Flexible(
                                child: Text(
                                  profileName,
                                  style: AttuneTypography.soraHeading(
                                    fontSize: 18,
                                    fontWeight: FontWeight.w700,
                                    color: colorScheme.onSurface,
                                  ),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              const SizedBox(width: 6),
                              Icon(
                                Icons.edit_rounded,
                                size: 14,
                                color: colorScheme.onSurface.withValues(alpha: 0.5),
                              ),
                            ],
                          ),
                          const SizedBox(height: 2),
                          Text(
                            persona,
                            style: AttuneTypography.jakartaBody(
                              fontSize: 12,
                              color: colorScheme.onSurface.withValues(alpha: 0.65),
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                          const SizedBox(height: 6),
                          Wrap(
                            spacing: 6,
                            runSpacing: 4,
                            children: [
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                                decoration: BoxDecoration(
                                  color: colorScheme.primary.withValues(alpha: 0.2),
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                child: Text(
                                  'Attune Core v$engineVersion',
                                  style: AttuneTypography.soraBadge(
                                    color: colorScheme.primary,
                                    fontSize: 10,
                                  ),
                                ),
                              ),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                                decoration: BoxDecoration(
                                  color: colorScheme.tertiary.withValues(alpha: 0.2),
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                child: Text(
                                  totalTracks > 0
                                      ? '$losslessCount Lossless (${((losslessCount / totalTracks) * 100).toStringAsFixed(0)}%)'
                                      : '0 Tracks',
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
                    subtitle: totalTracks > 0 ? 'Ingested locally' : 'No tracks',
                    colorScheme: colorScheme,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: _buildMetricTile(
                    label: 'PLAYTIME',
                    value: '$totalHours hrs',
                    subtitle: '$totalTracks library songs',
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
                    value: transitionValue,
                    subtitle: transitionSubtitle,
                    colorScheme: colorScheme,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: _buildMetricTile(
                    label: 'DAC ADAPTATION',
                    value: dacValue,
                    subtitle: dacSubtitle,
                    colorScheme: colorScheme,
                  ),
                ),
              ],
            ),

            const SizedBox(height: 16),

            // 3. Audio Fidelity Profile
            LiquidGlassContainer(
              padding: const EdgeInsets.all(16),
              borderRadius: 20,
              surfaceAlpha: 0.12,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        'AUDIO FIDELITY BREAKDOWN',
                        style: AttuneTypography.soraBadge(
                          color: colorScheme.onSurface.withValues(alpha: 0.6),
                          fontSize: 11,
                        ),
                      ),
                      Text(
                        totalTracks > 0
                            ? '${((losslessCount / totalTracks) * 100).toStringAsFixed(0)}% Lossless'
                            : 'Empty Library',
                        style: AttuneTypography.soraBadge(
                          color: colorScheme.primary,
                          fontSize: 11,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  if (totalTracks == 0)
                    Text(
                      'No tracks ingested. Scan your local music folder to view fidelity telemetry.',
                      style: AttuneTypography.jakartaBody(
                        fontSize: 12,
                        color: colorScheme.onSurface.withValues(alpha: 0.4),
                      ),
                    )
                  else ...[
                    // Multi-segment fidelity bar
                    ClipRRect(
                      borderRadius: BorderRadius.circular(4),
                      child: SizedBox(
                        height: 8,
                        child: Row(
                          children: [
                            if (hiResCount > 0)
                              Expanded(
                                flex: hiResCount,
                                child: Container(color: Colors.amberAccent),
                              ),
                            if (losslessCount - hiResCount > 0)
                              Expanded(
                                flex: losslessCount - hiResCount,
                                child: Container(color: colorScheme.tertiary),
                              ),
                            if (compressedCount > 0)
                              Expanded(
                                flex: compressedCount,
                                child: Container(color: colorScheme.onSurface.withValues(alpha: 0.2)),
                              ),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      spacing: 12,
                      runSpacing: 6,
                      children: [
                        _buildFidelityLegendItem(
                          label: 'Hi-Res ($hiResCount)',
                          pct: totalTracks > 0 ? (hiResCount / totalTracks * 100).toStringAsFixed(0) : '0',
                          color: Colors.amberAccent,
                        ),
                        _buildFidelityLegendItem(
                          label: 'Standard Lossless (${(losslessCount - hiResCount).clamp(0, totalTracks)})',
                          pct: totalTracks > 0
                              ? (((losslessCount - hiResCount).clamp(0, totalTracks)) / totalTracks * 100).toStringAsFixed(0)
                              : '0',
                          color: colorScheme.tertiary,
                        ),
                        _buildFidelityLegendItem(
                          label: 'Compressed ($compressedCount)',
                          pct: totalTracks > 0 ? (compressedCount / totalTracks * 100).toStringAsFixed(0) : '0',
                          color: colorScheme.onSurface.withValues(alpha: 0.45),
                        ),
                      ],
                    ),
                  ],
                ],
              ),
            ),

            const SizedBox(height: 16),

            // 4. Dynamic Genre Distribution
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
                  if (sortedGenres.isEmpty)
                    Text(
                      'No genre tags indexed. Add audio tracks to see genre breakdown.',
                      style: AttuneTypography.jakartaBody(
                        fontSize: 12,
                        color: colorScheme.onSurface.withValues(alpha: 0.4),
                      ),
                    )
                  else
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: List.generate(sortedGenres.length, (idx) {
                        final entry = sortedGenres[idx];
                        final color = palette[idx % palette.length];
                        final pct = totalTracks > 0
                            ? ((entry.value / totalTracks) * 100).toStringAsFixed(0)
                            : '0';
                        return _buildGenreChip(entry.key, entry.value, pct, color);
                      }),
                    ),
                ],
              ),
            ),

            const SizedBox(height: 24),

            // 5. Settings & Library Management
            Text(
              'SETTINGS & ACTIONS',
              style: AttuneTypography.soraBadge(color: colorScheme.primary),
            ),
            const SizedBox(height: 12),

            LiquidGlassContainer(
              padding: const EdgeInsets.symmetric(vertical: 6),
              borderRadius: 20,
              surfaceAlpha: 0.12,
              child: Material(
                type: MaterialType.transparency,
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
                      trailing: playback.isScanning
                          ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
                          : const Icon(Icons.chevron_right_rounded),
                      onTap: playback.isScanning
                          ? null
                          : () => playback.scanLocalAudioDirectories(forceRefresh: true),
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
                        widget.onRerunSetup?.call();
                      },
                    ),
                  ],
                ),
              ),
            ),

            const SizedBox(height: 24),

            // 6. About & Open-Source Licenses
            Text(
              'ABOUT & LEGAL',
              style: AttuneTypography.soraBadge(color: colorScheme.primary),
            ),
            const SizedBox(height: 12),

            LiquidGlassContainer(
              padding: const EdgeInsets.symmetric(vertical: 6),
              borderRadius: 20,
              surfaceAlpha: 0.12,
              child: Material(
                type: MaterialType.transparency,
                child: Column(
                  children: [
                    ListTile(
                      leading: Icon(Icons.info_outline_rounded, color: colorScheme.primary),
                      title: Text(
                        'About Attune',
                        style: AttuneTypography.soraSubheading(fontSize: 14, color: colorScheme.onSurface),
                      ),
                      subtitle: Text(
                        'Version $currentVersionStr (Native Core v$engineVersion)',
                        style: AttuneTypography.jakartaBody(fontSize: 12, color: colorScheme.onSurface.withValues(alpha: 0.5)),
                      ),
                      trailing: const Icon(Icons.chevron_right_rounded),
                      onTap: () {
                        Navigator.of(context).push(
                          MaterialPageRoute(
                            builder: (_) => const AboutStage(),
                          ),
                        );
                      },
                      onLongPress: () => showAttuneAboutDialog(context),
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
                          applicationVersion: 'v$currentVersionStr',
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
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ],
      ),
    );
  }

  Widget _buildFidelityLegendItem({
    required String label,
    required String pct,
    required Color color,
  }) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(
            color: color,
            shape: BoxShape.circle,
          ),
        ),
        const SizedBox(width: 6),
        Text(
          '$label • $pct%',
          style: AttuneTypography.jakartaBody(
            fontSize: 11,
            color: Colors.white70,
            fontWeight: FontWeight.w500,
          ),
        ),
      ],
    );
  }

  Widget _buildGenreChip(String label, int count, String pct, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.18),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.4), width: 1),
      ),
      child: Text(
        '$label: $count ($pct%)',
        style: AttuneTypography.soraBadge(
          color: color,
          fontSize: 11,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}
