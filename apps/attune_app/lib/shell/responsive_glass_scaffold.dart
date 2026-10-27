import 'package:flutter/material.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../stages/music_library_stage.dart';
import '../stages/now_playing_stage.dart';
import '../stages/playlist_queue_stage.dart';
import '../stages/synchronized_lyrics_stage.dart';
import '../theme/color_engine.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// ResponsiveGlassScaffold adhering to Section 4 of 05_UI_DESIGN_SPEC.md:
/// - Mobile Portrait (< 720px): Full-screen stage + Floating Glass Bottom Pill Bar + top-right settings trigger
/// - Tablet & Desktop (>= 720px): Shared Layout! Left Glass Nav Rail + Center Primary Stage + Right Docked Contextual Pane (Queue & Lyrics)
class ResponsiveGlassScaffold extends StatefulWidget {
  const ResponsiveGlassScaffold({super.key});

  @override
  State<ResponsiveGlassScaffold> createState() => _ResponsiveGlassScaffoldState();
}

class _ResponsiveGlassScaffoldState extends State<ResponsiveGlassScaffold> {
  int _activeNavIndex = 0; // 0: Playing, 1: Queue, 2: Lyrics, 3: Library
  int _dockedContextualTabIndex = 0; // 0: Queue, 1: Lyrics (for tablet/desktop)

  @override
  Widget build(BuildContext context) {
    final mediaQuery = MediaQuery.of(context);
    final width = mediaQuery.size.width;
    final isWide = width >= 720;

    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final currentTrack = controller.currentTrack;
        final nextTrack = controller.upNextTrack;

        final baseScheme = AttuneColorEngine.createSchemeFromSeed(
          currentTrack?.seedColor ?? AttuneColorEngine.fallbackPrimary,
        );
        final targetScheme = AttuneColorEngine.createSchemeFromSeed(
          nextTrack?.seedColor ?? AttuneColorEngine.fallbackSecondary,
        );
        final colorScheme = controller.isTransitioning
            ? AttuneColorEngine.lerpSchemes(baseScheme, targetScheme, controller.transitionProgress)
            : baseScheme;

        return Scaffold(
          backgroundColor: Colors.transparent,
          body: AmbientMeshBackground(
            colorScheme: colorScheme,
            isPlaying: controller.isPlaying,
            child: SafeArea(
              child: isWide
                  ? _buildTabletDesktopLayout(width, colorScheme)
                  : _buildMobileLayout(colorScheme),
            ),
          ),
        );
      },
    );
  }

  // Mobile Layout (< 720px)
  Widget _buildMobileLayout(ColorScheme colorScheme) {
    Widget activeStage;
    switch (_activeNavIndex) {
      case 0:
        activeStage = const NowPlayingStage();
        break;
      case 1:
        activeStage = const PlaylistQueueStage();
        break;
      case 2:
        activeStage = const SynchronizedLyricsStage();
        break;
      case 3:
        activeStage = const MusicLibraryStage();
        break;
      default:
        activeStage = const SizedBox();
    }

    return Stack(
      children: [
        // Active Stage Body
        Positioned.fill(
          bottom: 84, // Space for floating bottom pill bar
          child: activeStage,
        ),

        // Floating Glass Bottom Pill Bar
        Positioned(
          left: 20,
          right: 20,
          bottom: 16,
          child: LiquidGlassContainer(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            borderRadius: 28,
            blurSigma: 40.0,
            surfaceAlpha: 0.22,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceAround,
              children: [
                _buildMobileNavButton(0, Icons.play_circle_fill_rounded, 'Playing', colorScheme),
                _buildMobileNavButton(1, Icons.queue_music_rounded, 'Queue', colorScheme),
                _buildMobileNavButton(2, Icons.lyrics_rounded, 'Lyrics', colorScheme),
                _buildMobileNavButton(3, Icons.library_music_rounded, 'Library', colorScheme),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildMobileNavButton(int index, IconData icon, String label, ColorScheme colorScheme) {
    final selected = _activeNavIndex == index;
    return GestureDetector(
      onTap: () => setState(() => _activeNavIndex = index),
      behavior: HitTestBehavior.opaque,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        decoration: BoxDecoration(
          color: selected ? colorScheme.primary.withValues(alpha: 0.22) : Colors.transparent,
          borderRadius: BorderRadius.circular(16),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              icon,
              size: 20,
              color: selected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.6),
            ),
            if (selected) ...[
              const SizedBox(width: 6),
              Text(
                label,
                style: AttuneTypography.soraBadge(color: colorScheme.primary),
              ),
            ],
          ],
        ),
      ),
    );
  }

  // Tablet & Desktop Shared Layout (>= 720px)
  Widget _buildTabletDesktopLayout(double width, ColorScheme colorScheme) {
    return Row(
      children: [
        // 1. Left Floating Glass Navigation Rail
        Padding(
          padding: const EdgeInsets.all(16.0),
          child: LiquidGlassContainer(
            width: 80,
            padding: const EdgeInsets.symmetric(vertical: 24),
            borderRadius: 28,
            surfaceAlpha: 0.16,
            child: Column(
              children: [
                // App Logo Icon
                Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: colorScheme.primary.withValues(alpha: 0.25),
                    shape: BoxShape.circle,
                  ),
                  child: Icon(Icons.graphic_eq_rounded, color: colorScheme.primary, size: 24),
                ),
                const SizedBox(height: 32),

                // Nav Items
                _buildRailButton(0, Icons.music_note_rounded, 'Player', colorScheme),
                const SizedBox(height: 16),
                _buildRailButton(3, Icons.library_music_rounded, 'Library', colorScheme),
                const Spacer(),

                // Settings Button (triggers SettingsActivity on Android or Route on Desktop)
                LiquidGlassContainer(
                  padding: const EdgeInsets.all(10),
                  borderRadius: 16,
                  surfaceAlpha: 0.18,
                  onTap: () => PlatformNavigation.openSettings(context),
                  child: const Icon(Icons.settings_outlined, color: Colors.white, size: 22),
                ),
              ],
            ),
          ),
        ),

        // 2. Center-Left Primary Stage
        Expanded(
          flex: 5,
          child: _activeNavIndex == 3
              ? const MusicLibraryStage()
              : const NowPlayingStage(),
        ),

        // 3. Right Docked Contextual Glass Pane
        Expanded(
          flex: 4,
          child: Padding(
            padding: const EdgeInsets.all(16.0),
            child: LiquidGlassContainer(
              padding: const EdgeInsets.all(16),
              borderRadius: 32,
              surfaceAlpha: 0.14,
              child: Column(
                children: [
                  // Glass Segmented Tab Switcher
                  LiquidGlassContainer(
                    padding: const EdgeInsets.all(4),
                    borderRadius: 16,
                    surfaceAlpha: 0.18,
                    child: Row(
                      children: [
                        Expanded(
                          child: _buildContextTab('Up Next Queue', 0, colorScheme),
                        ),
                        Expanded(
                          child: _buildContextTab('Synced Lyrics', 1, colorScheme),
                        ),
                      ],
                    ),
                  ),

                  const SizedBox(height: 12),

                  // Tab Content
                  Expanded(
                    child: _dockedContextualTabIndex == 0
                        ? const PlaylistQueueStage()
                        : const SynchronizedLyricsStage(),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildRailButton(int index, IconData icon, String label, ColorScheme colorScheme) {
    final selected = _activeNavIndex == index;
    return GestureDetector(
      onTap: () => setState(() => _activeNavIndex = index),
      behavior: HitTestBehavior.opaque,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: selected ? colorScheme.primary.withValues(alpha: 0.24) : Colors.transparent,
          borderRadius: BorderRadius.circular(16),
        ),
        child: Icon(
          icon,
          size: 24,
          color: selected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.5),
        ),
      ),
    );
  }

  Widget _buildContextTab(String label, int index, ColorScheme colorScheme) {
    final selected = _dockedContextualTabIndex == index;
    return GestureDetector(
      onTap: () => setState(() => _dockedContextualTabIndex = index),
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 8),
        decoration: BoxDecoration(
          color: selected ? colorScheme.primary.withValues(alpha: 0.24) : Colors.transparent,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Center(
          child: Text(
            label,
            style: AttuneTypography.soraBadge(
              color: selected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.6),
            ),
          ),
        ),
      ),
    );
  }
}
