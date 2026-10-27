import 'dart:io';
import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Floating mini-player toolbar shown on main screens when audio is active.
/// Features a neon-like lighting box adjacent to the album artwork,
/// track metadata, and quick transport controls (previous, play/pause, next).
class FloatingMiniPlayer extends StatelessWidget {
  final VoidCallback onTap;

  const FloatingMiniPlayer({super.key, required this.onTap});

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final track = controller.currentTrack;
        if (track == null) return const SizedBox.shrink();

        final colorScheme = Theme.of(context).colorScheme;
        final accentColor = track.seedColor;

        return GestureDetector(
          onTap: onTap,
          behavior: HitTestBehavior.opaque,
          child: LiquidGlassContainer(
            borderRadius: 24,
            blurSigma: 32,
            surfaceAlpha: 0.26,
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            child: Row(
              children: [
                // 1. Neon-like lighting box adjacent to / wrapping album art
                Container(
                  width: 48,
                  height: 48,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(14),
                    boxShadow: [
                      BoxShadow(
                        color: accentColor.withValues(alpha: 0.45),
                        blurRadius: 14,
                        spreadRadius: 1,
                      ),
                    ],
                    border: Border.all(
                      color: accentColor.withValues(alpha: 0.8),
                      width: 1.5,
                    ),
                  ),
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: track.artworkFile != null && File(track.artworkFile!).existsSync()
                        ? Image.file(
                            File(track.artworkFile!),
                            fit: BoxFit.cover,
                          )
                        : Container(
                            color: accentColor.withValues(alpha: 0.3),
                            child: Icon(
                              Icons.music_note_rounded,
                              color: accentColor,
                              size: 26,
                            ),
                          ),
                  ),
                ),

                const SizedBox(width: 14),

                // 2. Track Title and Artist
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        track.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: AttuneTypography.soraSubheading(
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                          color: colorScheme.onSurface,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Row(
                        children: [
                          Flexible(
                            child: Text(
                              track.artist,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: AttuneTypography.jakartaBody(
                                fontSize: 12,
                                color: colorScheme.onSurface.withValues(alpha: 0.65),
                              ),
                            ),
                          ),
                          const SizedBox(width: 6),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                            decoration: BoxDecoration(
                              color: accentColor.withValues(alpha: 0.18),
                              borderRadius: BorderRadius.circular(6),
                            ),
                            child: Text(
                              track.format,
                              style: AttuneTypography.soraBadge(
                                color: accentColor,
                                fontSize: 9,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),

                // 3. Compact Transport Controls: Previous, Play/Pause, Next
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    IconButton(
                      icon: const Icon(Icons.skip_previous_rounded),
                      iconSize: 24,
                      visualDensity: VisualDensity.compact,
                      color: colorScheme.onSurface.withValues(alpha: 0.8),
                      onPressed: controller.skipPrevious,
                    ),
                    Container(
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: accentColor.withValues(alpha: 0.28),
                        border: Border.all(
                          color: accentColor.withValues(alpha: 0.6),
                          width: 1,
                        ),
                      ),
                      child: IconButton(
                        icon: Icon(
                          controller.isPlaying ? Icons.pause_rounded : Icons.play_arrow_rounded,
                        ),
                        iconSize: 26,
                        visualDensity: VisualDensity.compact,
                        color: colorScheme.onSurface,
                        onPressed: controller.togglePlay,
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.skip_next_rounded),
                      iconSize: 24,
                      visualDensity: VisualDensity.compact,
                      color: colorScheme.onSurface.withValues(alpha: 0.8),
                      onPressed: controller.skipNext,
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
