import 'dart:io';
import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/transition_connector_pill.dart';
import '../widgets/transition_details_dialog.dart';
import 'queue_add_tracks_stage.dart';

/// PlaylistQueueStage implementing Stage 2 of 05_UI_DESIGN_SPEC.md:
/// Now Playing mini-header, subtle inline transition connector pill,
/// filter chips, Smart Shuffle / Autoplay toggles, reactive queued track list,
/// dedicated Add Tracks navigation, Smart Shuffle <20 advisory banner,
/// and reordering lock in Smart Shuffle mode.
class PlaylistQueueStage extends StatefulWidget {
  const PlaylistQueueStage({super.key});

  @override
  State<PlaylistQueueStage> createState() => _PlaylistQueueStageState();
}

class _PlaylistQueueStageState extends State<PlaylistQueueStage> {
  int _selectedFilterIndex = 0;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final current = controller.currentTrack;
        final queue = controller.queue;
        final isSmartShuffle = controller.shuffleMode == ShuffleMode.smart;

        return ListView(
          physics: const BouncingScrollPhysics(),
          padding: const EdgeInsets.symmetric(horizontal: 14.0, vertical: 14.0),
          children: [
            // 1. Now Playing Mini Header (if track is loaded)
            if (current != null)
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
                        child: current.artworkFile != null && File(current.artworkFile!).existsSync()
                            ? Image.file(File(current.artworkFile!), fit: BoxFit.cover)
                            : Container(
                                decoration: BoxDecoration(
                                  gradient: LinearGradient(
                                    colors: [
                                      current.seedColor,
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
                            current.title,
                            style: AttuneTypography.soraTitleSmall(color: colorScheme.onSurface),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            current.artist,
                            style: AttuneTypography.jakartaBodySmall(
                              color: colorScheme.onSurface.withValues(alpha: 0.65),
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                    IconButton(
                      icon: Icon(
                        controller.isFavorite(current.id)
                            ? Icons.favorite_rounded
                            : Icons.favorite_border_rounded,
                        color: controller.isFavorite(current.id)
                            ? const Color(0xFFFF4081)
                            : colorScheme.onSurface.withValues(alpha: 0.5),
                        size: 20,
                      ),
                      tooltip: controller.isFavorite(current.id) ? 'Remove from favorites' : 'Favorite current track',
                      onPressed: () => controller.toggleFavorite(current.id),
                    ),
                    const SizedBox(width: 4),
                    Icon(
                      controller.isPlaying ? Icons.graphic_eq_rounded : Icons.pause_circle_outline_rounded,
                      color: colorScheme.primary,
                      size: 22,
                    ),
                  ],
                ),
              ),

            // Guaranteed breathing room between Now Playing and chips/transition pill
            if (current != null) ...[
              if (isSmartShuffle) ...[
                const SizedBox(height: 14),
                Center(
                  child: TransitionConnectorPill(
                    archetype: controller.activeArchetype,
                    durationSeconds: (controller.lastTransitionDurationSec != null && controller.lastTransitionDurationSec! > 0)
                        ? controller.lastTransitionDurationSec!
                        : 1.4,
                    confidenceScore: 0.94,
                    onTap: () => showTransitionDetailsDialog(context, controller, colorScheme),
                  ),
                ),
                const SizedBox(height: 18),
              ] else ...[
                const SizedBox(height: 18),
              ],
            ] else ...[
              const SizedBox(height: 12),
            ],

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

            const SizedBox(height: 14),

            // 4. Smart Shuffle Low Track Count Advisory Banner (<20 tracks)
            if (_selectedFilterIndex == 0 && isSmartShuffle && queue.length < 20) ...[
              LiquidGlassContainer(
                padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                borderRadius: 18,
                surfaceAlpha: 0.16,
                tintOverride: const Color(0xFFFFA000).withValues(alpha: 0.12),
                borderColor: const Color(0xFFFFC107).withValues(alpha: 0.35),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Icon(Icons.auto_awesome_rounded, color: Color(0xFFFFC107), size: 20),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Smart Shuffle Advisory (${queue.length}/20 tracks)',
                            style: AttuneTypography.soraBadge(color: const Color(0xFFFFC107), fontSize: 11),
                          ),
                          const SizedBox(height: 3),
                          Text(
                            'Smart Shuffle works best with at least 20 tracks to compute optimal Camelot key alignments and energy curves. Add more tracks for seamless transitions.',
                            style: AttuneTypography.jakartaBodySmall(
                              color: colorScheme.onSurface.withValues(alpha: 0.75),
                              fontSize: 11,
                            ),
                          ),
                          const SizedBox(height: 8),
                          GestureDetector(
                            onTap: () {
                              Navigator.of(context).push(
                                MaterialPageRoute(builder: (_) => const QueueAddTracksStage()),
                              );
                            },
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Text(
                                  '+ Add Tracks to Queue',
                                  style: AttuneTypography.soraBadge(color: colorScheme.primary, fontSize: 11),
                                ),
                                const SizedBox(width: 4),
                                Icon(Icons.arrow_forward_rounded, size: 12, color: colorScheme.primary),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 14),
            ],

            // 5. Header / Action Bar depending on active filter
            if (_selectedFilterIndex == 0)
              () {
                String shuffleLabel;
                IconData shuffleIcon;
                bool isShuffleActive;
                String? shuffleTooltip;
                switch (controller.shuffleMode) {
                  case ShuffleMode.off:
                    shuffleLabel = 'Shuffle Off';
                    shuffleIcon = Icons.shuffle_rounded;
                    isShuffleActive = false;
                    shuffleTooltip = controller.isSmartShuffleSupported
                        ? 'Linear playback order'
                        : 'Linear playback order (Smart Shuffle unavailable in System Codec)';
                    break;
                  case ShuffleMode.standard:
                    shuffleLabel = controller.isSmartShuffleSupported ? 'Basic Shuffle' : 'Shuffle (Standard)';
                    shuffleIcon = Icons.shuffle_rounded;
                    isShuffleActive = true;
                    shuffleTooltip = controller.isSmartShuffleSupported
                        ? 'Randomized queue sequence'
                        : 'Randomized queue sequence (Smart Shuffle unavailable in System Codec)';
                    break;
                  case ShuffleMode.smart:
                    shuffleLabel = 'Smart Shuffle';
                    shuffleIcon = Icons.auto_awesome_rounded;
                    isShuffleActive = true;
                    shuffleTooltip = 'Profile & BPM-matched smart transition shuffle (reordering automated)';
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
                      tooltip: shuffleTooltip,
                      onTap: controller.cycleShuffleMode,
                    ),
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        // Add Tracks Action
                        GestureDetector(
                          onTap: () {
                            Navigator.of(context).push(
                              MaterialPageRoute(builder: (_) => const QueueAddTracksStage()),
                            );
                          },
                          child: Container(
                            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
                            decoration: BoxDecoration(
                              color: colorScheme.primary.withValues(alpha: 0.16),
                              borderRadius: BorderRadius.circular(10),
                              border: Border.all(
                                color: colorScheme.primary.withValues(alpha: 0.40),
                                width: 1,
                              ),
                            ),
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Icon(Icons.add_rounded, size: 14, color: colorScheme.primary),
                                const SizedBox(width: 4),
                                Text(
                                  'Add Tracks',
                                  style: AttuneTypography.soraBadge(color: colorScheme.primary, fontSize: 11),
                                ),
                              ],
                            ),
                          ),
                        ),
                        if (queue.isNotEmpty) ...[
                          const SizedBox(width: 8),
                          // Clear Queue Action
                          GestureDetector(
                            onTap: () {
                              controller.clearQueue();
                            },
                            child: Container(
                              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
                              decoration: BoxDecoration(
                                color: Colors.white.withValues(alpha: 0.05),
                                borderRadius: BorderRadius.circular(10),
                                border: Border.all(
                                  color: Colors.white.withValues(alpha: 0.08),
                                  width: 1,
                                ),
                              ),
                              child: Text(
                                'Clear',
                                style: AttuneTypography.jakartaBodySmall(
                                  color: colorScheme.onSurface.withValues(alpha: 0.50),
                                  fontSize: 11,
                                ),
                              ),
                            ),
                          ),
                        ],
                      ],
                    ),
                  ],
                );
              }()
            else if (_selectedFilterIndex == 1)
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    '${controller.favoriteTracks.length} FAVORITE TRACKS',
                    style: AttuneTypography.soraBadge(color: colorScheme.primary, fontSize: 11),
                  ),
                  if (controller.favoriteTracks.isNotEmpty)
                    GestureDetector(
                      onTap: () {
                        for (final t in controller.favoriteTracks) {
                          if (!controller.queue.any((q) => q.id == t.id)) {
                            controller.addToQueue(t);
                          }
                        }
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(
                            content: Text('Added ${controller.favoriteTracks.length} favorites to queue'),
                            duration: const Duration(seconds: 1),
                          ),
                        );
                      },
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
                        decoration: BoxDecoration(
                          color: colorScheme.primary.withValues(alpha: 0.16),
                          borderRadius: BorderRadius.circular(10),
                          border: Border.all(color: colorScheme.primary.withValues(alpha: 0.3)),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(Icons.playlist_add_rounded, size: 14, color: colorScheme.primary),
                            const SizedBox(width: 4),
                            Text(
                              '+ Add All to Queue',
                              style: AttuneTypography.soraBadge(color: colorScheme.primary, fontSize: 11),
                            ),
                          ],
                        ),
                      ),
                    ),
                ],
              )
            else if (_selectedFilterIndex == 2)
              Text(
                '${controller.getRepeatHookTracks().length} RECENT & REPEAT TRACKS',
                style: AttuneTypography.soraBadge(color: colorScheme.primary, fontSize: 11),
              )
            else if (_selectedFilterIndex == 3)
              Text(
                'SMART SHUFFLE MIX CANDIDATES',
                style: AttuneTypography.soraBadge(color: colorScheme.primary, fontSize: 11),
              ),

            const SizedBox(height: 12),

            // 6. Tracks List or Empty State based on active filter
            () {
              List<dynamic> displayTracks;
              String emptyTitle;
              String emptySubtitle;
              IconData emptyIcon;
              bool isQueueView = _selectedFilterIndex == 0;
              bool allowReorder = isQueueView && !isSmartShuffle;

              switch (_selectedFilterIndex) {
                case 1:
                  displayTracks = controller.favoriteTracks;
                  emptyTitle = 'No Favorite Tracks';
                  emptySubtitle = 'Tap the heart icon on any song to add it to your Favorites.';
                  emptyIcon = Icons.favorite_border_rounded;
                  break;
                case 2:
                  displayTracks = controller.getRepeatHookTracks();
                  emptyTitle = 'No Playback History Yet';
                  emptySubtitle = 'Tracks you listen to frequently will appear here.';
                  emptyIcon = Icons.history_rounded;
                  break;
                case 3:
                  displayTracks = controller.lastEvaluatedCandidates.isNotEmpty
                      ? controller.lastEvaluatedCandidates.map((c) => c.track).toList()
                      : controller.getRepeatHookTracks();
                  emptyTitle = 'No Smart Mix Candidates';
                  emptySubtitle = 'Smart Shuffle will populate transition candidates during playback.';
                  emptyIcon = Icons.auto_awesome_rounded;
                  break;
                case 0:
                default:
                  displayTracks = queue;
                  emptyTitle = 'Your Play Queue is Empty';
                  emptySubtitle = 'Add songs from your library to start listening, or tap below to browse and build your queue.';
                  emptyIcon = Icons.queue_music_rounded;
                  break;
              }

              if (displayTracks.isEmpty) {
                return _buildEmptyStateCard(
                  context: context,
                  colorScheme: colorScheme,
                  title: emptyTitle,
                  subtitle: emptySubtitle,
                  icon: emptyIcon,
                  showAddButton: isQueueView,
                );
              }

              if (!allowReorder) {
                return Column(
                  children: List.generate(displayTracks.length, (index) {
                    return _buildQueueItem(
                      index: index,
                      item: displayTracks[index],
                      controller: controller,
                      colorScheme: colorScheme,
                      canReorder: false,
                      isQueueView: isQueueView,
                    );
                  }),
                );
              }

              // Standard / Shuffle Off Queue: Interactive Reorderable List
              return ReorderableListView.builder(
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                itemCount: displayTracks.length,
                onReorderItem: (oldIndex, newIndex) {
                  controller.reorderQueue(oldIndex, newIndex);
                },
                itemBuilder: (context, index) {
                  return KeyedSubtree(
                    key: ValueKey('queue_item_${displayTracks[index].id}_$index'),
                    child: _buildQueueItem(
                      index: index,
                      item: displayTracks[index],
                      controller: controller,
                      colorScheme: colorScheme,
                      canReorder: true,
                      isQueueView: true,
                    ),
                  );
                },
              );
            }(),
          ],
        );
      },
    );
  }

  Widget _buildEmptyStateCard({
    required BuildContext context,
    required ColorScheme colorScheme,
    required String title,
    required String subtitle,
    required IconData icon,
    bool showAddButton = false,
  }) {
    return LiquidGlassContainer(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 32),
      borderRadius: 22,
      surfaceAlpha: 0.10,
      child: Column(
        children: [
          Icon(icon, size: 48, color: colorScheme.primary.withValues(alpha: 0.45)),
          const SizedBox(height: 14),
          Text(
            title,
            style: AttuneTypography.soraTitleMedium(color: colorScheme.onSurface),
          ),
          const SizedBox(height: 6),
          Text(
            subtitle,
            style: AttuneTypography.jakartaBodySmall(
              color: colorScheme.onSurface.withValues(alpha: 0.60),
            ),
            textAlign: TextAlign.center,
          ),
          if (showAddButton) ...[
            const SizedBox(height: 18),
            FilledButton.icon(
              style: FilledButton.styleFrom(
                backgroundColor: colorScheme.primary,
                foregroundColor: colorScheme.onPrimary,
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
              ),
              icon: const Icon(Icons.add_rounded, size: 20),
              label: Text('Add Tracks to Queue', style: AttuneTypography.soraBadge(color: colorScheme.onPrimary)),
              onPressed: () {
                Navigator.of(context).push(
                  MaterialPageRoute(builder: (_) => const QueueAddTracksStage()),
                );
              },
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildQueueItem({
    required int index,
    required dynamic item,
    required PlaybackController controller,
    required ColorScheme colorScheme,
    required bool canReorder,
    bool isQueueView = true,
  }) {
    final isCurrent = isQueueView && index == controller.currentQueueIndex;
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
                    '${item.artist}${item.hasBpm ? " • ${item.formattedBpm}" : " • ${item.format}"}',
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
            const SizedBox(width: 4),

            // 1-Tap Heart Favorite Toggle
            IconButton(
              icon: Icon(
                controller.isFavorite(item.id)
                    ? Icons.favorite_rounded
                    : Icons.favorite_border_rounded,
                size: 18,
                color: controller.isFavorite(item.id)
                    ? const Color(0xFFFF4081)
                    : colorScheme.onSurface.withValues(alpha: 0.35),
              ),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 26, minHeight: 26),
              tooltip: controller.isFavorite(item.id) ? 'Remove from favorites' : 'Add to favorites',
              onPressed: () {
                controller.toggleFavorite(item.id);
              },
            ),

            // Action button: Remove if queue view, Add to queue if other view
            if (isQueueView)
              IconButton(
                icon: Icon(
                  Icons.close_rounded,
                  size: 16,
                  color: colorScheme.onSurface.withValues(alpha: 0.35),
                ),
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(minWidth: 26, minHeight: 26),
                tooltip: 'Remove from queue',
                onPressed: () {
                  controller.removeFromQueue(index);
                },
              )
            else
              IconButton(
                icon: Icon(
                  Icons.playlist_add_rounded,
                  size: 18,
                  color: colorScheme.primary.withValues(alpha: 0.8),
                ),
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(minWidth: 26, minHeight: 26),
                tooltip: 'Add to queue',
                onPressed: () {
                  controller.addToQueue(item);
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text('Added "${item.title}" to queue'),
                      duration: const Duration(seconds: 1),
                    ),
                  );
                },
              ),

            if (canReorder) ...[
              const SizedBox(width: 2),
              ReorderableDragStartListener(
                index: index,
                child: Icon(
                  Icons.drag_handle_rounded,
                  size: 18,
                  color: colorScheme.onSurface.withValues(alpha: 0.35),
                ),
              ),
            ] else if (isQueueView && controller.shuffleMode == ShuffleMode.smart) ...[
              const SizedBox(width: 2),
              Tooltip(
                message: 'Smart Shuffle dynamically selects upcoming tracks based on Camelot harmony & BPM',
                child: Icon(
                  Icons.auto_awesome_rounded,
                  size: 14,
                  color: colorScheme.primary.withValues(alpha: 0.40),
                ),
              ),
            ],
          ],
        ),
      ),
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
    String? tooltip,
    required VoidCallback onTap,
  }) {
    final chip = GestureDetector(
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

    if (tooltip != null) {
      return Tooltip(message: tooltip, child: chip);
    }
    return chip;
  }
}
