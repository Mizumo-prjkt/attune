import 'dart:io';
import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../theme/color_engine.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/audio_quality_badge.dart';
import '../widgets/transition_connector_pill.dart';
import 'synchronized_lyrics_stage.dart';

/// NowPlayingStage implementing Stage 1 of 05_UI_DESIGN_SPEC.md:
/// Integrated lyrics toggle inside player, collapse header button,
/// subtle Cue-Zone wave progress bar, Sora title/artist,
/// Human-Friendly Audio Quality badge, transport controls with Smart Shuffle,
/// and mini Up-Next pill.
class NowPlayingStage extends StatefulWidget {
  final bool? isPlaying;
  final VoidCallback? onTogglePlay;
  final VoidCallback? onNext;
  final VoidCallback? onPrevious;
  final VoidCallback? onDismiss;

  const NowPlayingStage({
    super.key,
    this.isPlaying,
    this.onTogglePlay,
    this.onNext,
    this.onPrevious,
    this.onDismiss,
  });

  @override
  State<NowPlayingStage> createState() => _NowPlayingStageState();
}

class _NowPlayingStageState extends State<NowPlayingStage> {
  bool _showLyrics = false;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final playing = widget.isPlaying ?? controller.isPlaying;
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

        return Container(
          color: colorScheme.surface.withValues(alpha: 0.98),
          child: SafeArea(
            child: SingleChildScrollView(
              physics: const BouncingScrollPhysics(),
          padding: const EdgeInsets.symmetric(horizontal: 24.0, vertical: 12.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              // 0. Top Bar: Dismiss/Collapse Button, View Mode Segment (Artwork vs Lyrics), Settings
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  if (widget.onDismiss != null)
                    IconButton(
                      icon: const Icon(Icons.keyboard_arrow_down_rounded, size: 28),
                      color: colorScheme.onSurface.withValues(alpha: 0.8),
                      onPressed: widget.onDismiss,
                      tooltip: 'Collapse to main screen',
                    )
                  else
                    const SizedBox(width: 48),

                  // Segmented switch: Artwork vs Lyrics
                  LiquidGlassContainer(
                    padding: const EdgeInsets.all(3),
                    borderRadius: 16,
                    surfaceAlpha: 0.16,
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        _buildModeTab('Artwork', !_showLyrics, colorScheme, () {
                          setState(() => _showLyrics = false);
                        }),
                        _buildModeTab('Lyrics', _showLyrics, colorScheme, () {
                          setState(() => _showLyrics = true);
                        }),
                      ],
                    ),
                  ),

                  IconButton(
                    icon: const Icon(Icons.settings_outlined, size: 22),
                    color: colorScheme.onSurface.withValues(alpha: 0.8),
                    onPressed: () => PlatformNavigation.openSettings(context),
                    tooltip: 'Audio Settings',
                  ),
                ],
              ),

              const SizedBox(height: 16),

              // 1. Hero Artwork Card OR Integrated Synchronized Lyrics
              Center(
                child: AnimatedSwitcher(
                  duration: const Duration(milliseconds: 300),
                  child: _showLyrics
                      ? SizedBox(
                          key: const ValueKey('lyrics_view'),
                          width: 320,
                          height: 320,
                          child: LiquidGlassContainer(
                            padding: EdgeInsets.zero,
                            borderRadius: 32,
                            surfaceAlpha: 0.16,
                            child: const ClipRRect(
                              borderRadius: BorderRadius.all(Radius.circular(32)),
                              child: SynchronizedLyricsStage(),
                            ),
                          ),
                        )
                      : LiquidGlassContainer(
                          key: const ValueKey('artwork_view'),
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
                            ],
                          ),
                        ),
                ),
              ),

              const SizedBox(height: 24),

              // 2. Track Title & Artist
              Text(
                track?.title ?? 'Rabbit Hole',
                style: AttuneTypography.soraTitleLarge(color: colorScheme.onSurface),
                textAlign: TextAlign.center,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 4),
              Text(
                track?.artist ?? 'DECO*27 feat. Hatsune Miku',
                style: AttuneTypography.jakartaArtistSubtitle(
                  color: colorScheme.onSurface.withValues(alpha: 0.72),
                ),
                textAlign: TextAlign.center,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),

              const SizedBox(height: 12),

              // 3. Human-Friendly Audio Quality Glass Badge (Section 3.1)
              AudioQualityBadge(
                format: track?.format ?? 'FLAC',
                sampleRate: track?.sampleRate ?? 44100,
                bitDepth: track?.bitDepth ?? 16,
                isProprietary: track?.isProprietary ?? false,
              ),

              const SizedBox(height: 20),

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

              const SizedBox(height: 14),

              // 5. Transport Controls with Tri-State Shuffle (Off, Standard, Smart)
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  _buildShuffleButton(controller, colorScheme),
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(Icons.skip_previous_rounded),
                    iconSize: 34,
                    visualDensity: VisualDensity.compact,
                    color: colorScheme.onSurface,
                    onPressed: widget.onPrevious ?? controller.skipPrevious,
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
                    onTap: widget.onTogglePlay ?? controller.togglePlay,
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
                    onPressed: widget.onNext ?? controller.skipNext,
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

              const SizedBox(height: 20),

              // 6. Subtle Inline Transition Connector Pill (Section 3.2)
              if (controller.isTransitioning ||
                  (controller.shuffleMode == ShuffleMode.smart && controller.pendingRecipe != null)) ...[
                Builder(
                  builder: (context) {
                    final recipe = controller.pendingRecipe;
                    final timing = recipe?['timing'] as Map<String, dynamic>?;
                    final durSec = timing != null
                        ? (((timing['track_a_exit_end_ms'] as num? ?? 1400) -
                                (timing['track_a_transition_start_ms'] as num? ?? 0))
                            .abs() / 1000.0)
                        : 1.4;
                    final conf = (recipe?['confidence_score'] as num?)?.toDouble() ?? 0.95;
                    return TransitionConnectorPill(
                      archetype: controller.activeArchetype,
                      durationSeconds: durSec,
                      confidenceScore: conf,
                      onTap: () => PlatformNavigation.openTransitionInspector(context),
                    );
                  },
                ),
                const SizedBox(height: 16),
              ],

              // 7. Mini Up-Next Row Preview
              if (nextTrack != null)
                LiquidGlassContainer(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                  borderRadius: 20,
                  surfaceAlpha: 0.12,
                  child: Row(
                    children: [
                      Container(
                        width: 36,
                        height: 36,
                        decoration: BoxDecoration(
                          borderRadius: BorderRadius.circular(10),
                          gradient: LinearGradient(
                            colors: [nextTrack.seedColor, nextTrack.seedColor.withValues(alpha: 0.5)],
                          ),
                        ),
                        child: ClipRRect(
                          borderRadius: BorderRadius.circular(10),
                          child: nextTrack.artworkFile != null && File(nextTrack.artworkFile!).existsSync()
                              ? Image.file(File(nextTrack.artworkFile!), fit: BoxFit.cover)
                              : const Icon(Icons.music_note, color: Colors.white, size: 20),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              'UP NEXT',
                              style: AttuneTypography.soraBadge(
                                color: colorScheme.onSurface.withValues(alpha: 0.6),
                                fontSize: 9,
                              ),
                            ),
                            Text(
                              nextTrack.title,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: AttuneTypography.soraSubheading(
                                fontSize: 13,
                                fontWeight: FontWeight.w600,
                                color: colorScheme.onSurface,
                              ),
                            ),
                          ],
                        ),
                      ),
                      TransitionConnectorPill(
                        archetype: 'Riff Handoff',
                        durationSeconds: 1.4,
                        confidenceScore: 0.91,
                        isCompact: true,
                        onTap: () => PlatformNavigation.openTransitionInspector(context),
                      ),
                    ],
                  ),
                ),

              const SizedBox(height: 80),
            ],
          ),
            ),
          ),
        );
      },
    );
  }

  Widget _buildModeTab(String label, bool isSelected, ColorScheme colorScheme, VoidCallback onTap) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected ? colorScheme.primary.withValues(alpha: 0.28) : Colors.transparent,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Text(
          label,
          style: AttuneTypography.soraBadge(
            color: isSelected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.6),
            fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
          ),
        ),
      ),
    );
  }

  Widget _buildShuffleButton(PlaybackController controller, ColorScheme colorScheme) {
    IconData icon;
    Color color;
    String tooltip;

    switch (controller.shuffleMode) {
      case ShuffleMode.off:
        icon = Icons.shuffle_rounded;
        color = colorScheme.onSurface.withValues(alpha: 0.35);
        tooltip = 'Shuffle Off (Linear Sequence)';
        break;
      case ShuffleMode.standard:
        icon = Icons.shuffle_rounded;
        color = colorScheme.primary;
        tooltip = 'Basic Shuffle (Random Track)';
        break;
      case ShuffleMode.smart:
        icon = Icons.auto_awesome_rounded;
        color = colorScheme.tertiary;
        tooltip = 'Smart Shuffle (Algorithmic Transition Match)';
        break;
    }

    return IconButton(
      icon: Icon(icon),
      iconSize: 22,
      visualDensity: VisualDensity.compact,
      color: color,
      tooltip: tooltip,
      onPressed: controller.cycleShuffleMode,
    );
  }

  Widget _buildFallbackCover(ColorScheme colorScheme) {
    return Container(
      decoration: BoxDecoration(
        gradient: LinearGradient(
          colors: [
            colorScheme.primary.withValues(alpha: 0.5),
            colorScheme.secondary.withValues(alpha: 0.3),
            colorScheme.surface,
          ],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
      ),
      child: Center(
        child: Opacity(
          opacity: 0.85,
          child: Image.asset(
            'assets/icon/icon_transparent.png',
            width: 96,
            height: 96,
            fit: BoxFit.contain,
          ),
        ),
      ),
    );
  }
}
