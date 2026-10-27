import 'dart:io';
import 'package:flutter/material.dart';
import '../models/track_item.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../services/settings_service.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

enum _ExclusionFilterTab {
  all,
  excluded,
  available;

  String get label {
    switch (this) {
      case _ExclusionFilterTab.all:
        return 'All Songs';
      case _ExclusionFilterTab.excluded:
        return 'Excluded';
      case _ExclusionFilterTab.available:
        return 'Available';
    }
  }
}

/// Dedicated full-screen UI for managing Song Exclusions with instant real-time search,
/// category filtering, album art previews, and batch restore capabilities.
class SongExclusionStage extends StatefulWidget {
  const SongExclusionStage({super.key});

  @override
  State<SongExclusionStage> createState() => _SongExclusionStageState();
}

class _SongExclusionStageState extends State<SongExclusionStage> {
  final TextEditingController _searchController = TextEditingController();
  final SettingsService _settings = SettingsService.instance;
  final PlaybackController _playback = PlaybackController.instance;

  String _searchQuery = '';
  _ExclusionFilterTab _currentTab = _ExclusionFilterTab.all;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  String _formatDuration(Duration d) {
    final m = d.inMinutes;
    final s = d.inSeconds % 60;
    return '$m:${s.toString().padLeft(2, '0')}';
  }

  Widget _buildArtworkThumbnail(TrackItem track, ColorScheme colorScheme, {bool isExcluded = false}) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(10),
      child: Container(
        width: 48,
        height: 48,
        color: isExcluded
            ? Colors.red.withValues(alpha: 0.15)
            : colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
        child: Stack(
          alignment: Alignment.center,
          children: [
            if (track.artworkFile != null && File(track.artworkFile!).existsSync())
              Image.file(
                File(track.artworkFile!),
                width: 48,
                height: 48,
                fit: BoxFit.cover,
                errorBuilder: (context, error, stackTrace) => Icon(
                  isExcluded ? Icons.music_off_rounded : Icons.music_note_rounded,
                  color: isExcluded ? Colors.redAccent : Colors.white54,
                  size: 22,
                ),
              )
            else
              Icon(
                isExcluded ? Icons.music_off_rounded : Icons.music_note_rounded,
                color: isExcluded ? Colors.redAccent : Colors.white54,
                size: 22,
              ),
            if (isExcluded)
              Container(
                width: 48,
                height: 48,
                color: Colors.black.withValues(alpha: 0.45),
                child: const Icon(
                  Icons.block_rounded,
                  color: Colors.redAccent,
                  size: 20,
                ),
              ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: AmbientMeshBackground(
        colorScheme: colorScheme,
        child: SafeArea(
          child: Column(
            children: [
              // 1. App Bar Header
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16.0, vertical: 12.0),
                child: Row(
                  children: [
                    LiquidGlassContainer(
                      padding: const EdgeInsets.all(8),
                      borderRadius: 14,
                      surfaceAlpha: 0.18,
                      onTap: () => PlatformNavigation.closeCurrent(context),
                      child: const Icon(Icons.arrow_back_rounded, size: 20, color: Colors.white),
                    ),
                    const SizedBox(width: 14),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Song Exclusion Manager',
                            style: AttuneTypography.soraTitleMedium(color: Colors.white),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            'Filter tracks out of queues, shuffle, and library',
                            style: AttuneTypography.jakartaBodySmall(color: Colors.white60),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),

              // 2. Liquid Glass Search Bar
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 6.0),
                child: LiquidGlassContainer(
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
                  borderRadius: 18,
                  surfaceAlpha: 0.14,
                  child: Row(
                    children: [
                      Icon(
                        Icons.search_rounded,
                        color: Colors.white.withValues(alpha: 0.6),
                        size: 20,
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: _searchController,
                          style: AttuneTypography.jakartaBodyMedium(color: Colors.white),
                          decoration: InputDecoration(
                            hintText: 'Search by song title, artist, or album...',
                            hintStyle: AttuneTypography.jakartaBodyMedium(
                              color: Colors.white.withValues(alpha: 0.4),
                            ),
                            border: InputBorder.none,
                            isDense: true,
                          ),
                          onChanged: (val) {
                            setState(() {
                              _searchQuery = val.trim();
                            });
                          },
                        ),
                      ),
                      if (_searchQuery.isNotEmpty)
                        IconButton(
                          icon: const Icon(Icons.clear_rounded, size: 18, color: Colors.white70),
                          splashRadius: 18,
                          onPressed: () {
                            _searchController.clear();
                            setState(() {
                              _searchQuery = '';
                            });
                          },
                        ),
                    ],
                  ),
                ),
              ),

              // 3. Tab Filter Row (All, Excluded, Available)
              ListenableBuilder(
                listenable: Listenable.merge([_settings, _playback]),
                builder: (context, _) {
                  final allTracks = _playback.rawScannedTracks.isNotEmpty
                      ? _playback.rawScannedTracks
                      : _playback.libraryTracks;
                  final excludedIds = _settings.excludedSongIds;
                  final excludedCount = allTracks
                      .where((t) => excludedIds.contains(t.id) || excludedIds.contains(t.filePath))
                      .length;
                  final availableCount = allTracks.length - excludedCount;

                  return Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 8.0),
                    child: Row(
                      children: _ExclusionFilterTab.values.map((tab) {
                        final isSelected = _currentTab == tab;
                        String countText;
                        switch (tab) {
                          case _ExclusionFilterTab.all:
                            countText = '${allTracks.length}';
                            break;
                          case _ExclusionFilterTab.excluded:
                            countText = '$excludedCount';
                            break;
                          case _ExclusionFilterTab.available:
                            countText = '$availableCount';
                            break;
                        }

                        return Padding(
                          padding: const EdgeInsets.only(right: 8.0),
                          child: InkWell(
                            borderRadius: BorderRadius.circular(12),
                            onTap: () {
                              setState(() {
                                _currentTab = tab;
                              });
                            },
                            child: AnimatedContainer(
                              duration: const Duration(milliseconds: 180),
                              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                              decoration: BoxDecoration(
                                color: isSelected
                                    ? (tab == _ExclusionFilterTab.excluded
                                        ? Colors.redAccent.withValues(alpha: 0.28)
                                        : colorScheme.primary.withValues(alpha: 0.28))
                                    : Colors.white.withValues(alpha: 0.06),
                                borderRadius: BorderRadius.circular(12),
                                border: Border.all(
                                  color: isSelected
                                      ? (tab == _ExclusionFilterTab.excluded
                                          ? Colors.redAccent.withValues(alpha: 0.6)
                                          : colorScheme.primary.withValues(alpha: 0.6))
                                      : Colors.white10,
                                ),
                              ),
                              child: Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Text(
                                    tab.label,
                                    style: AttuneTypography.soraTitleSmall(
                                      fontSize: 12,
                                      color: isSelected ? Colors.white : Colors.white70,
                                    ),
                                  ),
                                  const SizedBox(width: 6),
                                  Text(
                                    countText,
                                    style: AttuneTypography.jakartaBodySmall(
                                      fontSize: 11,
                                      color: isSelected
                                          ? (tab == _ExclusionFilterTab.excluded
                                              ? Colors.redAccent
                                              : colorScheme.primary)
                                          : Colors.white38,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        );
                      }).toList(),
                    ),
                  );
                },
              ),

              const SizedBox(height: 4),

              // 4. Track List Body
              Expanded(
                child: ListenableBuilder(
                  listenable: Listenable.merge([_settings, _playback]),
                  builder: (context, _) {
                    final allTracks = _playback.rawScannedTracks.isNotEmpty
                        ? _playback.rawScannedTracks
                        : _playback.libraryTracks;
                    final excludedIds = _settings.excludedSongIds;

                    // Filter by search query
                    var searchFiltered = allTracks.where((t) {
                      if (_searchQuery.isEmpty) return true;
                      final q = _searchQuery.toLowerCase();
                      return t.title.toLowerCase().contains(q) ||
                          t.artist.toLowerCase().contains(q) ||
                          t.album.toLowerCase().contains(q);
                    }).toList();

                    // Filter by active tab
                    List<TrackItem> displayedTracks;
                    switch (_currentTab) {
                      case _ExclusionFilterTab.all:
                        displayedTracks = searchFiltered;
                        break;
                      case _ExclusionFilterTab.excluded:
                        displayedTracks = searchFiltered
                            .where((t) => excludedIds.contains(t.id) || excludedIds.contains(t.filePath))
                            .toList();
                        break;
                      case _ExclusionFilterTab.available:
                        displayedTracks = searchFiltered
                            .where((t) => !excludedIds.contains(t.id) && !excludedIds.contains(t.filePath))
                            .toList();
                        break;
                    }

                    if (displayedTracks.isEmpty) {
                      return Center(
                        child: Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 32.0),
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              Icon(
                                _searchQuery.isNotEmpty
                                    ? Icons.search_off_rounded
                                    : (_currentTab == _ExclusionFilterTab.excluded
                                        ? Icons.check_circle_outline_rounded
                                        : Icons.music_note_rounded),
                                size: 48,
                                color: Colors.white38,
                              ),
                              const SizedBox(height: 16),
                              Text(
                                _searchQuery.isNotEmpty
                                    ? 'No tracks matching "$_searchQuery"'
                                    : (_currentTab == _ExclusionFilterTab.excluded
                                        ? 'No tracks currently excluded'
                                        : 'No available tracks found'),
                                style: AttuneTypography.soraTitleSmall(color: Colors.white70),
                                textAlign: TextAlign.center,
                              ),
                              const SizedBox(height: 6),
                              Text(
                                _searchQuery.isNotEmpty
                                    ? 'Try refining your search keyword or clearing the search query.'
                                    : 'Tracks excluded from playback will appear here.',
                                style: AttuneTypography.jakartaBodySmall(color: Colors.white38),
                                textAlign: TextAlign.center,
                              ),
                              if (_searchQuery.isNotEmpty) ...[
                                const SizedBox(height: 16),
                                LiquidGlassContainer(
                                  borderRadius: 12,
                                  surfaceAlpha: 0.16,
                                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                                  onTap: () {
                                    _searchController.clear();
                                    setState(() {
                                      _searchQuery = '';
                                    });
                                  },
                                  child: Text(
                                    'Clear Search',
                                    style: AttuneTypography.soraTitleSmall(
                                      fontSize: 12,
                                      color: colorScheme.primary,
                                    ),
                                  ),
                                ),
                              ],
                            ],
                          ),
                        ),
                      );
                    }

                    return ListView.builder(
                      physics: const BouncingScrollPhysics(),
                      padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 8.0),
                      itemCount: displayedTracks.length,
                      itemBuilder: (context, index) {
                        final track = displayedTracks[index];
                        final isExcluded = excludedIds.contains(track.id) || excludedIds.contains(track.filePath);

                        return Container(
                          margin: const EdgeInsets.only(bottom: 8),
                          child: LiquidGlassContainer(
                            borderRadius: 16,
                            surfaceAlpha: isExcluded ? 0.20 : 0.10,
                            tintOverride: isExcluded ? Colors.redAccent.withValues(alpha: 0.16) : null,
                            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                            child: Row(
                              children: [
                                _buildArtworkThumbnail(track, colorScheme, isExcluded: isExcluded),
                                const SizedBox(width: 14),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      Text(
                                        track.title,
                                        style: AttuneTypography.soraTitleSmall(
                                          color: isExcluded ? Colors.white70 : Colors.white,
                                          fontSize: 13,
                                        ),
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                      ),
                                      const SizedBox(height: 2),
                                      Row(
                                        children: [
                                          Expanded(
                                            child: Text(
                                              '${track.artist} • ${track.album}',
                                              style: AttuneTypography.jakartaBodySmall(
                                                color: Colors.white54,
                                                fontSize: 11,
                                              ),
                                              maxLines: 1,
                                              overflow: TextOverflow.ellipsis,
                                            ),
                                          ),
                                          Text(
                                            _formatDuration(track.duration),
                                            style: AttuneTypography.jakartaBodySmall(
                                              color: Colors.white38,
                                              fontSize: 10,
                                            ),
                                          ),
                                        ],
                                      ),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 12),

                                // Action Button: Exclude / Restore
                                InkWell(
                                  borderRadius: BorderRadius.circular(10),
                                  onTap: () {
                                    if (isExcluded) {
                                      _settings.removeExcludedSong(track.id);
                                      _settings.removeExcludedSong(track.filePath);
                                    } else {
                                      _settings.toggleSongExcluded(track.id);
                                    }
                                    _playback.applyLibraryFilters();
                                  },
                                  child: Container(
                                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                                    decoration: BoxDecoration(
                                      color: isExcluded
                                          ? Colors.redAccent.withValues(alpha: 0.20)
                                          : Colors.white.withValues(alpha: 0.08),
                                      borderRadius: BorderRadius.circular(10),
                                      border: Border.all(
                                        color: isExcluded
                                            ? Colors.redAccent.withValues(alpha: 0.5)
                                            : Colors.white12,
                                      ),
                                    ),
                                    child: Row(
                                      mainAxisSize: MainAxisSize.min,
                                      children: [
                                        Icon(
                                          isExcluded ? Icons.restore_rounded : Icons.block_rounded,
                                          size: 14,
                                          color: isExcluded ? Colors.redAccent : Colors.white70,
                                        ),
                                        const SizedBox(width: 6),
                                        Text(
                                          isExcluded ? 'Restore' : 'Exclude',
                                          style: AttuneTypography.soraTitleSmall(
                                            fontSize: 11,
                                            color: isExcluded ? Colors.redAccent : Colors.white70,
                                          ),
                                        ),
                                      ],
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ),
                        );
                      },
                    );
                  },
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
