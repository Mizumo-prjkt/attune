import 'dart:io';
import 'package:flutter/material.dart';
import '../models/track_item.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// HomeStage: The primary landing and browsing hub of Attune.
/// Features quick search, standard musical genre filter pills,
/// hero jumping card, quick mixes, and all discovered audio tracks.
class HomeStage extends StatefulWidget {
  final VoidCallback? onOpenNowPlaying;

  const HomeStage({super.key, this.onOpenNowPlaying});

  @override
  State<HomeStage> createState() => _HomeStageState();
}

class _HomeStageState extends State<HomeStage> {
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
                        hintStyle: AttuneTypography.jakartaBody(
                          color: colorScheme.onSurface.withValues(alpha: 0.4),
                        ),
                        border: InputBorder.none,
                        isDense: true,
                      ),
                      onChanged: (val) => setState(() => _searchQuery = val),
                    ),
                  ),
                  if (_searchQuery.isNotEmpty)
                    GestureDetector(
                      onTap: () {
                        _searchController.clear();
                        setState(() => _searchQuery = '');
                      },
                      child: Icon(Icons.clear_rounded, size: 20, color: colorScheme.onSurface.withValues(alpha: 0.5)),
                    )
                  else
                    GestureDetector(
                      onTap: () => controller.scanLocalAudioDirectories(forceRefresh: true),
                      child: Icon(
                        Icons.refresh_rounded,
                        size: 20,
                        color: controller.isScanning
                            ? colorScheme.primary
                            : colorScheme.onSurface.withValues(alpha: 0.4),
                      ),
                    ),
                ],
              ),
            ),

            const SizedBox(height: 14),

            // 2. Standard Musical Genre Filter Pills
            SizedBox(
              height: 36,
              child: ListView.separated(
                scrollDirection: Axis.horizontal,
                physics: const BouncingScrollPhysics(),
                itemCount: _filters.length,
                separatorBuilder: (_, _) => const SizedBox(width: 8),
                itemBuilder: (context, index) {
                  final isSelected = _selectedFilter == index;
                  return GestureDetector(
                    onTap: () => setState(() => _selectedFilter = index),
                    child: LiquidGlassContainer(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                      borderRadius: 18,
                      surfaceAlpha: isSelected ? 0.32 : 0.08,
                      borderColor: isSelected ? colorScheme.primary.withValues(alpha: 0.6) : null,
                      child: Center(
                        child: Text(
                          _filters[index],
                          style: AttuneTypography.soraBadge(
                            color: isSelected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.7),
                            fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
                          ),
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),

            const SizedBox(height: 20),

            // 3. Quick Action Feature Cards
            Row(
              children: [
                Expanded(
                  child: _buildQuickCard(
                    title: 'Liked Songs',
                    subtitle: '${allTracks.length} local tracks',
                    icon: Icons.favorite_rounded,
                    accentColor: colorScheme.primary,
                    colorScheme: colorScheme,
                    onTap: () {
                      if (allTracks.isNotEmpty) {
                        controller.playTrack(allTracks.first);
                        widget.onOpenNowPlaying?.call();
                      }
                    },
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: _buildQuickCard(
                    title: 'Heavy Rotation',
                    subtitle: 'Rock & Vocaloid',
                    icon: Icons.bolt_rounded,
                    accentColor: colorScheme.tertiary,
                    colorScheme: colorScheme,
                    onTap: () {
                      final rockTracks = allTracks.where((t) => t.genre == 'Rock' || t.genre == 'Electronic').toList();
                      if (rockTracks.isNotEmpty) {
                        controller.playTrack(rockTracks.first);
                        widget.onOpenNowPlaying?.call();
                      }
                    },
                  ),
                ),
              ],
            ),

            const SizedBox(height: 24),

            // 4. Section Header: Discovered Audio Files
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Text(
                    'DISCOVERED AUDIO FILES (${filteredTracks.length})',
                    style: AttuneTypography.soraBadge(
                      color: colorScheme.primary,
                      fontWeight: FontWeight.w700,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  '${allTracks.length} total',
                  style: AttuneTypography.soraBadge(
                    color: colorScheme.onSurface.withValues(alpha: 0.4),
                  ),
                ),
              ],
            ),

            const SizedBox(height: 12),

            // 5. Dynamic Track List
            if (filteredTracks.isEmpty)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 40),
                child: Center(
                  child: Text(
                    'No tracks found matching "$_searchQuery"',
                    style: AttuneTypography.jakartaBody(
                      color: colorScheme.onSurface.withValues(alpha: 0.5),
                    ),
                  ),
                ),
              )
            else
              ...filteredTracks.map((track) {
                final isCurrent = currentTrack?.id == track.id;
                return Padding(
                  padding: const EdgeInsets.only(bottom: 8.0),
                  child: _buildTrackRow(
                    track: track,
                    isCurrent: isCurrent,
                    isPlaying: isCurrent && controller.isPlaying,
                    colorScheme: colorScheme,
                    onTap: () {
                      controller.playTrack(track);
                      widget.onOpenNowPlaying?.call();
                    },
                  ),
                );
              }),

            const SizedBox(height: 90), // Spacing for floating mini player
          ],
        );
      },
    );
  }

  Widget _buildQuickCard({
    required String title,
    required String subtitle,
    required IconData icon,
    required Color accentColor,
    required ColorScheme colorScheme,
    required VoidCallback onTap,
  }) {
    return GestureDetector(
      onTap: onTap,
      child: LiquidGlassContainer(
        padding: const EdgeInsets.all(16),
        borderRadius: 22,
        surfaceAlpha: 0.14,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: accentColor.withValues(alpha: 0.25),
                borderRadius: BorderRadius.circular(14),
              ),
              child: Icon(icon, color: accentColor, size: 22),
            ),
            const SizedBox(height: 14),
            Text(
              title,
              style: AttuneTypography.soraSubheading(
                fontWeight: FontWeight.w600,
                color: colorScheme.onSurface,
              ),
            ),
            const SizedBox(height: 3),
            Text(
              subtitle,
              style: AttuneTypography.jakartaBody(
                color: colorScheme.onSurface.withValues(alpha: 0.5),
                fontSize: 12,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildTrackRow({
    required TrackItem track,
    required bool isCurrent,
    required bool isPlaying,
    required ColorScheme colorScheme,
    required VoidCallback onTap,
  }) {
    final minutes = track.duration.inMinutes;
    final seconds = track.duration.inSeconds % 60;
    final durationStr = '$minutes:${seconds.toString().padLeft(2, '0')}';

    return GestureDetector(
      onTap: onTap,
      behavior: HitTestBehavior.opaque,
      child: LiquidGlassContainer(
        padding: const EdgeInsets.all(12),
        borderRadius: 18,
        surfaceAlpha: isCurrent ? 0.26 : 0.08,
        borderColor: isCurrent ? colorScheme.primary.withValues(alpha: 0.5) : null,
        child: Row(
          children: [
            // Album Art Thumbnail
            Container(
              width: 48,
              height: 48,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(12),
                gradient: LinearGradient(
                  colors: [track.seedColor.withValues(alpha: 0.6), track.seedColor.withValues(alpha: 0.2)],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
              ),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: track.artworkFile != null && File(track.artworkFile!).existsSync()
                    ? Image.file(
                        File(track.artworkFile!),
                        fit: BoxFit.cover,
                      )
                    : Icon(
                        Icons.music_note_rounded,
                        color: Colors.white.withValues(alpha: 0.7),
                        size: 24,
                      ),
              ),
            ),

            const SizedBox(width: 14),

            // Track Title and Artist
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      if (isCurrent) ...[
                        Icon(Icons.play_arrow_rounded, color: colorScheme.primary, size: 16),
                        const SizedBox(width: 4),
                      ],
                      Expanded(
                        child: Text(
                          track.title,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: AttuneTypography.soraSubheading(
                            fontWeight: FontWeight.w600,
                            color: isCurrent ? colorScheme.primary : colorScheme.onSurface,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 3),
                  Text(
                    '${track.artist} • ${track.format} • ${track.bpm.toStringAsFixed(1)} BPM',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: AttuneTypography.jakartaBody(
                      color: colorScheme.onSurface.withValues(alpha: 0.55),
                      fontSize: 12,
                    ),
                  ),
                ],
              ),
            ),

            const SizedBox(width: 12),

            // Duration & Play State
            Text(
              durationStr,
              style: AttuneTypography.soraBadge(
                color: colorScheme.onSurface.withValues(alpha: 0.5),
              ),
            ),
            const SizedBox(width: 10),
            Icon(
              isPlaying ? Icons.pause_circle_filled_rounded : Icons.play_circle_fill_rounded,
              color: isCurrent ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.4),
              size: 24,
            ),
          ],
        ),
      ),
    );
  }
}
