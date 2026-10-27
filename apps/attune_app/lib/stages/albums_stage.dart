import 'dart:io';
import 'package:flutter/material.dart';
import '../models/track_item.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// AlbumsStage: Replaces lyrics in main navigation with a dedicated Album library.
/// Displays an optical glass grid of all discovered albums with cover art,
/// artist names, and interactive album track inspection.
class AlbumsStage extends StatefulWidget {
  final VoidCallback? onOpenNowPlaying;

  const AlbumsStage({super.key, this.onOpenNowPlaying});

  @override
  State<AlbumsStage> createState() => _AlbumsStageState();
}

class _AlbumsStageState extends State<AlbumsStage> {
  String? _selectedAlbum;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final allTracks = controller.libraryTracks;

        // Group tracks by album name
        final albumMap = <String, List<TrackItem>>{};
        for (final track in allTracks) {
          final albumName = track.album.isNotEmpty ? track.album : 'Singles & Loose Tracks';
          albumMap.putIfAbsent(albumName, () => []).add(track);
        }

        final albumNames = albumMap.keys.toList()
          ..sort((a, b) => a.compareTo(b));

        if (_selectedAlbum != null && albumMap.containsKey(_selectedAlbum)) {
          return _buildAlbumDetailView(
            albumName: _selectedAlbum!,
            tracks: albumMap[_selectedAlbum!]!,
            colorScheme: colorScheme,
            controller: controller,
          );
        }

        return ListView(
          physics: const BouncingScrollPhysics(),
          padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 16.0),
          children: [
            // Header
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  'ALBUMS & COLLECTIONS',
                  style: AttuneTypography.soraBadge(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                Text(
                  '${albumNames.length} albums',
                  style: AttuneTypography.soraBadge(
                    color: colorScheme.onSurface.withValues(alpha: 0.4),
                  ),
                ),
              ],
            ),

            const SizedBox(height: 16),

            // Grid of Albums
            GridView.builder(
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
                maxCrossAxisExtent: 220,
                mainAxisSpacing: 16,
                crossAxisSpacing: 16,
                childAspectRatio: 0.76,
              ),
              itemCount: albumNames.length,
              itemBuilder: (context, index) {
                final name = albumNames[index];
                final tracks = albumMap[name]!;
                final firstWithArtwork = tracks.firstWhere(
                  (t) => t.artworkFile != null && File(t.artworkFile!).existsSync(),
                  orElse: () => tracks.first,
                );

                return _buildAlbumCard(
                  name: name,
                  tracks: tracks,
                  leadTrack: firstWithArtwork,
                  colorScheme: colorScheme,
                  onTap: () => setState(() => _selectedAlbum = name),
                );
              },
            ),

            const SizedBox(height: 90),
          ],
        );
      },
    );
  }

  Widget _buildAlbumCard({
    required String name,
    required List<TrackItem> tracks,
    required TrackItem leadTrack,
    required ColorScheme colorScheme,
    required VoidCallback onTap,
  }) {
    return GestureDetector(
      onTap: onTap,
      child: LiquidGlassContainer(
        padding: const EdgeInsets.all(12),
        borderRadius: 22,
        surfaceAlpha: 0.12,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Album Artwork
            Expanded(
              child: Container(
                width: double.infinity,
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(16),
                  gradient: LinearGradient(
                    colors: [
                      leadTrack.seedColor.withValues(alpha: 0.7),
                      leadTrack.seedColor.withValues(alpha: 0.2),
                    ],
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                  ),
                  boxShadow: [
                    BoxShadow(
                      color: leadTrack.seedColor.withValues(alpha: 0.25),
                      blurRadius: 10,
                      offset: const Offset(0, 4),
                    ),
                  ],
                ),
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(16),
                  child: leadTrack.artworkFile != null && File(leadTrack.artworkFile!).existsSync()
                      ? Image.file(
                          File(leadTrack.artworkFile!),
                          fit: BoxFit.cover,
                        )
                      : Center(
                          child: Icon(
                            Icons.album_rounded,
                            size: 40,
                            color: Colors.white.withValues(alpha: 0.8),
                          ),
                        ),
                ),
              ),
            ),

            const SizedBox(height: 10),

            // Title
            Text(
              name,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: AttuneTypography.soraSubheading(
                fontSize: 14,
                fontWeight: FontWeight.w600,
                color: colorScheme.onSurface,
              ),
            ),

            const SizedBox(height: 2),

            // Artist and Track Count
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Text(
                    leadTrack.artist,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: AttuneTypography.jakartaBody(
                      fontSize: 12,
                      color: colorScheme.onSurface.withValues(alpha: 0.55),
                    ),
                  ),
                ),
                Text(
                  '${tracks.length} tracks',
                  style: AttuneTypography.soraBadge(
                    fontSize: 10,
                    color: colorScheme.primary,
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildAlbumDetailView({
    required String albumName,
    required List<TrackItem> tracks,
    required ColorScheme colorScheme,
    required PlaybackController controller,
  }) {
    final firstTrack = tracks.first;

    return ListView(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 16.0),
      children: [
        // Back Button
        Row(
          children: [
            GestureDetector(
              onTap: () => setState(() => _selectedAlbum = null),
              child: LiquidGlassContainer(
                padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                borderRadius: 16,
                surfaceAlpha: 0.16,
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.arrow_back_rounded, size: 18),
                    const SizedBox(width: 6),
                    Text(
                      'All Albums',
                      style: AttuneTypography.soraBadge(color: colorScheme.onSurface),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),

        const SizedBox(height: 18),

        // Album Hero Banner
        LiquidGlassContainer(
          padding: const EdgeInsets.all(18),
          borderRadius: 24,
          surfaceAlpha: 0.16,
          child: Row(
            children: [
              Container(
                width: 88,
                height: 88,
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(16),
                  boxShadow: [
                    BoxShadow(
                      color: firstTrack.seedColor.withValues(alpha: 0.35),
                      blurRadius: 14,
                    ),
                  ],
                ),
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(16),
                  child: firstTrack.artworkFile != null && File(firstTrack.artworkFile!).existsSync()
                      ? Image.file(File(firstTrack.artworkFile!), fit: BoxFit.cover)
                      : Container(
                          color: firstTrack.seedColor.withValues(alpha: 0.3),
                          child: Icon(Icons.album_rounded, size: 40, color: firstTrack.seedColor),
                        ),
                ),
              ),

              const SizedBox(width: 16),

              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      albumName,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: AttuneTypography.soraHeading(
                        fontSize: 18,
                        fontWeight: FontWeight.w700,
                        color: colorScheme.onSurface,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '${firstTrack.artist} • ${tracks.length} songs',
                      style: AttuneTypography.jakartaBody(
                        color: colorScheme.onSurface.withValues(alpha: 0.65),
                      ),
                    ),
                    const SizedBox(height: 12),
                    ElevatedButton.icon(
                      style: ElevatedButton.styleFrom(
                        backgroundColor: colorScheme.primary,
                        foregroundColor: colorScheme.onPrimary,
                        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
                      ),
                      onPressed: () {
                        controller.playTrack(firstTrack);
                        widget.onOpenNowPlaying?.call();
                      },
                      icon: const Icon(Icons.play_arrow_rounded, size: 20),
                      label: const Text('Play Album'),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),

        const SizedBox(height: 24),

        // Track List
        Text(
          'TRACKS IN THIS ALBUM',
          style: AttuneTypography.soraBadge(color: colorScheme.primary),
        ),

        const SizedBox(height: 12),

        ...tracks.asMap().entries.map((entry) {
          final index = entry.key + 1;
          final track = entry.value;
          final isCurrent = controller.currentTrack?.id == track.id;

          return Padding(
            padding: const EdgeInsets.only(bottom: 8.0),
            child: GestureDetector(
              onTap: () {
                controller.playTrack(track);
                widget.onOpenNowPlaying?.call();
              },
              child: LiquidGlassContainer(
                padding: const EdgeInsets.all(12),
                borderRadius: 16,
                surfaceAlpha: isCurrent ? 0.28 : 0.08,
                borderColor: isCurrent ? colorScheme.primary.withValues(alpha: 0.5) : null,
                child: Row(
                  children: [
                    Text(
                      '$index',
                      style: AttuneTypography.soraBadge(
                        color: isCurrent ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.4),
                      ),
                    ),
                    const SizedBox(width: 14),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            track.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: AttuneTypography.soraSubheading(
                              fontSize: 14,
                              fontWeight: FontWeight.w600,
                              color: isCurrent ? colorScheme.primary : colorScheme.onSurface,
                            ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            '${track.format} • ${track.bpm.toStringAsFixed(1)} BPM',
                            style: AttuneTypography.jakartaBody(
                              fontSize: 11,
                              color: colorScheme.onSurface.withValues(alpha: 0.5),
                            ),
                          ),
                        ],
                      ),
                    ),
                    Icon(
                      isCurrent && controller.isPlaying ? Icons.pause_circle_filled_rounded : Icons.play_circle_fill_rounded,
                      color: isCurrent ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.4),
                      size: 22,
                    ),
                  ],
                ),
              ),
            ),
          );
        }),

        const SizedBox(height: 90),
      ],
    );
  }
}
