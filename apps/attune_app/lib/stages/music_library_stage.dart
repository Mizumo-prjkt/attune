import 'dart:io';
import 'package:flutter/material.dart';
import '../models/track_item.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// MusicLibraryStage implementing Stage 4 of 05_UI_DESIGN_SPEC.md:
/// Search bar, filter pills, Liked Songs / Heavy Rotation / Local Audio cards,
/// and live scanned library track list with real-time audio playback triggering.
class MusicLibraryStage extends StatefulWidget {
  const MusicLibraryStage({super.key});

  @override
  State<MusicLibraryStage> createState() => _MusicLibraryStageState();
}

class _MusicLibraryStageState extends State<MusicLibraryStage> {
  int _selectedFilter = 0;
  final TextEditingController _searchController = TextEditingController();
  String _searchQuery = '';

  final List<String> _filters = ['All Music', 'Rock', 'Pop', 'Electronic', 'Jazz', 'Acoustic'];

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final allTracks = controller.libraryTracks;
        final currentTrack = controller.currentTrack;

        // Apply search query filter
        var filteredTracks = allTracks.where((t) {
          if (_searchQuery.isEmpty) return true;
          final q = _searchQuery.toLowerCase();
          return t.title.toLowerCase().contains(q) ||
              t.artist.toLowerCase().contains(q) ||
              t.album.toLowerCase().contains(q);
        }).toList();

        // Apply genre filter pill
        switch (_selectedFilter) {
          case 1: // Rock
            filteredTracks = filteredTracks.where((t) {
              return t.genre == 'Rock' ||
                  t.title.toLowerCase().contains('rock') ||
                  t.artist.toLowerCase().contains('neru');
            }).toList();
            break;
          case 2: // Pop
            filteredTracks = filteredTracks.where((t) {
              return t.genre == 'Pop' || t.genre == 'J-Pop';
            }).toList();
            break;
          case 3: // Electronic
            filteredTracks = filteredTracks.where((t) {
              return t.genre == 'Electronic' ||
                  t.title.toLowerCase().contains('dance') ||
                  t.title.toLowerCase().contains('boogie') ||
                  t.title.toLowerCase().contains('synth');
            }).toList();
            break;
          case 4: // Jazz
            filteredTracks = filteredTracks.where((t) => t.genre == 'Jazz').toList();
            break;
          case 5: // Acoustic
            filteredTracks = filteredTracks.where((t) => t.genre == 'Acoustic').toList();
            break;
          case 0: // All Music
          default:
            break;
        }

        return ListView(
          physics: const BouncingScrollPhysics(),
          padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 16.0),
          children: [
            // 1. Search Bar
            LiquidGlassContainer(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
              borderRadius: 20,
              surfaceAlpha: 0.12,
              child: Row(
                children: [
                  Icon(Icons.search_rounded, color: colorScheme.onSurface.withValues(alpha: 0.6), size: 22),
                  const SizedBox(width: 12),
                  Expanded(
                    child: TextField(
                      controller: _searchController,
                      style: AttuneTypography.jakartaBodyMedium(color: colorScheme.onSurface),
                      decoration: InputDecoration(
                        hintText: 'Search ${allTracks.length} tracks, artists, albums...',
                        hintStyle: AttuneTypography.jakartaBodyMedium(
                          color: colorScheme.onSurface.withValues(alpha: 0.5),
                        ),
                        border: InputBorder.none,
                        isDense: true,
                        contentPadding: const EdgeInsets.symmetric(vertical: 8),
                      ),
                      onChanged: (val) => setState(() => _searchQuery = val.trim()),
                    ),
                  ),
                  if (_searchQuery.isNotEmpty)
                    IconButton(
                      icon: const Icon(Icons.close_rounded, size: 18),
                      color: colorScheme.onSurface.withValues(alpha: 0.6),
                      onPressed: () {
                        _searchController.clear();
                        setState(() => _searchQuery = '');
                      },
                    )
                  else
                    IconButton(
                      icon: controller.isScanning
                          ? const SizedBox(
                              width: 16,
                              height: 16,
                              child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                            )
                          : const Icon(Icons.refresh_rounded, size: 20),
                      color: colorScheme.onSurface.withValues(alpha: 0.6),
                      onPressed: controller.isScanning ? null : () => controller.scanLocalAudioDirectories(),
                    ),
                ],
              ),
            ),

            const SizedBox(height: 16),

            // 2. Filter Pills
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              physics: const BouncingScrollPhysics(),
              child: Row(
                children: List.generate(_filters.length, (i) {
                  final selected = _selectedFilter == i;
                  return Padding(
                    padding: const EdgeInsets.only(right: 8.0),
                    child: LiquidGlassContainer(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
                      borderRadius: 14,
                      surfaceAlpha: selected ? 0.28 : 0.08,
                      tintOverride: selected ? colorScheme.primary.withValues(alpha: 0.24) : null,
                      onTap: () => setState(() => _selectedFilter = i),
                      child: Text(
                        _filters[i],
                        style: AttuneTypography.soraBadge(
                          color: selected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.7),
                        ),
                      ),
                    ),
                  );
                }),
              ),
            ),

            const SizedBox(height: 20),

            // 3. Featured Quick Tiles
            Row(
              children: [
                Expanded(
                  child: LiquidGlassContainer(
                    padding: const EdgeInsets.all(16),
                    borderRadius: 24,
                    surfaceAlpha: 0.14,
                    tintOverride: colorScheme.primary.withValues(alpha: 0.18),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Container(
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: colorScheme.primary.withValues(alpha: 0.3),
                            borderRadius: BorderRadius.circular(12),
                          ),
                          child: const Icon(Icons.favorite_rounded, color: Colors.white, size: 22),
                        ),
                        const SizedBox(height: 14),
                        Text('Liked Songs', style: AttuneTypography.soraTitleSmall(color: Colors.white)),
                        const SizedBox(height: 2),
                        Text(
                          '${allTracks.length} local tracks',
                          style: AttuneTypography.jakartaBodySmall(
                            color: Colors.white.withValues(alpha: 0.7),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: LiquidGlassContainer(
                    padding: const EdgeInsets.all(16),
                    borderRadius: 24,
                    surfaceAlpha: 0.14,
                    tintOverride: colorScheme.secondary.withValues(alpha: 0.18),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Container(
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: colorScheme.secondary.withValues(alpha: 0.3),
                            borderRadius: BorderRadius.circular(12),
                          ),
                          child: const Icon(Icons.bolt_rounded, color: Colors.white, size: 22),
                        ),
                        const SizedBox(height: 14),
                        Text('Heavy Rotation', style: AttuneTypography.soraTitleSmall(color: Colors.white)),
                        const SizedBox(height: 2),
                        Text(
                          'Rock & Vocaloid',
                          style: AttuneTypography.jakartaBodySmall(
                            color: Colors.white.withValues(alpha: 0.7),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),

            const SizedBox(height: 24),

            // 4. Section Header with count
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4.0),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    'DISCOVERED AUDIO FILES (${filteredTracks.length})',
                    style: AttuneTypography.soraSectionHeader(color: colorScheme.primary),
                  ),
                  if (controller.isScanning)
                    Text(
                      'Scanning...',
                      style: AttuneTypography.soraBadge(color: colorScheme.secondary),
                    )
                  else
                    Text(
                      '${allTracks.length} total',
                      style: AttuneTypography.soraBadge(color: colorScheme.onSurface.withValues(alpha: 0.5)),
                    ),
                ],
              ),
            ),

            const SizedBox(height: 12),

            // 5. Track List
            if (filteredTracks.isEmpty)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 40.0),
                child: Center(
                  child: Text(
                    controller.isScanning ? 'Scanning filesystem...' : 'No tracks found matching query.',
                    style: AttuneTypography.jakartaBodyMedium(
                      color: colorScheme.onSurface.withValues(alpha: 0.5),
                    ),
                  ),
                ),
              )
            else
              ...List.generate(filteredTracks.length, (index) {
                final track = filteredTracks[index];
                final isCurrent = currentTrack?.id == track.id;

                return Padding(
                  padding: const EdgeInsets.only(bottom: 8.0),
                  child: LiquidGlassContainer(
                    padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                    borderRadius: 16,
                    surfaceAlpha: isCurrent ? 0.26 : 0.10,
                    tintOverride: isCurrent ? colorScheme.primary.withValues(alpha: 0.22) : null,
                    onTap: () => controller.playTrack(track),
                    child: Row(
                      children: [
                        // Artwork or Icon thumbnail
                        ClipRRect(
                          borderRadius: BorderRadius.circular(10),
                          child: Container(
                            width: 44,
                            height: 44,
                            color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
                            child: track.artworkFile != null && File(track.artworkFile!).existsSync()
                                ? Image.file(
                                    File(track.artworkFile!),
                                    fit: BoxFit.cover,
                                    errorBuilder: (context, error, stackTrace) => _buildFallbackThumb(track, colorScheme),
                                  )
                                : _buildFallbackThumb(track, colorScheme),
                          ),
                        ),
                        const SizedBox(width: 14),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                children: [
                                  if (isCurrent) ...[
                                    Icon(
                                      controller.isPlaying ? Icons.play_arrow_rounded : Icons.pause_rounded,
                                      size: 16,
                                      color: colorScheme.primary,
                                    ),
                                    const SizedBox(width: 4),
                                  ],
                                  Expanded(
                                    child: Text(
                                      track.title,
                                      style: AttuneTypography.soraTitleSmall(
                                        color: isCurrent ? colorScheme.primary : colorScheme.onSurface,
                                      ),
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 2),
                              Text(
                                '${track.artist} • ${track.format} • ${track.bpm > 0 ? "${track.bpm.toStringAsFixed(1)} BPM" : "${track.sampleRate} Hz"}',
                                style: AttuneTypography.jakartaBodySmall(
                                  color: colorScheme.onSurface.withValues(alpha: 0.6),
                                ),
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                              ),
                            ],
                          ),
                        ),
                        Text(
                          '${track.duration.inMinutes}:${(track.duration.inSeconds % 60).toString().padLeft(2, '0')}',
                          style: AttuneTypography.soraMetric(
                            color: colorScheme.onSurface.withValues(alpha: 0.5),
                            fontSize: 12,
                          ),
                        ),
                        const SizedBox(width: 6),
                        IconButton(
                          icon: Icon(
                            isCurrent && controller.isPlaying ? Icons.pause_circle_filled_rounded : Icons.play_circle_fill_rounded,
                            size: 26,
                            color: isCurrent ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.4),
                          ),
                          onPressed: () {
                            if (isCurrent) {
                              controller.togglePlay();
                            } else {
                              controller.playTrack(track);
                            }
                          },
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

  Widget _buildFallbackThumb(TrackItem track, ColorScheme colorScheme) {
    return Container(
      decoration: BoxDecoration(
        gradient: LinearGradient(
          colors: [
            track.seedColor.withValues(alpha: 0.8),
            track.seedColor.withValues(alpha: 0.4),
          ],
        ),
      ),
      child: Center(
        child: Icon(Icons.music_note_rounded, color: Colors.white.withValues(alpha: 0.9), size: 22),
      ),
    );
  }
}
