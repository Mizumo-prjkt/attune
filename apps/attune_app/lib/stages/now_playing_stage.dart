import 'dart:io';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import '../models/track_item.dart';
import '../services/lyrics_service.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../services/settings_service.dart';
import '../theme/color_engine.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/transition_connector_pill.dart';
import '../widgets/transition_details_dialog.dart';
import 'equalizer_stage.dart';
import 'now_playing_skin_picker_stage.dart';
import 'synchronized_lyrics_stage.dart';

/// Full-height Now Playing Stage implementing Section 3 of UI Design Spec:
/// - Floating Hero Artwork with 32px curvature & dynamic shadow
/// - Integrated Human-Friendly Audio Quality badge (Section 3.1)
/// - Inline Subtle Transition Connector Pill (Section 3.2)
/// - Living Beat-Synchronized Liquid Glass Refraction
/// - High-Legibility Full Screen Synchronized Lyrics with Real Source Resolution
class NowPlayingStage extends StatefulWidget {
  final VoidCallback? onDismiss;
  final VoidCallback? onPrevious;
  final VoidCallback? onNext;
  final VoidCallback? onTogglePlay;

  const NowPlayingStage({
    super.key,
    this.onDismiss,
    this.onPrevious,
    this.onNext,
    this.onTogglePlay,
  });

  @override
  State<NowPlayingStage> createState() => _NowPlayingStageState();
}

class _NowPlayingStageState extends State<NowPlayingStage> with TickerProviderStateMixin {
  bool _showLyrics = false;
  AnimationController? _morphController;
  CurvedAnimation? _morphCurvedAnim;
  int _currentShapeIndex = 0;
  int _previousShapeIndex = 0;
  String? _lastTrackId;
  String? _lastArtworkFile;
  String? _previousArtworkFile;

  @override
  void initState() {
    super.initState();
    _morphController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 750),
    );
    _morphCurvedAnim = CurvedAnimation(
      parent: _morphController!,
      curve: Curves.easeInOutCubic,
    );
    _morphController!.value = 1.0;
  }

  @override
  void dispose() {
    _morphController?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final isTabletDesktop = MediaQuery.sizeOf(context).width >= 720;

    return ListenableBuilder(
      listenable: Listenable.merge([PlaybackController.instance, SettingsService.instance]),
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final settings = SettingsService.instance;
        final track = controller.currentTrack;
        final nextTrack = controller.upNextTrack;
        final playing = controller.isPlaying;

        final posSec = controller.positionSec;
        final durSec = controller.durationSec > 0 ? controller.durationSec : 159.0;

        final currentMin = (posSec / 60).floor();
        final currentSec = (posSec % 60).floor();
        final totalMin = (durSec / 60).floor();
        final totalSec = (durSec % 60).floor();

        // Calculate dynamic ColorScheme based on track seedColor and transition morphing
        final baseScheme =
            AttuneColorEngine.createSchemeFromSeed(track?.seedColor ?? AttuneColorEngine.fallbackPrimary);
        final targetScheme =
            AttuneColorEngine.createSchemeFromSeed(nextTrack?.seedColor ?? AttuneColorEngine.fallbackSecondary);
        final colorScheme = controller.isTransitioning
            ? AttuneColorEngine.lerpSchemes(baseScheme, targetScheme, controller.transitionProgress)
            : baseScheme;

        // Update morphing shape when track changes
        if (track?.id != _lastTrackId) {
          _previousShapeIndex = _currentShapeIndex;
          _previousArtworkFile = _lastArtworkFile;
          _currentShapeIndex = _pickFreeshapeIndexWithRecency(track?.id ?? 'default');
          _lastTrackId = track?.id;
          _lastArtworkFile = track?.artworkFile;
          if (_lastTrackId != null && _previousArtworkFile != null) {
            _morphController?.forward(from: 0.0);
          }
        }

        // If lyrics mode is activated on mobile, render the full-page SynchronizedLyricsStage
        if (_showLyrics) {
          final skin = settings.nowPlayingSkin;
          final hasArtwork = track?.artworkFile != null && File(track!.artworkFile!).existsSync();
          final parsedLyrics = LyricsService.instance.currentParsedLyrics;
          final hasRealLyrics = parsedLyrics != null &&
              parsedLyrics.hasLyrics &&
              parsedLyrics.lines.isNotEmpty &&
              !parsedLyrics.lines.every((l) => l.text.toLowerCase().contains('no lyrics'));

          return Container(
            color: colorScheme.surface.withValues(alpha: 0.20),
            child: Stack(
              children: [
                // In Material You Freeshapes skin, show toned-down background freeshape art ONLY when lyrics exist
                if (skin == NowPlayingSkin.materialYouFreeshapes && hasArtwork && hasRealLyrics)
                  Positioned.fill(
                    child: Center(
                      child: Opacity(
                        opacity: 0.22,
                        child: SizedBox(
                          width: 320,
                          height: 320,
                          child: ClipPath(
                            clipper: MorphingFreeshapeClipper(
                              shapeFrom: _currentShapeIndex,
                              shapeTo: _currentShapeIndex,
                              progress: 1.0,
                            ),
                            child: Image.file(
                              File(track.artworkFile!),
                              fit: BoxFit.cover,
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                SynchronizedLyricsStage(
                  isFullPage: true,
                  onBackToArtwork: () => setState(() => _showLyrics = false),
                ),
              ],
            ),
          );
        }

        return Container(
          color: colorScheme.surface.withValues(alpha: 0.20),
          child: SafeArea(
            child: SingleChildScrollView(
              physics: const BouncingScrollPhysics(),
              padding: const EdgeInsets.symmetric(horizontal: 24.0, vertical: 12.0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.center,
                children: [
                  // 0. Top Bar: Dismiss/Collapse Button, Status / Mode Segment, Style & Settings
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      if (widget.onDismiss != null)
                        IconButton(
                          icon: const Icon(Icons.keyboard_arrow_down_rounded, size: 28),
                          color: colorScheme.onSurface.withValues(alpha: 0.8),
                          onPressed: widget.onDismiss,
                          tooltip: 'Collapse to main screen',
                        )
                      else
                        const SizedBox(width: 48),

                      // Mobile: Segmented switch (Artwork vs Lyrics). Tablet/Desktop: Sleek badge pill
                      if (!isTabletDesktop)
                        LiquidGlassContainer(
                          padding: const EdgeInsets.all(3),
                          borderRadius: 16,
                          surfaceAlpha: 0.16,
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              _buildModeTab('Artwork', !_showLyrics, colorScheme, () {
                                setState(() => _showLyrics = false);
                              }),
                              _buildModeTab('Lyrics', _showLyrics, colorScheme, () {
                                setState(() => _showLyrics = true);
                              }),
                            ],
                          ),
                        )
                      else
                        LiquidGlassContainer(
                          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
                          borderRadius: 16,
                          surfaceAlpha: 0.16,
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(Icons.graphic_eq_rounded, size: 16, color: colorScheme.primary),
                              const SizedBox(width: 6),
                              Text(
                                'NOW PLAYING',
                                style: AttuneTypography.soraBadge(color: colorScheme.primary, fontSize: 10),
                              ),
                            ],
                          ),
                        ),

                      Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          if (track != null)
                            IconButton(
                              icon: Icon(
                                controller.isFavorite(track.id)
                                    ? Icons.favorite_rounded
                                    : Icons.favorite_border_rounded,
                                size: 22,
                                color: controller.isFavorite(track.id)
                                    ? const Color(0xFFFF4081)
                                    : colorScheme.onSurface.withValues(alpha: 0.8),
                              ),
                              onPressed: () => controller.toggleFavorite(track.id),
                              tooltip: controller.isFavorite(track.id) ? 'Favorited' : 'Favorite track',
                            ),
                          IconButton(
                            icon: const Icon(Icons.equalizer_rounded, size: 22),
                            color: colorScheme.onSurface.withValues(alpha: 0.8),
                            onPressed: () {
                              Navigator.of(context).push(
                                MaterialPageRoute(
                                  builder: (_) => const EqualizerStage(),
                                ),
                              );
                            },
                            tooltip: 'Attune Studio Master FX & Equalizer',
                          ),
                          IconButton(
                            icon: const Icon(Icons.palette_rounded, size: 22),
                            color: colorScheme.onSurface.withValues(alpha: 0.8),
                            onPressed: () {
                              Navigator.of(context).push(
                                MaterialPageRoute(
                                  builder: (_) => const NowPlayingSkinPickerStage(),
                                ),
                              );
                            },
                            tooltip: 'Player Skins & Visual Aesthetics',
                          ),
                          IconButton(
                            icon: const Icon(Icons.tune_rounded, size: 22),
                            color: colorScheme.onSurface.withValues(alpha: 0.8),
                            onPressed: () => _showPlayerStyleDialog(context, colorScheme),
                            tooltip: 'Living Beat & Glass Refraction',
                          ),
                          IconButton(
                            icon: const Icon(Icons.settings_outlined, size: 22),
                            color: colorScheme.onSurface.withValues(alpha: 0.8),
                            onPressed: () => PlatformNavigation.openSettings(context),
                            tooltip: 'Audio Settings',
                          ),
                        ],
                      ),
                    ],
                  ),

                  const SizedBox(height: 16),

                  // 1. Dynamic Hero Artwork Card based on active NowPlayingSkin
                  _buildHeroArtwork(track, colorScheme, settings.nowPlayingSkin, MediaQuery.sizeOf(context).width),

                  const SizedBox(height: 20),

                  // 2. Track Title, Artist, and Live Timed Lyric Snippet (for Normal Square Big)
                  () {
                    final isNormalSquareBig = settings.nowPlayingSkin == NowPlayingSkin.normalSquareBig;
                    final heroSize = (MediaQuery.sizeOf(context).width >= 720)
                        ? 360.0
                        : math.min(MediaQuery.sizeOf(context).width - 48, 340.0);

                    // Query timed lyrics for Normal Square Big live position line
                    final parsed = LyricsService.instance.currentParsedLyrics;
                    final isTimed = parsed != null &&
                        parsed.hasLyrics &&
                        parsed.lines.any((l) => l.timeSec >= 0.0);
                    LyricLine? activeLine;
                    if (isTimed && parsed.lines.isNotEmpty) {
                      int idx = 0;
                      for (int i = 0; i < parsed.lines.length; i++) {
                        if (parsed.lines[i].timeSec >= 0 && posSec >= parsed.lines[i].timeSec) {
                          idx = i;
                        } else if (parsed.lines[i].timeSec > posSec) {
                          break;
                        }
                      }
                      activeLine = parsed.lines[idx];
                    }

                    if (isNormalSquareBig) {
                      return SizedBox(
                        width: heroSize,
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            // Small space with current lyrics position (cream white / brightly colored syllables)
                            if (isTimed && activeLine != null)
                              _buildNormalSquareBigLyricSnippet(activeLine, posSec, colorScheme),
                            Row(
                              crossAxisAlignment: CrossAxisAlignment.center,
                              children: [
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      Text(
                                        track?.title ?? (controller.queue.isEmpty ? 'No Track Playing' : 'Rabbit Hole'),
                                        style: AttuneTypography.soraTitleLarge(color: colorScheme.onSurface),
                                        textAlign: TextAlign.left,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                      ),
                                      const SizedBox(height: 4),
                                      Text(
                                        track?.artist ?? (controller.queue.isEmpty ? 'Queue is empty' : 'DECO*27 feat. Hatsune Miku'),
                                        style: AttuneTypography.jakartaArtistSubtitle(
                                          color: colorScheme.onSurface.withValues(alpha: 0.72),
                                        ),
                                        textAlign: TextAlign.left,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                      ),
                                    ],
                                  ),
                                ),
                                if (track != null) ...[
                                  IconButton(
                                    icon: Icon(
                                      controller.isFavorite(track.id)
                                          ? Icons.favorite_rounded
                                          : Icons.favorite_border_rounded,
                                      size: 26,
                                      color: controller.isFavorite(track.id)
                                          ? const Color(0xFFFF4081)
                                          : colorScheme.onSurface.withValues(alpha: 0.7),
                                    ),
                                    tooltip: controller.isFavorite(track.id) ? 'Remove from favorites' : 'Favorite track',
                                    onPressed: () => controller.toggleFavorite(track.id),
                                  ),
                                ],
                              ],
                            ),
                          ],
                        ),
                      );
                    }

                    return SizedBox(
                      width: heroSize,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.center,
                        children: [
                          Text(
                            track?.title ?? (controller.queue.isEmpty ? 'No Track Playing' : 'Rabbit Hole'),
                            style: AttuneTypography.soraTitleLarge(color: colorScheme.onSurface),
                            textAlign: TextAlign.center,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          const SizedBox(height: 4),
                          Text(
                            track?.artist ?? (controller.queue.isEmpty ? 'Queue is empty' : 'DECO*27 feat. Hatsune Miku'),
                            style: AttuneTypography.jakartaArtistSubtitle(
                              color: colorScheme.onSurface.withValues(alpha: 0.72),
                            ),
                            textAlign: TextAlign.center,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    );
                  }(),

                  const SizedBox(height: 20),

                  // 4. Subtle Cue-Zone Wave Progress Bar
                  Column(
                    children: [
                      SliderTheme(
                        data: SliderTheme.of(context).copyWith(
                          trackHeight: 4,
                          activeTrackColor: colorScheme.primary,
                          inactiveTrackColor: colorScheme.surfaceContainerHigh.withValues(alpha: 0.3),
                          thumbColor: Colors.white,
                          thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                          overlayShape: const RoundSliderOverlayShape(overlayRadius: 14),
                        ),
                        child: Slider(
                          value: posSec.clamp(0.0, durSec),
                          min: 0.0,
                          max: durSec,
                          onChanged: (v) => controller.seek(v),
                        ),
                      ),
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 16.0),
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Text(
                              '${currentMin.toString().padLeft(2, '0')}:${currentSec.toString().padLeft(2, '0')}',
                              style: AttuneTypography.soraMetric(
                                fontSize: 12,
                                color: colorScheme.onSurface.withValues(alpha: 0.6),
                              ),
                            ),
                            Text(
                              '${totalMin.toString().padLeft(2, '0')}:${totalSec.toString().padLeft(2, '0')}',
                              style: AttuneTypography.soraMetric(
                                fontSize: 12,
                                color: colorScheme.onSurface.withValues(alpha: 0.6),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),

                  const SizedBox(height: 14),

                  // 5. Transport Controls with Tri-State Shuffle (Off, Standard, Smart)
                  Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      _buildShuffleButton(controller, colorScheme),
                      const SizedBox(width: 8),
                      IconButton(
                        icon: const Icon(Icons.skip_previous_rounded),
                        iconSize: 34,
                        visualDensity: VisualDensity.compact,
                        color: colorScheme.onSurface,
                        onPressed: widget.onPrevious ?? controller.skipPrevious,
                      ),
                      const SizedBox(width: 12),

                      // Play / Pause Glass Button
                      LiquidGlassContainer(
                        width: 68,
                        height: 68,
                        padding: EdgeInsets.zero,
                        borderRadius: 34,
                        surfaceAlpha: 0.22,
                        tintOverride: colorScheme.primary.withValues(alpha: 0.28),
                        onTap: widget.onTogglePlay ?? controller.togglePlay,
                        child: Center(
                          child: Icon(
                            playing ? Icons.pause_rounded : Icons.play_arrow_rounded,
                            size: 36,
                            color: Colors.white,
                          ),
                        ),
                      ),

                      const SizedBox(width: 12),
                      IconButton(
                        icon: const Icon(Icons.skip_next_rounded),
                        iconSize: 34,
                        visualDensity: VisualDensity.compact,
                        color: colorScheme.onSurface,
                        onPressed: widget.onNext ?? controller.skipNext,
                      ),
                      const SizedBox(width: 8),
                      IconButton(
                        icon: const Icon(Icons.repeat_rounded),
                        iconSize: 22,
                        visualDensity: VisualDensity.compact,
                        color: colorScheme.onSurface.withValues(alpha: 0.7),
                        onPressed: () {},
                      ),
                    ],
                  ),

                  const SizedBox(height: 20),

                  // 6. Subtle Inline Transition Connector Pill (Section 3.2)
                  // Tapping displays why this archetype was selected rather than raw diagnostics
                  if (controller.isTransitioning ||
                      controller.shuffleMode == ShuffleMode.smart) ...[
                    Builder(
                      builder: (context) {
                        final recipe = controller.pendingRecipe;
                        final timing = recipe?['timing'] as Map<String, dynamic>?;
                        final durSec = timing != null
                            ? (((timing['track_a_exit_end_ms'] as num? ?? 1400) -
                                    (timing['track_a_transition_start_ms'] as num? ?? 0))
                                .abs() / 1000.0)
                            : 1.4;
                        final conf = (recipe?['confidence_score'] as num?)?.toDouble() ?? 0.95;
                        return TransitionConnectorPill(
                          archetype: controller.activeArchetype,
                          durationSeconds: durSec,
                          confidenceScore: conf,
                          onTap: () => _showTransitionDetailsDialog(context, controller, colorScheme),
                        );
                      },
                    ),
                    const SizedBox(height: 16),
                  ],

                  // 7. Mini Up-Next Row Preview (Cleaned up, no duplicate pill)
                  if (nextTrack != null)
                    LiquidGlassContainer(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                      borderRadius: 20,
                      surfaceAlpha: 0.12,
                      onTap: () => _showTransitionDetailsDialog(context, controller, colorScheme),
                      child: Row(
                        children: [
                          Container(
                            width: 36,
                            height: 36,
                            decoration: BoxDecoration(
                              borderRadius: BorderRadius.circular(10),
                              gradient: LinearGradient(
                                colors: [nextTrack.seedColor, nextTrack.seedColor.withValues(alpha: 0.5)],
                              ),
                            ),
                            child: ClipRRect(
                              borderRadius: BorderRadius.circular(10),
                              child: nextTrack.artworkFile != null && File(nextTrack.artworkFile!).existsSync()
                                  ? Image.file(File(nextTrack.artworkFile!), fit: BoxFit.cover)
                                  : const Icon(Icons.music_note, color: Colors.white, size: 20),
                            ),
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  'UP NEXT',
                                  style: AttuneTypography.soraBadge(
                                    color: colorScheme.onSurface.withValues(alpha: 0.6),
                                    fontSize: 9,
                                  ),
                                ),
                                Text(
                                  nextTrack.title,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: AttuneTypography.soraSubheading(
                                    fontSize: 13,
                                    fontWeight: FontWeight.w600,
                                    color: colorScheme.onSurface,
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),

                  const SizedBox(height: 80),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  void _showTransitionDetailsDialog(
    BuildContext context,
    PlaybackController controller,
    ColorScheme colorScheme,
  ) {
    showTransitionDetailsDialog(context, controller, colorScheme);
  }

  void _showPlayerStyleDialog(BuildContext context, ColorScheme colorScheme) {
    showDialog(
      context: context,
      barrierColor: Colors.black.withValues(alpha: 0.45),
      builder: (context) {
        return ListenableBuilder(
          listenable: PlaybackController.instance,
          builder: (context, _) {
            final controller = PlaybackController.instance;
            return Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 400),
                child: LiquidGlassContainer(
                  margin: const EdgeInsets.symmetric(horizontal: 24),
                  padding: const EdgeInsets.all(24),
                  borderRadius: 28,
                  surfaceAlpha: 0.22,
                  blurSigma: 32,
                  child: Material(
                    color: Colors.transparent,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Text(
                              'Player Style & Glass Refraction',
                              style: AttuneTypography.soraHeading(
                                fontSize: 16,
                                fontWeight: FontWeight.bold,
                                color: colorScheme.onSurface,
                              ),
                            ),
                            IconButton(
                              icon: const Icon(Icons.close_rounded, size: 20),
                              color: colorScheme.onSurface.withValues(alpha: 0.6),
                              onPressed: () => Navigator.of(context).pop(),
                            ),
                          ],
                        ),
                        const SizedBox(height: 16),
                        ListTile(
                          contentPadding: EdgeInsets.zero,
                          leading: Icon(Icons.blur_on_rounded, color: colorScheme.primary),
                          title: Text(
                            'Living Beat & Lavalamp Globs',
                            style: AttuneTypography.soraSubheading(fontSize: 13, color: colorScheme.onSurface),
                          ),
                          subtitle: Text(
                            'Fluid liquid light globs pulsing dynamically to track BPM & kick',
                            style: AttuneTypography.jakartaBodySmall(color: colorScheme.onSurface.withValues(alpha: 0.6)),
                          ),
                          trailing: Switch(
                            value: controller.livingBeatMeshEnabled,
                            activeThumbColor: colorScheme.primary,
                            onChanged: (v) {
                              controller.setLivingBeatMeshEnabled(v);
                            },
                          ),
                        ),
                        ListTile(
                          contentPadding: EdgeInsets.zero,
                          leading: Icon(Icons.water_drop_rounded, color: colorScheme.secondary),
                          title: Text(
                            'Frosted Glass Depth',
                            style: AttuneTypography.soraSubheading(fontSize: 13, color: colorScheme.onSurface),
                          ),
                          subtitle: Text(
                            'Viscous liquid refraction with 40px backdrop blur',
                            style: AttuneTypography.jakartaBodySmall(color: colorScheme.onSurface.withValues(alpha: 0.6)),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            );
          },
        );
      },
    );
  }

  Widget _buildModeTab(String label, bool isSelected, ColorScheme colorScheme, VoidCallback onTap) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected ? colorScheme.primary.withValues(alpha: 0.28) : Colors.transparent,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Text(
          label,
          style: AttuneTypography.soraBadge(
            color: isSelected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.6),
            fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
          ),
        ),
      ),
    );
  }

  Widget _buildShuffleButton(PlaybackController controller, ColorScheme colorScheme) {
    IconData icon;
    Color color;
    String tooltip;

    switch (controller.shuffleMode) {
      case ShuffleMode.off:
        icon = Icons.shuffle_rounded;
        color = colorScheme.onSurface.withValues(alpha: 0.35);
        tooltip = controller.isSmartShuffleSupported
            ? 'Shuffle Off (Linear Sequence)'
            : 'Shuffle Off (Linear Sequence • Smart Shuffle requires Attune Engine)';
        break;
      case ShuffleMode.standard:
        icon = Icons.shuffle_rounded;
        color = colorScheme.primary;
        tooltip = controller.isSmartShuffleSupported
            ? 'Basic Shuffle (Random Track)'
            : 'Basic Shuffle (System Codec • Smart Shuffle requires Attune Engine)';
        break;
      case ShuffleMode.smart:
        icon = Icons.auto_awesome_rounded;
        color = colorScheme.tertiary;
        tooltip = 'Smart Shuffle (Algorithmic Transition Match)';
        break;
    }

    return IconButton(
      icon: Icon(icon),
      iconSize: 22,
      visualDensity: VisualDensity.compact,
      color: color,
      tooltip: tooltip,
      onPressed: controller.cycleShuffleMode,
    );
  }

  Widget _buildFallbackCover(ColorScheme colorScheme) {
    return Container(
      decoration: BoxDecoration(
        gradient: LinearGradient(
          colors: [
            colorScheme.primary.withValues(alpha: 0.5),
            colorScheme.secondary.withValues(alpha: 0.3),
            colorScheme.surface,
          ],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
      ),
      child: Center(
        child: Opacity(
          opacity: 0.85,
          child: Image.asset(
            'assets/icon/icon_transparent.png',
            width: 96,
            height: 96,
            fit: BoxFit.contain,
          ),
        ),
      ),
    );
  }

  Widget _buildHeroArtwork(
    TrackItem? track,
    ColorScheme colorScheme,
    NowPlayingSkin skin,
    double screenWidth,
  ) {
    final heroSize = (screenWidth >= 720) ? 360.0 : math.min(screenWidth - 48, 340.0);

    switch (skin) {
      case NowPlayingSkin.materialYouFreeshapes:
        return _buildFreeshapesArtwork(track, colorScheme, heroSize);
      case NowPlayingSkin.normalSquareBig:
        return _buildNormalSquareBigArtwork(track, colorScheme, heroSize);
      case NowPlayingSkin.defaultSkin:
        return Center(
          child: LiquidGlassContainer(
            key: const ValueKey('artwork_view'),
            width: 320,
            height: 320,
            padding: EdgeInsets.zero,
            borderRadius: 32,
            surfaceAlpha: 0.16,
            child: Stack(
              fit: StackFit.expand,
              children: [
                if (track?.artworkFile != null && File(track!.artworkFile!).existsSync())
                  ClipRRect(
                    borderRadius: BorderRadius.circular(32),
                    child: Image.file(
                      File(track.artworkFile!),
                      fit: BoxFit.cover,
                      errorBuilder: (context, error, stackTrace) => _buildFallbackCover(colorScheme),
                    ),
                  )
                else
                  _buildFallbackCover(colorScheme),
              ],
            ),
          ),
        );
    }
  }

  Widget _buildFreeshapesArtwork(TrackItem? track, ColorScheme colorScheme, double size) {
    final hasArtwork = track?.artworkFile != null && File(track!.artworkFile!).existsSync();
    final hasPrevArtwork = _previousArtworkFile != null && File(_previousArtworkFile!).existsSync();

    return Center(
      child: AnimatedBuilder(
        animation: _morphCurvedAnim ?? const AlwaysStoppedAnimation(1.0),
        builder: (context, _) {
          final progress = _morphCurvedAnim?.value ?? 1.0;

          return SizedBox(
            width: size,
            height: size,
            child: Stack(
              alignment: Alignment.center,
              children: [
                // Ambient soft radial glow behind shape
                Container(
                  width: size * 0.90,
                  height: size * 0.90,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    boxShadow: [
                      BoxShadow(
                        color: (track?.seedColor ?? colorScheme.primary).withValues(alpha: 0.35),
                        blurRadius: 36,
                        spreadRadius: 4,
                      ),
                    ],
                  ),
                ),
                ClipPath(
                  clipper: MorphingFreeshapeClipper(
                    shapeFrom: _previousShapeIndex,
                    shapeTo: _currentShapeIndex,
                    progress: progress,
                  ),
                  child: SizedBox(
                    width: size,
                    height: size,
                    child: Stack(
                      fit: StackFit.expand,
                      children: [
                        // Previous artwork fading out during morphological morph
                        if (hasPrevArtwork && progress < 1.0)
                          Opacity(
                            opacity: (1.0 - progress).clamp(0.0, 1.0),
                            child: Image.file(
                              File(_previousArtworkFile!),
                              fit: BoxFit.cover,
                            ),
                          ),
                        // Current artwork fading in
                        Opacity(
                          opacity: progress.clamp(0.0, 1.0),
                          child: hasArtwork
                              ? Image.file(
                                  File(track.artworkFile!),
                                  fit: BoxFit.cover,
                                  errorBuilder: (context, error, stackTrace) => _buildFallbackCover(colorScheme),
                                )
                              : _buildFallbackCover(colorScheme),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }

  /// Live snippet of active timed lyric position for NormalSquareBig skin
  Widget _buildNormalSquareBigLyricSnippet(LyricLine activeLine, double currentPosSec, ColorScheme colorScheme) {
    final hasSyllable = activeLine.hasSyllableSync;

    return Padding(
      padding: const EdgeInsets.only(bottom: 10.0),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: Colors.white.withValues(alpha: 0.06),
          borderRadius: BorderRadius.circular(10),
          border: Border.all(
            color: const Color(0xFFFFFDD0).withValues(alpha: 0.16),
            width: 1,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.music_note_rounded,
              size: 14,
              color: const Color(0xFFFFFDD0).withValues(alpha: 0.8),
            ),
            const SizedBox(width: 6),
            Flexible(
              child: hasSyllable
                  ? RichText(
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      text: TextSpan(
                        children: activeLine.words.map((word) {
                          final isSung = currentPosSec >= word.endSec;
                          final isCurrent = currentPosSec >= word.startSec && currentPosSec < word.endSec;

                          Color textColor;
                          FontWeight fontWeight;
                          if (isCurrent) {
                            textColor = Colors.white; // Bright highlighted active syllable
                            fontWeight = FontWeight.w800;
                          } else if (isSung) {
                            textColor = const Color(0xFFFFFDD0); // Cream white sung words
                            fontWeight = FontWeight.w600;
                          } else {
                            textColor = const Color(0xFFFFFDD0).withValues(alpha: 0.40); // Dimmed upcoming words
                            fontWeight = FontWeight.w400;
                          }

                          return TextSpan(
                            text: '${word.text} ',
                            style: AttuneTypography.jakartaBodySmall(
                              color: textColor,
                              fontSize: 12,
                            ).copyWith(fontWeight: fontWeight),
                          );
                        }).toList(),
                      ),
                    )
                  : Text(
                      activeLine.text,
                      style: AttuneTypography.jakartaBodySmall(
                        color: const Color(0xFFFFFDD0),
                        fontSize: 12,
                      ).copyWith(fontWeight: FontWeight.w600),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildNormalSquareBigArtwork(TrackItem? track, ColorScheme colorScheme, double size) {
    final hasArtwork = track?.artworkFile != null && File(track!.artworkFile!).existsSync();

    return Center(
      child: GestureDetector(
        onLongPress: () {
          setState(() => _showLyrics = !_showLyrics);
        },
        child: Container(
          width: size,
          height: size,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(24),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withValues(alpha: 0.45),
                blurRadius: 30,
                offset: const Offset(0, 10),
              ),
            ],
          ),
          child: Stack(
            fit: StackFit.expand,
            children: [
              ClipRRect(
                borderRadius: BorderRadius.circular(24),
                child: hasArtwork
                    ? Image.file(
                        File(track.artworkFile!),
                        fit: BoxFit.cover,
                        errorBuilder: (context, error, stackTrace) => _buildFallbackCover(colorScheme),
                      )
                    : _buildFallbackCover(colorScheme),
              ),
              // Side subtitles / lyrics toggle button on lower right
              Positioned(
                right: 12,
                bottom: 12,
                child: GestureDetector(
                  onTap: () => setState(() => _showLyrics = !_showLyrics),
                  child: LiquidGlassContainer(
                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                    borderRadius: 14,
                    surfaceAlpha: 0.38,
                    blurSigma: 24,
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          _showLyrics ? Icons.album_rounded : Icons.subtitles_rounded,
                          size: 16,
                          color: Colors.white,
                        ),
                        const SizedBox(width: 5),
                        Text(
                          _showLyrics ? 'Cover' : 'Lyrics',
                          style: AttuneTypography.soraBadge(fontSize: 10, color: Colors.white),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Material You Expressive Freeshapes: Recency Scoring & Morphological Morphing
// ---------------------------------------------------------------------------

/// Recency ring buffer preventing consecutive/repetitive shapes
final List<int> _recentFreeshapeIndices = [];

/// Evaluates normalized radius r(theta) for theta in [0, 2*pi] across 8 distinct shapes
double _evaluateFreeshapeRadius(int shapeIndex, double theta) {
  switch (shapeIndex) {
    case 0: // 8-Petal Flower
      return 0.86 + 0.14 * math.cos(8 * theta);
    case 1: // 4-Leaf Clover
      return 0.82 + 0.18 * math.cos(4 * theta);
    case 2: // 6-Point Star
      return 0.80 + 0.20 * math.cos(6 * theta);
    case 3: // Organic Fluid Blob (Asymmetric Harmonic contour)
      return 0.82 + 0.12 * math.cos(theta) + 0.10 * math.sin(2 * theta) - 0.08 * math.cos(3 * theta);
    case 4: // Squircle (Continuous superellipse approximation)
      return 0.90 + 0.10 * math.cos(4 * theta) - 0.03 * math.cos(8 * theta);
    case 5: // 12-Point Scallop / Sunburst
      return 0.89 + 0.11 * math.cos(12 * theta);
    case 6: // 5-Point Star
      return 0.80 + 0.20 * math.cos(5 * theta);
    case 7: // Rounded Diamond
    default:
      return 0.78 + 0.22 * math.cos(2 * theta);
  }
}

/// Selects an expressive shape with progressive recency penalty
int _pickFreeshapeIndexWithRecency(String trackId) {
  const totalShapes = 8;
  int bestIndex = 0;
  double bestScore = -999999.0;

  for (int k = 0; k < totalShapes; k++) {
    final hashVal = (trackId.hashCode.abs() + (k * 43) + 17) % 1000;
    double score = hashVal / 1000.0;

    // Apply heavy recency penalty to recently used shapes
    for (int r = 0; r < _recentFreeshapeIndices.length; r++) {
      if (_recentFreeshapeIndices[r] == k) {
        score -= (100.0 / (r + 1));
      }
    }

    if (score > bestScore) {
      bestScore = score;
      bestIndex = k;
    }
  }

  // Update ring buffer
  _recentFreeshapeIndices.insert(0, bestIndex);
  if (_recentFreeshapeIndices.length > 5) {
    _recentFreeshapeIndices.removeLast();
  }

  return bestIndex;
}

/// Morphological CustomClipper that fluidly interpolates contour points from shapeFrom to shapeTo
class MorphingFreeshapeClipper extends CustomClipper<Path> {
  final int shapeFrom;
  final int shapeTo;
  final double progress; // 0.0 to 1.0

  const MorphingFreeshapeClipper({
    required this.shapeFrom,
    required this.shapeTo,
    required this.progress,
  });

  @override
  Path getClip(Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = size.width / 2;
    final path = Path();
    const steps = 140;

    for (int i = 0; i <= steps; i++) {
      final theta = (i / steps) * 2 * math.pi;
      final rFrom = _evaluateFreeshapeRadius(shapeFrom, theta);
      final rTo = _evaluateFreeshapeRadius(shapeTo, theta);
      final blendedMod = (1.0 - progress) * rFrom + progress * rTo;
      final r = radius * blendedMod;
      final x = center.dx + r * math.cos(theta);
      final y = center.dy + r * math.sin(theta);
      if (i == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }
    path.close();
    return path;
  }

  @override
  bool shouldReclip(covariant MorphingFreeshapeClipper oldClipper) {
    return oldClipper.shapeFrom != shapeFrom ||
        oldClipper.shapeTo != shapeTo ||
        oldClipper.progress != progress;
  }
}


class FreeshapeFlowerClipper extends CustomClipper<Path> {
  final int petals;
  final double depth;
  const FreeshapeFlowerClipper({this.petals = 8, this.depth = 0.14});
  @override
  Path getClip(Size size) => const MorphingFreeshapeClipper(shapeFrom: 0, shapeTo: 0, progress: 1.0).getClip(size);
  @override
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}

class FreeshapeCloverClipper extends CustomClipper<Path> {
  const FreeshapeCloverClipper();
  @override
  Path getClip(Size size) => const MorphingFreeshapeClipper(shapeFrom: 1, shapeTo: 1, progress: 1.0).getClip(size);
  @override
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}

class FreeshapeStarClipper extends CustomClipper<Path> {
  final int points;
  const FreeshapeStarClipper({this.points = 6});
  @override
  Path getClip(Size size) => const MorphingFreeshapeClipper(shapeFrom: 2, shapeTo: 2, progress: 1.0).getClip(size);
  @override
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}

class FreeshapeOrganicBlobClipper extends CustomClipper<Path> {
  const FreeshapeOrganicBlobClipper();
  @override
  Path getClip(Size size) => const MorphingFreeshapeClipper(shapeFrom: 3, shapeTo: 3, progress: 1.0).getClip(size);
  @override
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}

class FreeshapeSquircleClipper extends CustomClipper<Path> {
  const FreeshapeSquircleClipper();
  @override
  Path getClip(Size size) => const MorphingFreeshapeClipper(shapeFrom: 4, shapeTo: 4, progress: 1.0).getClip(size);
  @override
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}
