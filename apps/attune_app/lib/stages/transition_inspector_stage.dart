import 'package:flutter/material.dart';
import '../services/platform_navigation.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// TransitionInspectorStage: Full Dual-Deck Transition Inspector
/// Visualizes Deck A / Deck B cue-zone waveforms, 3-band EQ curves,
/// vocal protection status, and reverb freeze wash curves.
/// Supports running as an independent native Android Activity (TransitionInspectorActivity)
/// allowing system-level Predictive Back gesture window peeking!
class TransitionInspectorStage extends StatefulWidget {
  final bool isStandaloneActivity;

  const TransitionInspectorStage({
    super.key,
    this.isStandaloneActivity = false,
  });

  @override
  State<TransitionInspectorStage> createState() => _TransitionInspectorStageState();
}

class _TransitionInspectorStageState extends State<TransitionInspectorStage> {
  double _previewProgress = 0.5;

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
              // 1. App Bar Header
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
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Transition Inspector',
                            style: AttuneTypography.soraTitleMedium(color: Colors.white),
                          ),
                          Text(
                            'Rabbit Hole  ➔  (Not) A Devil',
                            style: AttuneTypography.jakartaBodySmall(
                              color: colorScheme.primary,
                            ),
                          ),
                        ],
                      ),
                    ),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                      decoration: BoxDecoration(
                        color: colorScheme.primary.withValues(alpha: 0.2),
                        borderRadius: BorderRadius.circular(10),
                      ),
                      child: Text(
                        'Riff Handoff • 1.4s',
                        style: AttuneTypography.soraBadge(color: colorScheme.primary),
                      ),
                    ),
                  ],
                ),
              ),

              // 2. Inspector Visualization Body
              Expanded(
                child: ListView(
                  physics: const BouncingScrollPhysics(),
                  padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 8.0),
                  children: [
                    // Section 1: Vocal Integrity Protection Status Badge (ADR-010)
                    LiquidGlassContainer(
                      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                      borderRadius: 18,
                      surfaceAlpha: 0.16,
                      tintOverride: Colors.green.withValues(alpha: 0.18),
                      child: Row(
                        children: [
                          const Icon(Icons.shield_rounded, color: Colors.greenAccent, size: 22),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  'Vocal Integrity Protection: ACTIVE (ADR-010)',
                                  style: AttuneTypography.soraBadge(color: Colors.greenAccent),
                                ),
                                const SizedBox(height: 2),
                                Text(
                                  'Miku vocal phrase finishes at 157.96s • Mid EQ held flat at 0.0 dB',
                                  style: AttuneTypography.jakartaBodySmall(
                                    color: Colors.white.withValues(alpha: 0.8),
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),

                    const SizedBox(height: 20),

                    // Section 2: Dual-Deck Waveforms (Deck A vs Deck B)
                    Text('DUAL-DECK PHRASE ALIGNMENT', style: AttuneTypography.soraSectionHeader(color: colorScheme.primary)),
                    const SizedBox(height: 10),
                    LiquidGlassContainer(
                      padding: const EdgeInsets.all(16),
                      borderRadius: 20,
                      surfaceAlpha: 0.12,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          // Deck A Track Waveform Bar
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Text('Deck A: Rabbit Hole', style: AttuneTypography.soraTitleSmall(color: Colors.white)),
                              Text('156.56s ➔ 157.96s (Cut)', style: AttuneTypography.soraMetric(color: colorScheme.primary, fontSize: 11)),
                            ],
                          ),
                          const SizedBox(height: 8),
                          _buildWaveformStrip(colorScheme.primary, 0.85, isDeckA: true),

                          const SizedBox(height: 16),

                          // Deck B Track Waveform Bar
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Text('Deck B: (Not) A Devil', style: AttuneTypography.soraTitleSmall(color: Colors.white)),
                              Text('1.02s ➔ 2.42s (Locked)', style: AttuneTypography.soraMetric(color: colorScheme.secondary, fontSize: 11)),
                            ],
                          ),
                          const SizedBox(height: 8),
                          _buildWaveformStrip(colorScheme.secondary, 0.35, isDeckA: false),
                        ],
                      ),
                    ),

                    const SizedBox(height: 20),

                    // Section 3: 3-Band Isolator EQ & Reverb Automation Curves
                    Text('DSP AUTOMATION KEYFRAMES', style: AttuneTypography.soraSectionHeader(color: colorScheme.primary)),
                    const SizedBox(height: 10),
                    LiquidGlassContainer(
                      padding: const EdgeInsets.all(16),
                      borderRadius: 20,
                      surfaceAlpha: 0.12,
                      child: Column(
                        children: [
                          _buildKeyframeRow('0 ms (Beat 1)', 'Deck A: 0.0 dB flat | Deck B: -18.0 dB (sub -24 dB)', colorScheme),
                          const Divider(height: 16, color: Colors.white12),
                          _buildKeyframeRow('350 ms (Beat 2)', 'Deck A sub: -6.0 dB | Deck B: -8.0 dB', colorScheme),
                          const Divider(height: 16, color: Colors.white12),
                          _buildKeyframeRow('700 ms (Beat 3)', 'Deck A sub: -12.0 dB | Deck B: -4.0 dB', colorScheme),
                          const Divider(height: 16, color: Colors.white12),
                          _buildKeyframeRow('1050 ms (Beat 4)', 'Deck A mid: 0.0 dB (Vocal Safe!) | Deck B: -1.5 dB', colorScheme),
                          const Divider(height: 16, color: Colors.white12),
                          _buildKeyframeRow('1400 ms (Resolution)', 'Deck A: Cut -60 dB + 0.45 Reverb Freeze Spillover', colorScheme),
                        ],
                      ),
                    ),

                    const SizedBox(height: 20),

                    // Section 4: Live Scrub Scrubber Preview
                    Text('TRANSITION PROGRESS PREVIEW', style: AttuneTypography.soraSectionHeader(color: colorScheme.primary)),
                    const SizedBox(height: 8),
                    SliderTheme(
                      data: SliderTheme.of(context).copyWith(
                        trackHeight: 6,
                        activeTrackColor: colorScheme.primary,
                        inactiveTrackColor: colorScheme.surfaceContainerHigh.withValues(alpha: 0.3),
                        thumbColor: Colors.white,
                        thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 8),
                      ),
                      child: Slider(
                        value: _previewProgress,
                        min: 0.0,
                        max: 1.0,
                        onChanged: (v) => setState(() => _previewProgress = v),
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

  Widget _buildWaveformStrip(Color color, double progress, {required bool isDeckA}) {
    return Container(
      height: 36,
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.05),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Stack(
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: List.generate(36, (i) {
              final h = (i % 5 + 1) * 6.0;
              final active = isDeckA ? (i / 36.0 <= progress) : (i / 36.0 >= progress);
              return Container(
                width: 3,
                height: h,
                decoration: BoxDecoration(
                  color: active ? color : Colors.white24,
                  borderRadius: BorderRadius.circular(2),
                ),
              );
            }),
          ),
        ],
      ),
    );
  }

  Widget _buildKeyframeRow(String time, String details, ColorScheme colorScheme) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 110,
          child: Text(
            time,
            style: AttuneTypography.soraMetric(
              fontSize: 11,
              weight: FontWeight.w700,
              color: colorScheme.primary,
            ),
          ),
        ),
        Expanded(
          child: Text(
            details,
            style: AttuneTypography.jakartaBodySmall(color: Colors.white),
          ),
        ),
      ],
    );
  }
}
