import 'dart:io';
import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/transition_connector_pill.dart';

/// PlaylistQueueStage implementing Stage 2 of 05_UI_DESIGN_SPEC.md:
/// Now Playing mini-header, subtle inline transition connector pill,
/// filter chips, Smart Shuffle / Autoplay toggles, and reactive queued track list.
class PlaylistQueueStage extends StatefulWidget {
  const PlaylistQueueStage({super.key});

  @override
  State<PlaylistQueueStage> createState() => _PlaylistQueueStageState();
}

class _PlaylistQueueStageState extends State<PlaylistQueueStage> {
  int _selectedFilterIndex = 0;
  bool _autoplay = true;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final current = controller.currentTrack;
        final queue = controller.queue;

        return ListView(
          physics: const BouncingScrollPhysics(),
          padding: const EdgeInsets.symmetric(horizontal: 14.0, vertical: 14.0),
          children: [
            // 1. Now Playing Mini Header
            LiquidGlassContainer(
              padding: const EdgeInsets.all(14),
              borderRadius: 20,
              surfaceAlpha: 0.14,
              child: Row(
                children: [
                  ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: Container(
                      width: 48,
                      height: 48,
                      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
                      child: current?.artworkFile != null && File(current!.artworkFile!).existsSync()
                          ? Image.file(File(current.artworkFile!), fit: BoxFit.cover)
                          : Container(
                              decoration: BoxDecoration(
                                gradient: LinearGradient(
                                  colors: [
                                    current?.seedColor ?? colorScheme.primary,
                                    colorScheme.secondary,
                                  ],
                                ),
                              ),
                              child: const Icon(Icons.equalizer_rounded, color: Colors.white, size: 24),
                            ),
                    ),
                  ),
                  const SizedBox(width: 14),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'NOW PLAYING',
                          style: AttuneTypography.soraSectionHeader(color: colorScheme.primary),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          current?.title ?? 'Rabbit Hole',
                          style: AttuneTypography.soraTitleSmall(color: colorScheme.onSurface),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        Text(
                          current?.artist ?? 'DECO*27 feat. Hatsune Miku',
                          style: AttuneTypography.jakartaBodySmall(
                            color: colorScheme.onSurface.withValues(alpha: 0.65),
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ],
                    ),
                  ),
                  Icon(
                    controller.isPlaying ? Icons.graphic_eq_rounded : Icons.pause_circle_outline_rounded,
                    color: colorScheme.primary,
                    size: 22,
                  ),
                ],
              ),
            ),

            const SizedBox(height: 12),

            // 2. Subtle Inline Transition Connector Pill (Section 3.2)
            Center(
              child: TransitionConnectorPill(
                archetype: controller.activeArchetype,
                durationSeconds: 1.4,
                confidenceScore: 0.94,
              ),
            ),

            const SizedBox(height: 16),

            // 3. Category Filter Chips
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              physics: const BouncingScrollPhysics(),
              child: Row(
                children: [
                  _buildFilterChip('Up Next', 0, colorScheme),
                  const SizedBox(width: 8),
                  _buildFilterChip('Favorites', 1, colorScheme),
                  const SizedBox(width: 8),
                  _buildFilterChip('History', 2, colorScheme),
                  const SizedBox(width: 8),
                  _buildFilterChip('Smart Mix', 3, colorScheme),
                ],
              ),
            ),

            const SizedBox(height: 16),

            // 4. Quick Toggles (Tri-State Shuffle & Autoplay)
            () {
              String shuffleLabel;
              IconData shuffleIcon;
              bool isShuffleActive;
              switch (controller.shuffleMode) {
                case ShuffleMode.off:
                  shuffleLabel = 'Shuffle Off';
                  shuffleIcon = Icons.shuffle_rounded;
                  isShuffleActive = false;
                  break;
                case ShuffleMode.standard:
                  shuffleLabel = 'Basic Shuffle';
                  shuffleIcon = Icons.shuffle_rounded;
                  isShuffleActive = true;
                  break;
                case ShuffleMode.smart:
                  shuffleLabel = 'Smart Shuffle';
                  shuffleIcon = Icons.auto_awesome_rounded;
                  isShuffleActive = true;
                  break;
              }

              return Wrap(
                alignment: WrapAlignment.spaceBetween,
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 8,
                runSpacing: 8,
                children: [
                  _buildActionChip(
                    icon: shuffleIcon,
                    label: shuffleLabel,
                    active: isShuffleActive,
                    colorScheme: colorScheme,
                    onTap: controller.cycleShuffleMode,
                  ),
                  _buildActionChip(
                    icon: Icons.all_inclusive_rounded,
                    label: 'Autoplay',
                    active: _autoplay,
                    colorScheme: colorScheme,
                    onTap: () => setState(() => _autoplay = !_autoplay),
                  ),
                  Text(
                    '${queue.length} songs',
                    style: AttuneTypography.jakartaBodySmall(
                      color: colorScheme.onSurface.withValues(alpha: 0.5),
                    ),
                  ),
                ],
              );
            }(),

            const SizedBox(height: 12),

            // 5. Queued Streams List
            ...List.generate(queue.length, (index) {
              final item = queue[index];
              final isCurrent = index == controller.currentQueueIndex;
              final durMin = item.duration.inMinutes;
              final durSec = (item.duration.inSeconds % 60).toString().padLeft(2, '0');

              return Padding(
                padding: const EdgeInsets.only(bottom: 8.0),
                child: LiquidGlassContainer(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 10),
                  borderRadius: 16,
                  surfaceAlpha: isCurrent ? 0.22 : 0.10,
                  tintOverride: isCurrent ? colorScheme.primary.withValues(alpha: 0.20) : null,
                  onTap: () => controller.playTrack(item),
                  child: Row(
                    children: [
                      Text(
                        '${index + 1}',
                        style: AttuneTypography.soraMetric(
                          fontSize: 12,
                          weight: FontWeight.w600,
                          color: isCurrent ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.4),
                        ),
                      ),
                      const SizedBox(width: 8),
                      ClipRRect(
                        borderRadius: BorderRadius.circular(10),
                        child: Container(
                          width: 38,
                          height: 38,
                          color: item.seedColor.withValues(alpha: 0.4),
                          child: item.artworkFile != null && File(item.artworkFile!).existsSync()
                              ? Image.file(File(item.artworkFile!), fit: BoxFit.cover)
                              : Center(
                                  child: Icon(
                                    isCurrent ? Icons.play_arrow_rounded : Icons.music_note_rounded,
                                    color: Colors.white,
                                    size: 18,
                                  ),
                                ),
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              item.title,
                              style: AttuneTypography.soraTitleSmall(
                                color: isCurrent ? colorScheme.primary : colorScheme.onSurface,
                              ),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                            const SizedBox(height: 2),
                            Text(
                              '${item.artist} • ${item.bpm.toStringAsFixed(1)} BPM',
                              style: AttuneTypography.jakartaBodySmall(
                                color: colorScheme.onSurface.withValues(alpha: 0.6),
                              ),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(width: 6),
                      Text(
                        '$durMin:$durSec',
                        style: AttuneTypography.soraMetric(
                          fontSize: 11,
                          color: colorScheme.onSurface.withValues(alpha: 0.5),
                        ),
                      ),
                      const SizedBox(width: 6),
                      Icon(
                        Icons.drag_handle_rounded,
                        size: 18,
                        color: colorScheme.onSurface.withValues(alpha: 0.3),
                      ),
                    ],
                  ),
                ),
              );
            }),
          ],
        );
      },
    );
  }

  Widget _buildFilterChip(String label, int index, ColorScheme colorScheme) {
    final active = _selectedFilterIndex == index;
    return GestureDetector(
      onTap: () => setState(() => _selectedFilterIndex = index),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
        decoration: BoxDecoration(
          color: active ? colorScheme.primary.withValues(alpha: 0.24) : Colors.white.withValues(alpha: 0.06),
          borderRadius: BorderRadius.circular(12),
          border: Border.all(
            color: active ? colorScheme.primary.withValues(alpha: 0.5) : Colors.white.withValues(alpha: 0.08),
            width: 1,
          ),
        ),
        child: Text(
          label,
          style: AttuneTypography.soraBadge(
            color: active ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.6),
          ),
        ),
      ),
    );
  }

  Widget _buildActionChip({
    required IconData icon,
    required String label,
    required bool active,
    required ColorScheme colorScheme,
    required VoidCallback onTap,
  }) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
        decoration: BoxDecoration(
          color: active ? colorScheme.primary.withValues(alpha: 0.16) : Colors.white.withValues(alpha: 0.05),
          borderRadius: BorderRadius.circular(10),
          border: Border.all(
            color: active ? colorScheme.primary.withValues(alpha: 0.4) : Colors.white.withValues(alpha: 0.06),
            width: 1,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              icon,
              size: 14,
              color: active ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.5),
            ),
            const SizedBox(width: 5),
            Text(
              label,
              style: AttuneTypography.jakartaBodySmall(
                color: active ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.7),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
