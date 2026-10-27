import 'package:flutter/material.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// MusicLibraryStage implementing Stage 4 of 05_UI_DESIGN_SPEC.md:
/// Search bar, filter pills, Liked Songs / Heavy Rotation / Local Audio cards,
/// and Recently Played carousel.
class MusicLibraryStage extends StatefulWidget {
  const MusicLibraryStage({super.key});

  @override
  State<MusicLibraryStage> createState() => _MusicLibraryStageState();
}

class _MusicLibraryStageState extends State<MusicLibraryStage> {
  int _selectedFilter = 0;

  final List<String> _filters = ['All Music', 'Playlists', 'Albums', 'Artists', 'Local Audio'];

  final List<Map<String, String>> _recentTracks = [
    {'title': 'Rabbit Hole', 'artist': 'DECO*27', 'format': 'FLAC • 44.1k'},
    {'title': '(Not) A Devil', 'artist': 'DECO*27, PinocchioP', 'format': 'FLAC • 44.1k'},
    {'title': 'Liar Dancer', 'artist': 'Masarada', 'format': 'FLAC • 44.1k'},
    {'title': 'Tokyo Teddy Bear', 'artist': 'Neru', 'format': 'MP3 • 320k'},
    {'title': 'Lost One\'s Weeping', 'artist': 'Neru', 'format': 'MP3 • 320k'},
  ];

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListView(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 16.0),
      children: [
        // 1. Search Bar
        LiquidGlassContainer(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          borderRadius: 20,
          surfaceAlpha: 0.12,
          child: Row(
            children: [
              Icon(Icons.search_rounded, color: colorScheme.onSurface.withValues(alpha: 0.6), size: 22),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  'Search songs, artists, albums, or vibes...',
                  style: AttuneTypography.jakartaBodyMedium(
                    color: colorScheme.onSurface.withValues(alpha: 0.5),
                  ),
                ),
              ),
              Icon(Icons.mic_none_rounded, color: colorScheme.onSurface.withValues(alpha: 0.6), size: 20),
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
                      '124 tracks',
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

        // 4. Recently Played Section Header
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4.0),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                'RECENTLY PLAYED',
                style: AttuneTypography.soraSectionHeader(color: colorScheme.primary),
              ),
              Text(
                'See all',
                style: AttuneTypography.soraBadge(color: colorScheme.onSurface.withValues(alpha: 0.5)),
              ),
            ],
          ),
        ),

        const SizedBox(height: 12),

        // 5. Track List
        ...List.generate(_recentTracks.length, (index) {
          final t = _recentTracks[index];
          return Padding(
            padding: const EdgeInsets.only(bottom: 8.0),
            child: LiquidGlassContainer(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
              borderRadius: 16,
              surfaceAlpha: 0.10,
              child: Row(
                children: [
                  ClipRRect(
                    borderRadius: BorderRadius.circular(10),
                    child: Container(
                      width: 44,
                      height: 44,
                      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
                      child: Center(
                        child: Icon(Icons.music_note_rounded, color: colorScheme.primary, size: 22),
                      ),
                    ),
                  ),
                  const SizedBox(width: 14),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          t['title']!,
                          style: AttuneTypography.soraTitleSmall(color: colorScheme.onSurface),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '${t['artist']} • ${t['format']}',
                          style: AttuneTypography.jakartaBodySmall(
                            color: colorScheme.onSurface.withValues(alpha: 0.6),
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.more_vert_rounded),
                    color: colorScheme.onSurface.withValues(alpha: 0.5),
                    onPressed: () {},
                  ),
                ],
              ),
            ),
          );
        }),
      ],
    );
  }
}
