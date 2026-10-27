import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// SettingsAndAudioStage implementing Stage 5 of 05_UI_DESIGN_SPEC.md:
/// Liquid Glass Translucency selector, Dynamic Cover Theming toggle,
/// Audio Engine Fidelity, and External Compute Node / Cache Sync controls.
/// Supports running as an independent native Android Activity (SettingsActivity)
/// allowing system-level Predictive Back gesture window peeking!
class SettingsAndAudioStage extends StatefulWidget {
  final bool isStandaloneActivity;

  const SettingsAndAudioStage({
    super.key,
    this.isStandaloneActivity = false,
  });

  @override
  State<SettingsAndAudioStage> createState() => _SettingsAndAudioStageState();
}

class _SettingsAndAudioStageState extends State<SettingsAndAudioStage> {
  double _lensBlurSigma = 36.0;
  bool _dynamicCoverTheming = true;
  String _selectedThemeScheme = 'SchemeVibrant';
  final String _remoteNodeUrl = 'http://localhost:8080';

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: AmbientMeshBackground(
        colorScheme: colorScheme,
        child: SafeArea(
          child: Column(
            children: [
              // 1. App Bar Header with Back / Close Button
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16.0, vertical: 12.0),
                child: Row(
                  children: [
                    LiquidGlassContainer(
                      padding: const EdgeInsets.all(8),
                      borderRadius: 14,
                      surfaceAlpha: 0.18,
                      onTap: () => PlatformNavigation.closeCurrent(context),
                      child: const Icon(Icons.arrow_back_rounded, size: 20, color: Colors.white),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: Text(
                        'Settings & Audio Engine',
                        style: AttuneTypography.soraTitleMedium(color: Colors.white),
                      ),
                    ),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                      decoration: BoxDecoration(
                        color: colorScheme.primary.withValues(alpha: 0.2),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Text(
                        'v0.1.0',
                        style: AttuneTypography.soraMetric(
                          fontSize: 10,
                          weight: FontWeight.w700,
                          color: colorScheme.primary,
                        ),
                      ),
                    ),
                  ],
                ),
              ),

              // 2. Settings Content
              Expanded(
                child: ListView(
                  physics: const BouncingScrollPhysics(),
                  padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 8.0),
                  children: [
                    // Section 1: Liquid Glass Materiality
                    Text('LIQUID GLASS MATERIALITY', style: AttuneTypography.soraSectionHeader(color: colorScheme.primary)),
                    const SizedBox(height: 10),
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
                              Text('Frosted Backdrop Translucency', style: AttuneTypography.soraTitleSmall(color: Colors.white)),
                              Text(
                                '${_lensBlurSigma.toInt()}px',
                                style: AttuneTypography.soraMetric(color: colorScheme.primary),
                              ),
                            ],
                          ),
                          const SizedBox(height: 12),
                          Row(
                            children: [
                              _buildLensOption('16px Subtle', 16.0, colorScheme),
                              const SizedBox(width: 8),
                              _buildLensOption('36px Balanced', 36.0, colorScheme),
                              const SizedBox(width: 8),
                              _buildLensOption('48px Vibrant', 48.0, colorScheme),
                              const SizedBox(width: 8),
                              _buildLensOption('64px Frosted', 64.0, colorScheme),
                            ],
                          ),
                          const Divider(height: 24, color: Colors.white12),
                          ListenableBuilder(
                            listenable: PlaybackController.instance,
                            builder: (context, _) {
                              final controller = PlaybackController.instance;
                              return Row(
                                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                                children: [
                                  Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      Text(
                                        'Living Beat & Lavalamp Globs',
                                        style: AttuneTypography.soraTitleSmall(color: Colors.white),
                                      ),
                                      const SizedBox(height: 2),
                                      Text(
                                        'Fluid liquid light globs responding to track BPM & kick',
                                        style: AttuneTypography.jakartaBodySmall(
                                          color: colorScheme.onSurface.withValues(alpha: 0.6),
                                        ),
                                      ),
                                    ],
                                  ),
                                  Switch(
                                    value: controller.livingBeatMeshEnabled,
                                    activeThumbColor: colorScheme.primary,
                                    onChanged: (v) => controller.setLivingBeatMeshEnabled(v),
                                  ),
                                ],
                              );
                            },
                          ),
                        ],
                      ),
                    ),

                    const SizedBox(height: 24),

                    // Section 2: Material You Dynamic Theming
                    Text('MATERIAL YOU COLOR DYNAMICS', style: AttuneTypography.soraSectionHeader(color: colorScheme.primary)),
                    const SizedBox(height: 10),
                    LiquidGlassContainer(
                      padding: const EdgeInsets.all(16),
                      borderRadius: 20,
                      surfaceAlpha: 0.12,
                      child: Column(
                        children: [
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text('Dynamic Cover Theming', style: AttuneTypography.soraTitleSmall(color: Colors.white)),
                                  const SizedBox(height: 2),
                                  Text(
                                    'Extract HCT tones & morph with transition',
                                    style: AttuneTypography.jakartaBodySmall(
                                      color: colorScheme.onSurface.withValues(alpha: 0.6),
                                    ),
                                  ),
                                ],
                              ),
                              Switch(
                                value: _dynamicCoverTheming,
                                activeThumbColor: colorScheme.primary,
                                onChanged: (v) => setState(() => _dynamicCoverTheming = v),
                              ),
                            ],
                          ),
                          const Divider(height: 24, color: Colors.white12),
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Text('Palette Scheme Generator', style: AttuneTypography.jakartaBodyMedium(color: Colors.white)),
                              DropdownButton<String>(
                                value: _selectedThemeScheme,
                                dropdownColor: const Color(0xFF14161F),
                                underline: const SizedBox(),
                                items: const [
                                  DropdownMenuItem(value: 'SchemeVibrant', child: Text('Vibrant (Recommended)')),
                                  DropdownMenuItem(value: 'SchemeTonalSpot', child: Text('Tonal Spot')),
                                ],
                                onChanged: (v) {
                                  if (v != null) setState(() => _selectedThemeScheme = v);
                                },
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),

                    const SizedBox(height: 24),

                    // Section 3: Dual-Deck Audio Engine & Hardware DAC
                    Text('AUDIO ENGINE & HARDWARE DAC', style: AttuneTypography.soraSectionHeader(color: colorScheme.primary)),
                    const SizedBox(height: 10),
                    LiquidGlassContainer(
                      padding: const EdgeInsets.all(16),
                      borderRadius: 20,
                      surfaceAlpha: 0.12,
                      child: Column(
                        children: [
                          _buildEngineRow('Output Hardware DAC', '48,000 Hz • 2 Channels', colorScheme),
                          const Divider(height: 20, color: Colors.white12),
                          _buildEngineRow('Playhead Resampling', '4-Point Cubic Hermite (ADR-009)', colorScheme),
                          const Divider(height: 20, color: Colors.white12),
                          _buildEngineRow('Vocal Integrity Protection', 'Active & Syllable Locked (ADR-010)', colorScheme),
                          const Divider(height: 20, color: Colors.white12),
                          _buildEngineRow('Transition Phrasing', '1-Bar (4-Beat) Riff Handoff', colorScheme),
                        ],
                      ),
                    ),

                    const SizedBox(height: 24),

                    // Section 4: External Compute Node & Cache Sync
                    Text('EXTERNAL COMPUTE NODE & CACHE SYNC', style: AttuneTypography.soraSectionHeader(color: colorScheme.primary)),
                    const SizedBox(height: 10),
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
                              Text('Remote Node URL', style: AttuneTypography.soraTitleSmall(color: Colors.white)),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                                decoration: BoxDecoration(
                                  color: Colors.green.withValues(alpha: 0.2),
                                  borderRadius: BorderRadius.circular(6),
                                ),
                                child: Text(
                                  'Ready',
                                  style: AttuneTypography.soraMetric(fontSize: 10, color: Colors.greenAccent),
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 10),
                          LiquidGlassContainer(
                            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                            borderRadius: 12,
                            surfaceAlpha: 0.10,
                            child: Row(
                              children: [
                                const Icon(Icons.wifi_tethering_rounded, size: 18, color: Colors.white70),
                                const SizedBox(width: 8),
                                Expanded(
                                  child: Text(
                                    _remoteNodeUrl,
                                    style: AttuneTypography.soraMetric(fontSize: 13, color: Colors.white),
                                  ),
                                ),
                                TextButton(
                                  onPressed: () {
                                    ScaffoldMessenger.of(context).showSnackBar(
                                      const SnackBar(content: Text('Synced with External Compute Node!')),
                                    );
                                  },
                                  child: const Text('Sync Now'),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),

                    const SizedBox(height: 32),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildLensOption(String label, double sigma, ColorScheme colorScheme) {
    final selected = _lensBlurSigma == sigma;
    return Expanded(
      child: LiquidGlassContainer(
        padding: const EdgeInsets.symmetric(vertical: 8),
        borderRadius: 12,
        surfaceAlpha: selected ? 0.28 : 0.08,
        tintOverride: selected ? colorScheme.primary.withValues(alpha: 0.24) : null,
        onTap: () => setState(() => _lensBlurSigma = sigma),
        child: Center(
          child: Text(
            label,
            style: AttuneTypography.soraBadge(
              color: selected ? colorScheme.primary : Colors.white70,
            ),
            textAlign: TextAlign.center,
          ),
        ),
      ),
    );
  }

  Widget _buildEngineRow(String title, String value, ColorScheme colorScheme) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(title, style: AttuneTypography.jakartaBodyMedium(color: Colors.white70)),
        Text(
          value,
          style: AttuneTypography.soraMetric(
            fontSize: 12,
            weight: FontWeight.w600,
            color: colorScheme.primary,
          ),
        ),
      ],
    );
  }
}
