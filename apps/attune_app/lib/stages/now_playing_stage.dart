import 'dart:io';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import '../models/smart_shuffle_verdict.dart';
import '../models/track_item.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../services/settings_service.dart';
import '../theme/color_engine.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/transition_connector_pill.dart';
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

class _NowPlayingStageState extends State<NowPlayingStage> {
  bool _showLyrics = false;

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

        // If lyrics mode is activated on mobile, render the full-page SynchronizedLyricsStage
        if (_showLyrics) {
          final skin = settings.nowPlayingSkin;
          final hasArtwork = track?.artworkFile != null && File(track!.artworkFile!).existsSync();

          return Container(
            color: colorScheme.surface.withValues(alpha: 0.20),
            child: Stack(
              children: [
                // In Material You Freeshapes skin, show toned-down / dimmed background freeshape art
                if (skin == NowPlayingSkin.materialYouFreeshapes && hasArtwork)
                  Positioned.fill(
                    child: Center(
                      child: Opacity(
                        opacity: 0.22,
                        child: SizedBox(
                          width: 320,
                          height: 320,
                          child: ClipPath(
                            clipper: _getFreeshapeClipper(track.id),
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

                  const SizedBox(height: 24),

                  // 2. Track Title & Artist
                  Text(
                    track?.title ?? 'Rabbit Hole',
                    style: AttuneTypography.soraTitleLarge(color: colorScheme.onSurface),
                    textAlign: TextAlign.center,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    track?.artist ?? 'DECO*27 feat. Hatsune Miku',
                    style: AttuneTypography.jakartaArtistSubtitle(
                      color: colorScheme.onSurface.withValues(alpha: 0.72),
                    ),
                    textAlign: TextAlign.center,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),

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
    int activeTab = 0; // 0: Smart Shuffle Verdict, 1: Transition DSP Recipe

    showDialog(
      context: context,
      barrierColor: Colors.black.withValues(alpha: 0.45),
      builder: (context) {
        return StatefulBuilder(
          builder: (context, setDialogState) {
            final verdict = controller.getOrComputeSmartShuffleVerdict();
            final recipe = controller.pendingRecipe;
            final archetype = controller.activeArchetype.isNotEmpty ? controller.activeArchetype : '3-Band EQ Crossfade';
            final conf = (recipe?['confidence_score'] as num?)?.toDouble() ?? (verdict.winner.matchPercentage / 100.0);
            final matchPct = (conf * 100).toInt();

            return Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 500, maxHeight: 680),
                child: LiquidGlassContainer(
                  margin: const EdgeInsets.symmetric(horizontal: 20, vertical: 24),
                  padding: const EdgeInsets.all(22),
                  borderRadius: 28,
                  surfaceAlpha: 0.22,
                  blurSigma: 32,
                  child: Material(
                    color: Colors.transparent,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        // Top Header Row
                        Row(
                          children: [
                            Container(
                              padding: const EdgeInsets.all(10),
                              decoration: BoxDecoration(
                                shape: BoxShape.circle,
                                color: colorScheme.primary.withValues(alpha: 0.2),
                              ),
                              child: Icon(
                                activeTab == 0 ? Icons.auto_awesome_rounded : Icons.equalizer_rounded,
                                color: colorScheme.primary,
                                size: 22,
                              ),
                            ),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    'Transition Selection Rationale',
                                    style: AttuneTypography.soraHeading(
                                      fontSize: 16,
                                      fontWeight: FontWeight.bold,
                                      color: colorScheme.onSurface,
                                    ),
                                  ),
                                  Text(
                                    '$archetype • $matchPct% Harmonic Alignment',
                                    style: AttuneTypography.jakartaBodySmall(
                                      color: colorScheme.onSurface.withValues(alpha: 0.7),
                                    ),
                                  ),
                                ],
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

                        // Segmented Tab Selector
                        Container(
                          padding: const EdgeInsets.all(3),
                          decoration: BoxDecoration(
                            color: colorScheme.onSurface.withValues(alpha: 0.08),
                            borderRadius: BorderRadius.circular(14),
                          ),
                          child: Row(
                            children: [
                              Expanded(
                                child: _buildVerdictTabButton(
                                  label: '✦ Smart Shuffle Verdict',
                                  isSelected: activeTab == 0,
                                  colorScheme: colorScheme,
                                  onTap: () => setDialogState(() => activeTab = 0),
                                ),
                              ),
                              Expanded(
                                child: _buildVerdictTabButton(
                                  label: '∿ Transition DSP Recipe',
                                  isSelected: activeTab == 1,
                                  colorScheme: colorScheme,
                                  onTap: () => setDialogState(() => activeTab = 1),
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(height: 16),

                        // Scrollable Tab View
                        Flexible(
                          child: SingleChildScrollView(
                            physics: const BouncingScrollPhysics(),
                            child: activeTab == 0
                                ? _buildSmartShuffleVerdictView(verdict, controller, colorScheme, setDialogState)
                                : _buildDspRecipeView(colorScheme),
                          ),
                        ),
                        const SizedBox(height: 18),

                        // Footer Actions
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Flexible(
                              child: TextButton.icon(
                                icon: const Icon(Icons.analytics_outlined, size: 16),
                                label: const Text(
                                  'Open DSP Diagnostics',
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                                style: TextButton.styleFrom(
                                  foregroundColor: colorScheme.onSurface.withValues(alpha: 0.7),
                                  textStyle: AttuneTypography.soraBadge(fontSize: 11),
                                ),
                                onPressed: () {
                                  Navigator.of(context).pop();
                                  PlatformNavigation.openTransitionInspector(context);
                                },
                              ),
                            ),
                            const SizedBox(width: 12),
                            LiquidGlassContainer(
                              padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 8),
                              borderRadius: 14,
                              surfaceAlpha: 0.26,
                              tintOverride: colorScheme.primary.withValues(alpha: 0.28),
                              onTap: () => Navigator.of(context).pop(),
                              child: Text(
                                'Got it',
                                style: AttuneTypography.soraBadge(
                                  color: Colors.white,
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ),
                          ],
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

  Widget _buildVerdictTabButton({
    required String label,
    required bool isSelected,
    required ColorScheme colorScheme,
    required VoidCallback onTap,
  }) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 8),
        decoration: BoxDecoration(
          color: isSelected ? colorScheme.primary.withValues(alpha: 0.25) : Colors.transparent,
          borderRadius: BorderRadius.circular(12),
        ),
        alignment: Alignment.center,
        child: Text(
          label,
          style: AttuneTypography.soraBadge(
            fontSize: 11,
            color: isSelected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.65),
            fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
          ),
        ),
      ),
    );
  }

  Widget _buildSmartShuffleVerdictView(
    SmartShuffleVerdict verdict,
    PlaybackController controller,
    ColorScheme colorScheme,
    void Function(void Function()) setDialogState,
  ) {
    final winner = verdict.winner;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Winner Card
        LiquidGlassContainer(
          padding: const EdgeInsets.all(14),
          borderRadius: 18,
          surfaceAlpha: 0.18,
          tintOverride: colorScheme.primary.withValues(alpha: 0.16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Expanded(
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(Icons.stars_rounded, size: 16, color: colorScheme.primary),
                        const SizedBox(width: 6),
                        Flexible(
                          child: Text(
                            'WINNING CANDIDATE • UP NEXT',
                            style: AttuneTypography.soraBadge(
                              fontSize: 10,
                              fontWeight: FontWeight.w700,
                              color: colorScheme.primary,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                    decoration: BoxDecoration(
                      color: colorScheme.primary.withValues(alpha: 0.22),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(
                      '${winner.matchPercentage}% Match',
                      style: AttuneTypography.soraBadge(
                        fontSize: 11,
                        fontWeight: FontWeight.bold,
                        color: colorScheme.primary,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Text(
                winner.track.title,
                style: AttuneTypography.soraHeading(
                  fontSize: 14,
                  fontWeight: FontWeight.bold,
                  color: colorScheme.onSurface,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              Text(
                winner.track.artist,
                style: AttuneTypography.jakartaBodySmall(
                  fontSize: 11,
                  color: colorScheme.onSurface.withValues(alpha: 0.7),
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 10),
              Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: Colors.black.withValues(alpha: 0.18),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Text(
                  verdict.detailedRationale,
                  style: AttuneTypography.jakartaBodySmall(
                    fontSize: 11,
                    color: colorScheme.onSurface.withValues(alpha: 0.85),
                  ),
                ),
              ),
              const SizedBox(height: 10),
              // Factor breakdown chips
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: [
                  _buildVerdictFactorChip(Icons.music_note_rounded, winner.harmonicVerdict, colorScheme),
                  _buildVerdictFactorChip(Icons.speed_rounded, winner.tempoVerdict, colorScheme),
                  _buildVerdictFactorChip(Icons.bolt_rounded, winner.energyVerdict, colorScheme),
                  _buildVerdictFactorChip(Icons.replay_rounded, winner.rotationVerdict, colorScheme),
                ],
              ),
            ],
          ),
        ),

        const SizedBox(height: 18),

        // Alternatives Section
        Row(
          children: [
            Expanded(
              child: Text(
                'Other Evaluated Candidates (${verdict.alternatives.length})',
                style: AttuneTypography.soraSubheading(
                  fontSize: 12,
                  fontWeight: FontWeight.bold,
                  color: colorScheme.onSurface.withValues(alpha: 0.9),
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            const SizedBox(width: 8),
            Text(
              'Runner-ups',
              style: AttuneTypography.jakartaBodySmall(
                fontSize: 10,
                color: colorScheme.onSurface.withValues(alpha: 0.5),
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),

        if (verdict.alternatives.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Text(
              'No other candidates evaluated in current queue.',
              style: AttuneTypography.jakartaBodySmall(
                fontSize: 11,
                color: colorScheme.onSurface.withValues(alpha: 0.5),
              ),
            ),
          )
        else
          ...verdict.alternatives.asMap().entries.map((entry) {
            final rank = entry.key + 2;
            final alt = entry.value;
            return Container(
              margin: const EdgeInsets.only(bottom: 8),
              child: LiquidGlassContainer(
                padding: const EdgeInsets.all(10),
                borderRadius: 14,
                surfaceAlpha: 0.12,
                child: Row(
                  children: [
                    Container(
                      width: 26,
                      height: 26,
                      alignment: Alignment.center,
                      decoration: BoxDecoration(
                        color: colorScheme.onSurface.withValues(alpha: 0.08),
                        shape: BoxShape.circle,
                      ),
                      child: Text(
                        '#$rank',
                        style: AttuneTypography.soraBadge(
                          fontSize: 10,
                          fontWeight: FontWeight.bold,
                          color: colorScheme.onSurface.withValues(alpha: 0.7),
                        ),
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Expanded(
                                child: Text(
                                  alt.track.title,
                                  style: AttuneTypography.soraSubheading(
                                    fontSize: 12,
                                    fontWeight: FontWeight.w600,
                                    color: colorScheme.onSurface,
                                  ),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              const SizedBox(width: 6),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                                decoration: BoxDecoration(
                                  color: colorScheme.secondary.withValues(alpha: 0.16),
                                  borderRadius: BorderRadius.circular(6),
                                ),
                                child: Text(
                                  '${alt.matchPercentage}%',
                                  style: AttuneTypography.soraBadge(
                                    fontSize: 9,
                                    fontWeight: FontWeight.bold,
                                    color: colorScheme.secondary,
                                  ),
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 2),
                          Text(
                            '${alt.track.artist} • ${alt.primaryReason}',
                            style: AttuneTypography.jakartaBodySmall(
                              fontSize: 10,
                              color: colorScheme.onSurface.withValues(alpha: 0.6),
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                    InkWell(
                      borderRadius: BorderRadius.circular(10),
                      onTap: () {
                        controller.selectAlternativeAsUpNext(alt);
                        setDialogState(() {});
                      },
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                        decoration: BoxDecoration(
                          color: colorScheme.primary.withValues(alpha: 0.18),
                          borderRadius: BorderRadius.circular(10),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(Icons.swap_vert_rounded, size: 14, color: colorScheme.primary),
                            const SizedBox(width: 4),
                            Text(
                              'Promote',
                              style: AttuneTypography.soraBadge(
                                fontSize: 10,
                                fontWeight: FontWeight.w600,
                                color: colorScheme.primary,
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
          }),
      ],
    );
  }

  Widget _buildVerdictFactorChip(IconData icon, String label, ColorScheme colorScheme) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: colorScheme.onSurface.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 12, color: colorScheme.primary.withValues(alpha: 0.8)),
          const SizedBox(width: 5),
          Flexible(
            child: Text(
              label,
              style: AttuneTypography.jakartaBodySmall(
                fontSize: 10,
                color: colorScheme.onSurface.withValues(alpha: 0.8),
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildDspRecipeView(ColorScheme colorScheme) {
    return Column(
      children: [
        _buildRationaleItem(
          Icons.equalizer_rounded,
          '3-Band EQ Crossover (180 Hz)',
          'Bass frequencies are isolated and swapped cleanly to avoid destructive low-end phase cancellation and mud.',
          colorScheme,
        ),
        const SizedBox(height: 12),
        _buildRationaleItem(
          Icons.record_voice_over_rounded,
          'Vocal Completion Protection',
          'Transition trigger waits for the active singing phrase contour to conclude, preventing overlapping vocal clash.',
          colorScheme,
        ),
        const SizedBox(height: 12),
        _buildRationaleItem(
          Icons.music_note_rounded,
          'Phrase & Tempo Grid Alignment',
          'Locked to a 4-bar downbeat phrase boundary with micro-pitch preservation.',
          colorScheme,
        ),
      ],
    );
  }

  Widget _buildRationaleItem(IconData icon, String title, String description, ColorScheme colorScheme) {
    return LiquidGlassContainer(
      padding: const EdgeInsets.all(12),
      borderRadius: 16,
      surfaceAlpha: 0.12,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 18, color: colorScheme.primary),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: AttuneTypography.soraSubheading(
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                    color: colorScheme.onSurface,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  description,
                  style: AttuneTypography.jakartaBodySmall(
                    fontSize: 11,
                    color: colorScheme.onSurface.withValues(alpha: 0.65),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
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
    final trackId = track?.id ?? 'default';

    return Center(
      child: AnimatedSwitcher(
        duration: const Duration(milliseconds: 600),
        switchInCurve: Curves.easeInOutCubic,
        switchOutCurve: Curves.easeInOutCubic,
        transitionBuilder: (child, animation) {
          return FadeTransition(
            opacity: animation,
            child: ScaleTransition(
              scale: Tween<double>(begin: 0.90, end: 1.0).animate(animation),
              child: child,
            ),
          );
        },
        child: KeyedSubtree(
          key: ValueKey('freeshape_${track?.id ?? "none"}'),
          child: SizedBox(
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
                  clipper: _getFreeshapeClipper(trackId),
                  child: SizedBox(
                    width: size,
                    height: size,
                    child: hasArtwork
                        ? Image.file(
                            File(track.artworkFile!),
                            fit: BoxFit.cover,
                            errorBuilder: (context, error, stackTrace) => _buildFallbackCover(colorScheme),
                          )
                        : _buildFallbackCover(colorScheme),
                  ),
                ),
              ],
            ),
          ),
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

/// Dynamic shape picker determining expressive M3 contour per track
CustomClipper<Path> _getFreeshapeClipper(String trackId) {
  final idx = trackId.hashCode.abs() % 5;
  switch (idx) {
    case 0:
      return const FreeshapeFlowerClipper(petals: 8, depth: 0.14);
    case 1:
      return const FreeshapeCloverClipper();
    case 2:
      return const FreeshapeStarClipper(points: 6);
    case 3:
      return const FreeshapeOrganicBlobClipper();
    case 4:
    default:
      return const FreeshapeSquircleClipper();
  }
}

/// Expressive Material You Freeshape Clippers
class FreeshapeFlowerClipper extends CustomClipper<Path> {
  final int petals;
  final double depth;

  const FreeshapeFlowerClipper({this.petals = 8, this.depth = 0.14});

  @override
  Path getClip(Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = size.width / 2;
    final path = Path();
    const steps = 180;
    for (int i = 0; i <= steps; i++) {
      final theta = (i / steps) * 2 * math.pi;
      final mod = (1.0 - depth) + depth * math.cos(petals * theta);
      final r = radius * mod;
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
  bool shouldReclip(covariant FreeshapeFlowerClipper oldClipper) =>
      oldClipper.petals != petals || oldClipper.depth != depth;
}

class FreeshapeCloverClipper extends CustomClipper<Path> {
  const FreeshapeCloverClipper();

  @override
  Path getClip(Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = size.width / 2;
    final path = Path();
    const steps = 180;
    for (int i = 0; i <= steps; i++) {
      final theta = (i / steps) * 2 * math.pi;
      final mod = 0.82 + 0.18 * math.cos(4 * theta);
      final r = radius * mod;
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
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}

class FreeshapeStarClipper extends CustomClipper<Path> {
  final int points;
  const FreeshapeStarClipper({this.points = 6});

  @override
  Path getClip(Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = size.width / 2;
    final path = Path();
    const steps = 180;
    for (int i = 0; i <= steps; i++) {
      final theta = (i / steps) * 2 * math.pi;
      final mod = 0.80 + 0.20 * math.cos(points * theta);
      final r = radius * mod;
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
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}

class FreeshapeOrganicBlobClipper extends CustomClipper<Path> {
  const FreeshapeOrganicBlobClipper();

  @override
  Path getClip(Size size) {
    final w = size.width;
    final h = size.height;
    final path = Path();
    path.moveTo(w * 0.42, 0);
    path.cubicTo(w * 0.78, 0, w, h * 0.26, w, h * 0.56);
    path.cubicTo(w, h * 0.86, w * 0.72, h, w * 0.44, h);
    path.cubicTo(w * 0.16, h, 0, h * 0.76, 0, h * 0.44);
    path.cubicTo(0, h * 0.16, w * 0.16, 0, w * 0.42, 0);
    path.close();
    return path;
  }

  @override
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}

class FreeshapeSquircleClipper extends CustomClipper<Path> {
  const FreeshapeSquircleClipper();

  @override
  Path getClip(Size size) {
    final r = RRect.fromRectAndRadius(
      Rect.fromLTWH(0, 0, size.width, size.height),
      Radius.circular(size.width * 0.28),
    );
    final path = Path()..addRRect(r);
    return path;
  }

  @override
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}
