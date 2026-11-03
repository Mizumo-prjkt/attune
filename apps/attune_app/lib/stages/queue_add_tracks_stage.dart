import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/viewport_aware_artwork.dart';

/// Dedicated full-screen stage for adding tracks from the local audio library
/// into the active playback queue. Replaces popup modals with a rich Material You 3 / Liquid Glass interface.
class QueueAddTracksStage extends StatefulWidget {
  const QueueAddTracksStage({super.key});

  @override
  State<QueueAddTracksStage> createState() => _QueueAddTracksStageState();
}

class _QueueAddTracksStageState extends State<QueueAddTracksStage> {
  final TextEditingController _searchController = TextEditingController();
  final Set<String> _selectedTrackIds = {};
  String _searchQuery = '';
  String _selectedGenre = 'All';

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
        final libraryTracks = controller.libraryTracks;
        final currentQueueIds = controller.queue.map((t) => t.id).toSet();

        // Extract distinct genres from library
        final genres = <String>{'All', 'Lossless', 'Hi-Res'};
        for (final t in libraryTracks) {
          if (t.genre.trim().isNotEmpty && t.genre != 'Unknown') {
            genres.add(t.genre.trim());
          }
        }
        final genreList = genres.toList();

        // Filter tracks according to search query and genre chip
        final filteredTracks = libraryTracks.where((track) {
          if (_searchQuery.isNotEmpty) {
            final q = _searchQuery.toLowerCase();
            final matchesTitle = track.title.toLowerCase().contains(q);
            final matchesArtist = track.artist.toLowerCase().contains(q);
            final matchesAlbum = track.album.toLowerCase().contains(q);
            if (!matchesTitle && !matchesArtist && !matchesAlbum) {
              return false;
            }
          }

          if (_selectedGenre == 'Lossless') {
            return track.isLossless;
          } else if (_selectedGenre == 'Hi-Res') {
            return track.sampleRate > 48000 || track.bitDepth > 16;
          } else if (_selectedGenre != 'All') {
            return track.genre.trim().toLowerCase() == _selectedGenre.toLowerCase();
          }

          return true;
        }).toList();

        final allFilteredSelected = filteredTracks.isNotEmpty &&
            filteredTracks.every((t) => _selectedTrackIds.contains(t.id));

        return Scaffold(
          backgroundColor: Colors.transparent,
          body: Container(
            decoration: BoxDecoration(
              gradient: LinearGradient(
                begin: Alignment.topCenter,
                end: Alignment.bottomCenter,
                colors: [
                  colorScheme.surface.withValues(alpha: 0.94),
                  const Color(0xFF0C1017),
                ],
              ),
            ),
            child: SafeArea(
              child: Stack(
                children: [
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      // 1. Top App Bar Header
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 16.0, vertical: 12.0),
                        child: Row(
                          children: [
                            IconButton(
                              icon: const Icon(Icons.arrow_back_rounded, size: 24),
                              color: colorScheme.onSurface,
                              tooltip: 'Back to queue',
                              onPressed: () => Navigator.of(context).pop(),
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    'ADD TRACKS TO QUEUE',
                                    style: AttuneTypography.soraSectionHeader(color: colorScheme.primary),
                                  ),
                                  const SizedBox(height: 2),
                                  Text(
                                    '${libraryTracks.length} available songs in library',
                                    style: AttuneTypography.jakartaBodySmall(
                                      color: colorScheme.onSurface.withValues(alpha: 0.60),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                            // Quick Select All / Deselect All
                            TextButton.icon(
                              onPressed: () {
                                setState(() {
                                  if (allFilteredSelected) {
                                    for (final t in filteredTracks) {
                                      _selectedTrackIds.remove(t.id);
                                    }
                                  } else {
                                    for (final t in filteredTracks) {
                                      _selectedTrackIds.add(t.id);
                                    }
                                  }
                                });
                              },
                              icon: Icon(
                                allFilteredSelected
                                    ? Icons.deselect_rounded
                                    : Icons.select_all_rounded,
                                size: 18,
                                color: colorScheme.primary,
                              ),
                              label: Text(
                                allFilteredSelected ? 'Deselect' : 'Select All',
                                style: AttuneTypography.soraBadge(color: colorScheme.primary),
                              ),
                            ),
                          ],
                        ),
                      ),

                      // 2. Search Field Bar
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 16.0, vertical: 4.0),
                        child: LiquidGlassContainer(
                          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                          borderRadius: 16,
                          surfaceAlpha: 0.12,
                          child: Row(
                            children: [
                              Icon(Icons.search_rounded, color: colorScheme.primary, size: 20),
                              const SizedBox(width: 10),
                              Expanded(
                                child: TextField(
                                  controller: _searchController,
                                  style: AttuneTypography.jakartaBodyMedium(color: colorScheme.onSurface),
                                  decoration: InputDecoration(
                                    hintText: 'Search title, artist, or album...',
                                    hintStyle: AttuneTypography.jakartaBodyMedium(
                                      color: colorScheme.onSurface.withValues(alpha: 0.40),
                                    ),
                                    border: InputBorder.none,
                                    isDense: true,
                                  ),
                                  onChanged: (val) {
                                    setState(() => _searchQuery = val.trim());
                                  },
                                ),
                              ),
                              if (_searchQuery.isNotEmpty)
                                GestureDetector(
                                  onTap: () {
                                    _searchController.clear();
                                    setState(() => _searchQuery = '');
                                  },
                                  child: Icon(
                                    Icons.close_rounded,
                                    color: colorScheme.onSurface.withValues(alpha: 0.5),
                                    size: 18,
                                  ),
                                ),
                            ],
                          ),
                        ),
                      ),

                      const SizedBox(height: 10),

                      // 3. Category & Genre Horizontal Filter Chips
                      SizedBox(
                        height: 38,
                        child: ListView.separated(
                          scrollDirection: Axis.horizontal,
                          physics: const BouncingScrollPhysics(),
                          padding: const EdgeInsets.symmetric(horizontal: 16.0),
                          itemCount: genreList.length,
                          separatorBuilder: (context, index) => const SizedBox(width: 8),
                          itemBuilder: (context, index) {
                            final genre = genreList[index];
                            final isSelected = _selectedGenre == genre;
                            return GestureDetector(
                              onTap: () => setState(() => _selectedGenre = genre),
                              child: AnimatedContainer(
                                duration: const Duration(milliseconds: 200),
                                padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                                decoration: BoxDecoration(
                                  color: isSelected
                                      ? colorScheme.primary.withValues(alpha: 0.25)
                                      : Colors.white.withValues(alpha: 0.05),
                                  borderRadius: BorderRadius.circular(12),
                                  border: Border.all(
                                    color: isSelected
                                        ? colorScheme.primary.withValues(alpha: 0.6)
                                        : Colors.white.withValues(alpha: 0.08),
                                    width: 1,
                                  ),
                                ),
                                child: Text(
                                  genre,
                                  style: AttuneTypography.soraBadge(
                                    color: isSelected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.6),
                                  ),
                                ),
                              ),
                            );
                          },
                        ),
                      ),

                      const SizedBox(height: 12),

                      // 4. Track List
                      Expanded(
                        child: filteredTracks.isEmpty
                            ? Center(
                                child: Column(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    Icon(Icons.search_off_rounded, size: 48, color: colorScheme.primary.withValues(alpha: 0.4)),
                                    const SizedBox(height: 12),
                                    Text(
                                      'No tracks found matching your filter',
                                      style: AttuneTypography.soraTitleSmall(color: colorScheme.onSurface.withValues(alpha: 0.7)),
                                    ),
                                  ],
                                ),
                              )
                            : ListView.builder(
                                physics: const BouncingScrollPhysics(),
                                padding: const EdgeInsets.only(left: 16.0, right: 16.0, top: 4.0, bottom: 96.0),
                                itemCount: filteredTracks.length,
                                itemBuilder: (context, index) {
                                  final track = filteredTracks[index];
                                  final isSelected = _selectedTrackIds.contains(track.id);
                                  final isAlreadyInQueue = currentQueueIds.contains(track.id);
                                  final durMin = track.duration.inMinutes;
                                  final durSec = (track.duration.inSeconds % 60).toString().padLeft(2, '0');

                                  return Padding(
                                    padding: const EdgeInsets.only(bottom: 8.0),
                                    child: GestureDetector(
                                      onTap: () {
                                        setState(() {
                                          if (isSelected) {
                                            _selectedTrackIds.remove(track.id);
                                          } else {
                                            _selectedTrackIds.add(track.id);
                                          }
                                        });
                                      },
                                      child: LiquidGlassContainer(
                                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                                        borderRadius: 16,
                                        surfaceAlpha: isSelected ? 0.25 : 0.08,
                                        tintOverride: isSelected ? colorScheme.primary.withValues(alpha: 0.22) : null,
                                        child: Row(
                                          children: [
                                            // Selection Check Indicator
                                            AnimatedContainer(
                                              duration: const Duration(milliseconds: 180),
                                              width: 22,
                                              height: 22,
                                              decoration: BoxDecoration(
                                                shape: BoxShape.circle,
                                                color: isSelected ? colorScheme.primary : Colors.white.withValues(alpha: 0.08),
                                                border: Border.all(
                                                  color: isSelected ? colorScheme.primary : Colors.white.withValues(alpha: 0.25),
                                                  width: 1.5,
                                                ),
                                              ),
                                              child: isSelected
                                                  ? const Icon(Icons.check_rounded, color: Colors.black, size: 14)
                                                  : null,
                                            ),
                                            const SizedBox(width: 12),

                                            // Artwork Thumbnail
                                            ClipRRect(
                                              borderRadius: BorderRadius.circular(10),
                                              child: ViewportAwareArtwork(
                                                artworkFile: track.artworkFile,
                                                artworkAsset: track.artworkAsset,
                                                width: 42,
                                                height: 42,
                                                borderRadius: BorderRadius.circular(10),
                                                cacheWidth: 100,
                                                cacheHeight: 100,
                                                placeholder: Container(
                                                  width: 42,
                                                  height: 42,
                                                  color: track.seedColor.withValues(alpha: 0.35),
                                                  child: const Center(
                                                    child: Icon(Icons.music_note_rounded, color: Colors.white, size: 20),
                                                  ),
                                                ),
                                              ),
                                            ),
                                            const SizedBox(width: 12),

                                            // Metadata
                                            Expanded(
                                              child: Column(
                                                crossAxisAlignment: CrossAxisAlignment.start,
                                                children: [
                                                  Row(
                                                    children: [
                                                      Expanded(
                                                        child: Text(
                                                          track.title,
                                                          style: AttuneTypography.soraTitleSmall(
                                                            color: isSelected ? colorScheme.primary : colorScheme.onSurface,
                                                          ),
                                                          maxLines: 1,
                                                          overflow: TextOverflow.ellipsis,
                                                        ),
                                                      ),
                                                      if (isAlreadyInQueue)
                                                        Container(
                                                          margin: const EdgeInsets.only(left: 6),
                                                          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                                                          decoration: BoxDecoration(
                                                            color: Colors.white.withValues(alpha: 0.08),
                                                            borderRadius: BorderRadius.circular(6),
                                                          ),
                                                          child: Text(
                                                            'In Queue',
                                                            style: AttuneTypography.soraBadge(
                                                              fontSize: 9,
                                                              color: colorScheme.onSurface.withValues(alpha: 0.50),
                                                            ),
                                                          ),
                                                        ),
                                                    ],
                                                  ),
                                                  const SizedBox(height: 2),
                                                  Text(
                                                    '${track.artist}${track.hasBpm ? " • ${track.formattedBpm}" : ""}${track.musicalKey != "Pending Profile" && track.musicalKey.isNotEmpty ? " • ${track.musicalKey}" : ""}',
                                                    style: AttuneTypography.jakartaBodySmall(
                                                      color: colorScheme.onSurface.withValues(alpha: 0.60),
                                                    ),
                                                    maxLines: 1,
                                                    overflow: TextOverflow.ellipsis,
                                                  ),
                                                ],
                                              ),
                                            ),
                                            const SizedBox(width: 8),

                                            // Duration
                                            Text(
                                              '$durMin:$durSec',
                                              style: AttuneTypography.soraMetric(
                                                fontSize: 12,
                                                color: colorScheme.onSurface.withValues(alpha: 0.45),
                                              ),
                                            ),
                                          ],
                                        ),
                                      ),
                                    ),
                                  );
                                },
                              ),
                      ),
                    ],
                  ),

                  // 5. Floating Bottom Action Button Bar
                  Positioned(
                    left: 20,
                    right: 20,
                    bottom: 16,
                    child: LiquidGlassContainer(
                      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                      borderRadius: 24,
                      surfaceAlpha: 0.88,
                      blurSigma: 24,
                      child: Row(
                        children: [
                          Expanded(
                            child: Text(
                              _selectedTrackIds.isEmpty
                                  ? 'Select tracks above to add'
                                  : '${_selectedTrackIds.length} track${_selectedTrackIds.length == 1 ? "" : "s"} selected',
                              style: AttuneTypography.jakartaBodyMedium(
                                color: _selectedTrackIds.isEmpty
                                    ? colorScheme.onSurface.withValues(alpha: 0.55)
                                    : colorScheme.primary,
                                weight: FontWeight.w600,
                              ),
                            ),
                          ),
                          FilledButton.icon(
                            style: FilledButton.styleFrom(
                              backgroundColor: colorScheme.primary,
                              foregroundColor: colorScheme.onPrimary,
                              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                              padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 12),
                            ),
                            icon: const Icon(Icons.playlist_add_rounded, size: 20),
                            label: Text(
                              _selectedTrackIds.isEmpty ? 'Add All (${filteredTracks.length})' : 'Add to Queue',
                              style: AttuneTypography.soraBadge(
                                color: colorScheme.onPrimary,
                                fontSize: 13,
                              ),
                            ),
                            onPressed: () {
                              final tracksToAdd = _selectedTrackIds.isEmpty
                                  ? filteredTracks
                                  : libraryTracks.where((t) => _selectedTrackIds.contains(t.id)).toList();

                              if (tracksToAdd.isEmpty) return;
                              controller.addTracksToQueue(tracksToAdd);
                              Navigator.of(context).pop();

                              ScaffoldMessenger.of(context).showSnackBar(
                                SnackBar(
                                  content: Text('Added ${tracksToAdd.length} tracks to queue'),
                                  duration: const Duration(seconds: 2),
                                  behavior: SnackBarBehavior.floating,
                                  backgroundColor: const Color(0xFF161F2E),
                                ),
                              );
                            },
                          ),
                        ],
                      ),
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
}
