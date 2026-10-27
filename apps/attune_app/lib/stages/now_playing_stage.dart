import 'package:flutter/material.dart';
import '../services/platform_navigation.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/audio_quality_badge.dart';
import '../widgets/transition_connector_pill.dart';

/// NowPlayingStage implementing Stage 1 of 05_UI_DESIGN_SPEC.md:
/// Artwork card, subtle Cue-Zone wave progress bar, Sora title/artist,
/// Human-Friendly Audio Quality badge, transport controls, and mini Up-Next pill.
class NowPlayingStage extends StatefulWidget {
  final bool isPlaying;
  final VoidCallback onTogglePlay;
  final VoidCallback? onNext;
  final VoidCallback? onPrevious;

  const NowPlayingStage({
    super.key,
    required this.isPlaying,
    required this.onTogglePlay,
    this.onNext,
    this.onPrevious,
  });

  @override
  State<NowPlayingStage> createState() => _NowPlayingStageState();
}

class _NowPlayingStageState extends State<NowPlayingStage> {
  double _playbackPositionSec = 156.0;
  final double _trackDurationSec = 159.0;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    final currentMin = (_playbackPositionSec / 60).floor();
    final currentSec = (_playbackPositionSec % 60).floor();
    final totalMin = (_trackDurationSec / 60).floor();
    final totalSec = (_trackDurationSec % 60).floor();

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
                  // Album Art Image or vibrant gradient fallback
                  Container(
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
                  ),

                  // Subtle Top-Right Settings Quick Action
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
            'Rabbit Hole',
            style: AttuneTypography.soraTitleLarge(color: colorScheme.onSurface),
            textAlign: TextAlign.center,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 6),
          Text(
            'DECO*27 feat. Hatsune Miku',
            style: AttuneTypography.jakartaArtistSubtitle(
              color: colorScheme.onSurface.withValues(alpha: 0.72),
            ),
            textAlign: TextAlign.center,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),

          const SizedBox(height: 14),

          // 3. Human-Friendly Audio Quality Glass Badge (Section 3.1)
          const AudioQualityBadge(
            format: 'FLAC',
            sampleRate: 44100,
            bitDepth: 16,
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
                  value: _playbackPositionSec,
                  min: 0.0,
                  max: _trackDurationSec,
                  onChanged: (v) {
                    setState(() {
                      _playbackPositionSec = v;
                    });
                  },
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
                color: colorScheme.onSurface.withValues(alpha: 0.7),
                onPressed: () {},
              ),
              const SizedBox(width: 16),
              IconButton(
                icon: const Icon(Icons.skip_previous_rounded),
                iconSize: 36,
                color: colorScheme.onSurface,
                onPressed: widget.onPrevious,
              ),
              const SizedBox(width: 16),
              // Play/Pause Hero Button
              LiquidGlassContainer(
                width: 68,
                height: 68,
                padding: EdgeInsets.zero,
                borderRadius: 34,
                surfaceAlpha: 0.28,
                tintOverride: colorScheme.primary.withValues(alpha: 0.35),
                onTap: widget.onTogglePlay,
                child: Center(
                  child: Icon(
                    widget.isPlaying ? Icons.pause_rounded : Icons.play_arrow_rounded,
                    size: 36,
                    color: Colors.white,
                  ),
                ),
              ),
              const SizedBox(width: 16),
              IconButton(
                icon: const Icon(Icons.skip_next_rounded),
                iconSize: 36,
                color: colorScheme.onSurface,
                onPressed: widget.onNext,
              ),
              const SizedBox(width: 16),
              IconButton(
                icon: const Icon(Icons.repeat_rounded),
                iconSize: 22,
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
                        '(Not) A Devil',
                        style: AttuneTypography.soraTitleSmall(color: colorScheme.onSurface),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                const Flexible(
                  child: FittedBox(
                    fit: BoxFit.scaleDown,
                    child: TransitionConnectorPill(
                      archetype: 'Riff Handoff',
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
  }
}
