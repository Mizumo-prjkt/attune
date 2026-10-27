import 'dart:io';
import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../theme/color_engine.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/audio_quality_badge.dart';
import '../widgets/transition_connector_pill.dart';

/// NowPlayingStage implementing Stage 1 of 05_UI_DESIGN_SPEC.md:
/// Artwork card, subtle Cue-Zone wave progress bar, Sora title/artist,
/// Human-Friendly Audio Quality badge, transport controls, and mini Up-Next pill.
class NowPlayingStage extends StatelessWidget {
  final bool? isPlaying;
  final VoidCallback? onTogglePlay;
  final VoidCallback? onNext;
  final VoidCallback? onPrevious;

  const NowPlayingStage({
    super.key,
    this.isPlaying,
    this.onTogglePlay,
    this.onNext,
    this.onPrevious,
  });

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final playing = isPlaying ?? controller.isPlaying;
        final track = controller.currentTrack;
        final nextTrack = controller.upNextTrack;

        final posSec = controller.positionSec;
        final durSec = controller.durationSec > 0 ? controller.durationSec : 159.0;

        final currentMin = (posSec / 60).floor();
        final currentSec = (posSec % 60).floor();
        final totalMin = (durSec / 60).floor();
        final totalSec = (durSec % 60).floor();

        // Calculate dynamic ColorScheme based on track seedColor and transition morphing
        final baseScheme =
            AttuneColorEngine.createSchemeFromSeed(track?.seedColor ?? AttuneColorEngine.fallbackPrimary);
        final targetScheme =
            AttuneColorEngine.createSchemeFromSeed(nextTrack?.seedColor ?? AttuneColorEngine.fallbackSecondary);
        final colorScheme = controller.isTransitioning
            ? AttuneColorEngine.lerpSchemes(baseScheme, targetScheme, controller.transitionProgress)
            : baseScheme;

        return SingleChildScrollView(
          physics: const BouncingScrollPhysics(),
          padding: const EdgeInsets.symmetric(horizontal: 24.0, vertical: 16.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              const SizedBox(height: 8),

              // 1. Hero Artwork Glass Card
              Center(
                child: LiquidGlassContainer(
                  width: 320,
                  height: 320,
                  padding: EdgeInsets.zero,
                  borderRadius: 32,
                  surfaceAlpha: 0.16,
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      if (track?.artworkFile != null && File(track!.artworkFile!).existsSync())
                        ClipRRect(
                          borderRadius: BorderRadius.circular(32),
                          child: Image.file(
                            File(track.artworkFile!),
                            fit: BoxFit.cover,
                            errorBuilder: (context, error, stackTrace) => _buildFallbackCover(colorScheme),
                          ),
                        )
                      else
                        _buildFallbackCover(colorScheme),
                      Positioned(
                        top: 16,
                        right: 16,
                        child: LiquidGlassContainer(
                          padding: const EdgeInsets.all(8),
                          borderRadius: 14,
                          surfaceAlpha: 0.25,
                          onTap: () => PlatformNavigation.openSettings(context),
                          child: const Icon(
                            Icons.settings_outlined,
                            size: 18,
                            color: Colors.white,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),

              const SizedBox(height: 28),

              // 2. Track Title & Artist
              Text(
                track?.title ?? 'Rabbit Hole',
                style: AttuneTypography.soraTitleLarge(color: colorScheme.onSurface),
                textAlign: TextAlign.center,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 6),
              Text(
                track?.artist ?? 'DECO*27 feat. Hatsune Miku',
                style: AttuneTypography.jakartaArtistSubtitle(
                  color: colorScheme.onSurface.withValues(alpha: 0.72),
                ),
                textAlign: TextAlign.center,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),

              const SizedBox(height: 14),

              // 3. Human-Friendly Audio Quality Glass Badge (Section 3.1)
              AudioQualityBadge(
                format: track?.format ?? 'FLAC',
                sampleRate: track?.sampleRate ?? 44100,
                bitDepth: track?.bitDepth ?? 16,
                isProprietary: track?.isProprietary ?? false,
              ),

              const SizedBox(height: 24),

              // 4. Subtle Cue-Zone Wave Progress Bar
              Column(
                children: [
                  SliderTheme(
                    data: SliderTheme.of(context).copyWith(
                      trackHeight: 4,
                      activeTrackColor: colorScheme.primary,
                      inactiveTrackColor: colorScheme.surfaceContainerHigh.withValues(alpha: 0.3),
                      thumbColor: Colors.white,
                      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                      overlayShape: const RoundSliderOverlayShape(overlayRadius: 14),
                    ),
                    child: Slider(
                      value: posSec.clamp(0.0, durSec),
                      min: 0.0,
                      max: durSec,
                      onChanged: (v) => controller.seek(v),
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 16.0),
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text(
                          '${currentMin.toString().padLeft(2, '0')}:${currentSec.toString().padLeft(2, '0')}',
                          style: AttuneTypography.soraMetric(
                            fontSize: 12,
                            color: colorScheme.onSurface.withValues(alpha: 0.6),
                          ),
                        ),
                        Text(
                          '${totalMin.toString().padLeft(2, '0')}:${totalSec.toString().padLeft(2, '0')}',
                          style: AttuneTypography.soraMetric(
                            fontSize: 12,
                            color: colorScheme.onSurface.withValues(alpha: 0.6),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),

              const SizedBox(height: 16),

              // 5. Transport Controls
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  IconButton(
                    icon: const Icon(Icons.shuffle_rounded),
                    iconSize: 22,
                    visualDensity: VisualDensity.compact,
                    color: colorScheme.onSurface.withValues(alpha: 0.7),
                    onPressed: () {},
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(Icons.skip_previous_rounded),
                    iconSize: 34,
                    visualDensity: VisualDensity.compact,
                    color: colorScheme.onSurface,
                    onPressed: onPrevious ?? controller.skipPrevious,
                  ),
                  const SizedBox(width: 12),

                  // Play / Pause Glass Button
                  LiquidGlassContainer(
                    width: 68,
                    height: 68,
                    padding: EdgeInsets.zero,
                    borderRadius: 34,
                    surfaceAlpha: 0.22,
                    tintOverride: colorScheme.primary.withValues(alpha: 0.28),
                    onTap: onTogglePlay ?? controller.togglePlay,
                    child: Center(
                      child: Icon(
                        playing ? Icons.pause_rounded : Icons.play_arrow_rounded,
                        size: 36,
                        color: Colors.white,
                      ),
                    ),
                  ),

                  const SizedBox(width: 12),
                  IconButton(
                    icon: const Icon(Icons.skip_next_rounded),
                    iconSize: 34,
                    visualDensity: VisualDensity.compact,
                    color: colorScheme.onSurface,
                    onPressed: onNext ?? controller.skipNext,
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(Icons.repeat_rounded),
                    iconSize: 22,
                    visualDensity: VisualDensity.compact,
                    color: colorScheme.onSurface.withValues(alpha: 0.7),
                    onPressed: () {},
                  ),
                ],
              ),

              const SizedBox(height: 28),

              // 6. Up-Next Preview Bar with Transition Connector Pill (Section 3.2)
              LiquidGlassContainer(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                borderRadius: 20,
                surfaceAlpha: 0.12,
                child: Row(
                  children: [
                    ClipRRect(
                      borderRadius: BorderRadius.circular(10),
                      child: Container(
                        width: 40,
                        height: 40,
                        color: colorScheme.secondary.withValues(alpha: 0.5),
                        child: const Icon(Icons.music_note_rounded, color: Colors.white, size: 20),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'UP NEXT',
                            style: AttuneTypography.soraSectionHeader(
                              color: colorScheme.primary,
                            ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            nextTrack?.title ?? '(Not) A Devil',
                            style: AttuneTypography.soraTitleSmall(color: colorScheme.onSurface),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                    Flexible(
                      child: FittedBox(
                        fit: BoxFit.scaleDown,
                        child: TransitionConnectorPill(
                          archetype: controller.activeArchetype,
                          durationSeconds: 1.4,
                          isCompact: true,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
            ],
          ),
        );
      },
    );
  }

  Widget _buildFallbackCover(ColorScheme colorScheme) {
    return Container(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(32),
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            colorScheme.primary.withValues(alpha: 0.8),
            colorScheme.secondary.withValues(alpha: 0.8),
          ],
        ),
      ),
      child: Center(
        child: Icon(
          Icons.album_rounded,
          size: 110,
          color: Colors.white.withValues(alpha: 0.9),
        ),
      ),
    );
  }
}
