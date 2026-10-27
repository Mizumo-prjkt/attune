import 'package:flutter/material.dart';
import '../services/platform_navigation.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// TransitionInspectorStage: Full Dual-Deck Transition Inspector
/// Visualizes Deck A / Deck B cue-zone waveforms, live 3-band EQ curves,
/// vocal protection status, reverb freeze wash curves, and manual trigger controls.
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
  double _previewProgress = 0.0;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final deckA = controller.currentTrack;
        final deckB = controller.upNextTrack;
        final isTransitioning = controller.isTransitioning;
        final progress = isTransitioning ? controller.transitionProgress : _previewProgress;

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
                                '${deckA?.title ?? "Deck A"}  ➔  ${deckB?.title ?? "Deck B"}',
                                style: AttuneTypography.jakartaBodySmall(
                                  color: colorScheme.primary,
                                ),
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
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
                            isTransitioning
                                ? '${controller.activeArchetype} • ${(progress * 100).toInt()}%'
                                : '${controller.activeArchetype} • Armed',
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
                                      'Phrase boundary analysis guarantees no syllable cutoff • Mid EQ safe',
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
                        Text(
                          'DUAL-DECK PHRASE ALIGNMENT',
                          style: AttuneTypography.soraSectionHeader(color: colorScheme.primary),
                        ),
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
                                  Expanded(
                                    child: Text(
                                      'Deck A: ${deckA?.title ?? "No Track Loaded"}',
                                      style: AttuneTypography.soraTitleSmall(color: Colors.white),
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                  Text(
                                    '${controller.positionSec.toStringAsFixed(1)}s / ${controller.durationSec.toStringAsFixed(1)}s',
                                    style: AttuneTypography.soraMetric(color: colorScheme.primary, fontSize: 11),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 8),
                              _buildWaveformStrip(
                                colorScheme.primary,
                                controller.durationSec > 0 ? (controller.positionSec / controller.durationSec).clamp(0.0, 1.0) : 0.0,
                                isDeckA: true,
                              ),

                              const SizedBox(height: 16),

                              // Deck B Track Waveform Bar
                              Row(
                                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                                children: [
                                  Expanded(
                                    child: Text(
                                      'Deck B: ${deckB?.title ?? "Cue Empty"}',
                                      style: AttuneTypography.soraTitleSmall(color: Colors.white),
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                  Text(
                                    deckB != null ? '${deckB.bpm.toStringAsFixed(1)} BPM • ${deckB.musicalKey}' : 'Ready',
                                    style: AttuneTypography.soraMetric(color: colorScheme.secondary, fontSize: 11),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 8),
                              _buildWaveformStrip(colorScheme.secondary, progress, isDeckA: false),
                            ],
                          ),
                        ),

                        const SizedBox(height: 20),

                        // Section 3: Live DSP 3-Band Isolator EQ & Reverb Telemetry
                        Text(
                          'LIVE DSP AUTOMATION TELEMETRY',
                          style: AttuneTypography.soraSectionHeader(color: colorScheme.primary),
                        ),
                        const SizedBox(height: 10),
                        LiquidGlassContainer(
                          padding: const EdgeInsets.all(16),
                          borderRadius: 20,
                          surfaceAlpha: 0.12,
                          child: Column(
                            children: [
                              _buildTelemetryRow(
                                'Deck A EQ',
                                'Sub: ${controller.deckAEqLow.toStringAsFixed(1)} dB | Mid: ${controller.deckAEqMid.toStringAsFixed(1)} dB | High: ${controller.deckAEqHigh.toStringAsFixed(1)} dB',
                                colorScheme,
                              ),
                              const Divider(height: 16, color: Colors.white12),
                              _buildTelemetryRow(
                                'Deck B EQ',
                                'Sub: ${controller.deckBEqLow.toStringAsFixed(1)} dB | Mid: ${controller.deckBEqMid.toStringAsFixed(1)} dB | High: ${controller.deckBEqHigh.toStringAsFixed(1)} dB',
                                colorScheme,
                              ),
                              const Divider(height: 16, color: Colors.white12),
                              _buildTelemetryRow(
                                'Reverb Spillover',
                                '${(controller.reverbFreezeSend * 100).toStringAsFixed(0)}% Freeze Send Washout',
                                colorScheme,
                              ),
                              const Divider(height: 16, color: Colors.white12),
                              _buildTelemetryRow(
                                'Active Strategy',
                                '${controller.activeArchetype} • Smooth Crossfade & Phrase Snapping',
                                colorScheme,
                              ),
                            ],
                          ),
                        ),

                        const SizedBox(height: 20),

                        // Section 4: Live Transition Scrubber & Trigger Button
                        Text(
                          'TRANSITION PROGRESS & CONTROLS',
                          style: AttuneTypography.soraSectionHeader(color: colorScheme.primary),
                        ),
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
                            value: progress.clamp(0.0, 1.0),
                            min: 0.0,
                            max: 1.0,
                            onChanged: isTransitioning
                                ? null
                                : (v) => setState(() => _previewProgress = v),
                          ),
                        ),

                        const SizedBox(height: 12),

                        // Manual Trigger Action Button
                        LiquidGlassContainer(
                          padding: const EdgeInsets.symmetric(vertical: 14),
                          borderRadius: 20,
                          surfaceAlpha: 0.22,
                          tintOverride: colorScheme.primary.withValues(alpha: 0.28),
                          onTap: isTransitioning ? null : () => controller.triggerTransitionNow(),
                          child: Center(
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Icon(
                                  isTransitioning ? Icons.auto_awesome_rounded : Icons.flash_on_rounded,
                                  color: Colors.white,
                                  size: 20,
                                ),
                                const SizedBox(width: 8),
                                Text(
                                  isTransitioning ? 'TRANSITION IN PROGRESS...' : 'TRIGGER SEAMLESS TRANSITION NOW',
                                  style: AttuneTypography.soraTitleSmall(color: Colors.white),
                                ),
                              ],
                            ),
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
      },
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
              final active = isDeckA ? (i / 36.0 <= progress) : (i / 36.0 <= progress);
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

  Widget _buildTelemetryRow(String label, String details, ColorScheme colorScheme) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 120,
          child: Text(
            label,
            style: AttuneTypography.soraMetric(
              fontSize: 12,
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
