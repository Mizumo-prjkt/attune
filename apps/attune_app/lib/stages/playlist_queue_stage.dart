import 'package:flutter/material.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/transition_connector_pill.dart';

/// PlaylistQueueStage implementing Stage 2 of 05_UI_DESIGN_SPEC.md:
/// Now Playing mini-header, subtle inline transition connector pill,
/// filter chips, Smart Shuffle / Autoplay toggles, and queued track list.
class PlaylistQueueStage extends StatefulWidget {
  const PlaylistQueueStage({super.key});

  @override
  State<PlaylistQueueStage> createState() => _PlaylistQueueStageState();
}

class _PlaylistQueueStageState extends State<PlaylistQueueStage> {
  int _selectedFilterIndex = 0;
  bool _smartShuffle = true;
  bool _autoplay = true;

  final List<Map<String, String>> _queueItems = [
    {
      'title': '(Not) A Devil',
      'artist': 'DECO*27, PinocchioP',
      'duration': '02:44',
      'bpm': '149.3',
    },
    {
      'title': 'Liar Dancer',
      'artist': 'Masarada feat. Kasane Teto',
      'duration': '04:02',
      'bpm': '171.4',
    },
    {
      'title': 'Tokyo Teddy Bear',
      'artist': 'Neru feat. Kagamine Rin',
      'duration': '03:12',
      'bpm': '132.8',
    },
    {
      'title': 'Lost One\'s Weeping',
      'artist': 'Neru feat. Kagamine Rin',
      'duration': '03:36',
      'bpm': '159.6',
    },
    {
      'title': 'How-to World Domination',
      'artist': 'Neru feat. Kagamine Rin, Len',
      'duration': '03:18',
      'bpm': '150.1',
    },
    {
      'title': 'Konton Boogie',
      'artist': 'jon-YAKITORY',
      'duration': '02:51',
      'bpm': '181.6',
    },
  ];

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

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
                  decoration: BoxDecoration(
                    gradient: LinearGradient(
                      colors: [colorScheme.primary, colorScheme.secondary],
                    ),
                  ),
                  child: const Icon(Icons.equalizer_rounded, color: Colors.white, size: 24),
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
                      'Rabbit Hole',
                      style: AttuneTypography.soraTitleSmall(color: colorScheme.onSurface),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    Text(
                      'DECO*27 feat. Hatsune Miku',
                      style: AttuneTypography.jakartaBodySmall(
                        color: colorScheme.onSurface.withValues(alpha: 0.65),
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                ),
              ),
              Icon(Icons.graphic_eq_rounded, color: colorScheme.primary, size: 22),
            ],
          ),
        ),

        const SizedBox(height: 12),

        // 2. Subtle Inline Transition Connector Pill (Section 3.2)
        Center(
          child: TransitionConnectorPill(
            archetype: 'Riff Handoff',
            durationSeconds: 1.4,
            confidenceScore: 0.91,
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

        // 4. Quick Toggles (Smart Shuffle & Autoplay)
        Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 8,
          runSpacing: 8,
          children: [
            _buildActionChip(
              icon: Icons.auto_awesome_rounded,
              label: 'Smart Shuffle',
              active: _smartShuffle,
              colorScheme: colorScheme,
              onTap: () => setState(() => _smartShuffle = !_smartShuffle),
            ),
            _buildActionChip(
              icon: Icons.all_inclusive_rounded,
              label: 'Autoplay',
              active: _autoplay,
              colorScheme: colorScheme,
              onTap: () => setState(() => _autoplay = !_autoplay),
            ),
            Text(
              '${_queueItems.length} songs',
              style: AttuneTypography.jakartaBodySmall(
                color: colorScheme.onSurface.withValues(alpha: 0.5),
              ),
            ),
          ],
        ),

        const SizedBox(height: 12),

        // 5. Queued Streams List
        ...List.generate(_queueItems.length, (index) {
          final item = _queueItems[index];
          return Padding(
            padding: const EdgeInsets.only(bottom: 8.0),
            child: LiquidGlassContainer(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 10),
              borderRadius: 16,
              surfaceAlpha: 0.10,
              child: Row(
                children: [
                  Text(
                    '${index + 1}',
                    style: AttuneTypography.soraMetric(
                      fontSize: 12,
                      weight: FontWeight.w600,
                      color: colorScheme.onSurface.withValues(alpha: 0.4),
                    ),
                  ),
                  const SizedBox(width: 8),
                  ClipRRect(
                    borderRadius: BorderRadius.circular(10),
                    child: Container(
                      width: 38,
                      height: 38,
                      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.4),
                      child: Center(
                        child: Icon(
                          Icons.music_note_rounded,
                          color: colorScheme.onSurface.withValues(alpha: 0.7),
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
                          item['title']!,
                          style: AttuneTypography.soraTitleSmall(color: colorScheme.onSurface),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '${item['artist']} • ${item['bpm']} BPM',
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
                    item['duration']!,
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
  }

  Widget _buildFilterChip(String label, int index, ColorScheme colorScheme) {
    final selected = _selectedFilterIndex == index;
    return LiquidGlassContainer(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
      borderRadius: 14,
      surfaceAlpha: selected ? 0.28 : 0.10,
      tintOverride: selected ? colorScheme.primary.withValues(alpha: 0.22) : null,
      onTap: () => setState(() => _selectedFilterIndex = index),
      child: Text(
        label,
        style: AttuneTypography.soraBadge(
          color: selected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.7),
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
    return LiquidGlassContainer(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      borderRadius: 12,
      surfaceAlpha: active ? 0.22 : 0.08,
      tintOverride: active ? colorScheme.secondary.withValues(alpha: 0.18) : null,
      onTap: onTap,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: active ? colorScheme.secondary : colorScheme.onSurface.withValues(alpha: 0.5)),
          const SizedBox(width: 5),
          Text(
            label,
            style: AttuneTypography.soraBadge(
              color: active ? colorScheme.secondary : colorScheme.onSurface.withValues(alpha: 0.5),
            ),
          ),
        ],
      ),
    );
  }
}
