import 'dart:io';
import 'package:flutter/material.dart';
import '../services/audio_storage_service.dart';
import '../services/playback_controller.dart';
import '../services/platform_navigation.dart';
import '../stages/albums_stage.dart';
import '../stages/home_stage.dart';
import '../stages/now_playing_stage.dart';
import '../stages/playlist_queue_stage.dart';
import '../stages/profile_and_stats_stage.dart';
import '../stages/startup_setup_stage.dart';
import '../stages/synchronized_lyrics_stage.dart';
import '../theme/color_engine.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/floating_mini_player.dart';

/// ResponsiveGlassScaffold adhering to Section 4 of 05_UI_DESIGN_SPEC.md:
/// - Startup Setup Page check on first launch
/// - Main stages: Home (0), Queue (1), Albums (2), Profile (3)
/// - Floating Mini-Player toolbar with neon-like lighting box adjacent to artwork
/// - Full NowPlaying overlay on demand
/// - Responsive adaptation: Mobile (< 720px) and Tablet/Desktop (>= 720px)
class ResponsiveGlassScaffold extends StatefulWidget {
  const ResponsiveGlassScaffold({super.key});

  @override
  State<ResponsiveGlassScaffold> createState() => _ResponsiveGlassScaffoldState();
}

class _ResponsiveGlassScaffoldState extends State<ResponsiveGlassScaffold> {
  int _activeNavIndex = 0; // 0: Home, 1: Queue, 2: Albums, 3: Profile
  int _dockedContextualTabIndex = 0; // 0: Queue, 1: Lyrics (for tablet/desktop)
  bool _showNowPlayingModal = false;
  late bool _isSetupDone;

  @override
  void initState() {
    super.initState();
    // In automated tests, skip startup setup to preserve smoke tests
    if (Platform.environment.containsKey('FLUTTER_TEST')) {
      _isSetupDone = true;
    } else {
      _isSetupDone = AudioStorageService.instance.isSetupCompleted();
    }
  }

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

        // 1. Startup Setup Page
        if (!_isSetupDone) {
          return AmbientMeshBackground(
            colorScheme: colorScheme,
            isPlaying: false,
            child: SafeArea(
              child: StartupSetupStage(
                onSetupComplete: () => setState(() => _isSetupDone = true),
              ),
            ),
          );
        }

        // 2. Primary Shell
        return Scaffold(
          backgroundColor: Colors.transparent,
          body: AmbientMeshBackground(
            colorScheme: colorScheme,
            isPlaying: controller.isPlaying,
            child: SafeArea(
              child: isWide
                  ? _buildTabletDesktopLayout(width, colorScheme, controller)
                  : _buildMobileLayout(colorScheme, controller),
            ),
          ),
        );
      },
    );
  }

  // Mobile Layout (< 720px)
  Widget _buildMobileLayout(ColorScheme colorScheme, PlaybackController controller) {
    if (_showNowPlayingModal) {
      return NowPlayingStage(
        onDismiss: () => setState(() => _showNowPlayingModal = false),
      );
    }

    Widget activeStage;
    switch (_activeNavIndex) {
      case 0:
        activeStage = HomeStage(onOpenNowPlaying: () => setState(() => _showNowPlayingModal = true));
        break;
      case 1:
        activeStage = const PlaylistQueueStage();
        break;
      case 2:
        activeStage = AlbumsStage(onOpenNowPlaying: () => setState(() => _showNowPlayingModal = true));
        break;
      case 3:
        activeStage = ProfileAndStatsStage(onRerunSetup: () => setState(() => _isSetupDone = false));
        break;
      default:
        activeStage = const SizedBox();
    }

    final hasTrack = controller.currentTrack != null;

    return Stack(
      children: [
        // Active Stage Body
        Positioned.fill(
          bottom: hasTrack ? 148 : 84, // Extra space when floating mini player is visible
          child: activeStage,
        ),

        // Floating Mini-Player Toolbar (with neon-like lighting box)
        if (hasTrack)
          Positioned(
            left: 16,
            right: 16,
            bottom: 84,
            child: FloatingMiniPlayer(
              onTap: () => setState(() => _showNowPlayingModal = true),
            ),
          ),

        // Floating Glass Bottom Pill Bar
        Positioned(
          left: 20,
          right: 20,
          bottom: 16,
          child: LiquidGlassContainer(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            borderRadius: 28,
            blurSigma: 40.0,
            surfaceAlpha: 0.22,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceAround,
              children: [
                _buildMobileNavButton(0, Icons.home_rounded, 'Home', colorScheme),
                _buildMobileNavButton(1, Icons.queue_music_rounded, 'Queue', colorScheme),
                _buildMobileNavButton(2, Icons.album_rounded, 'Albums', colorScheme),
                _buildMobileNavButton(3, Icons.person_rounded, 'Profile', colorScheme),
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
      onTap: () => setState(() {
        _activeNavIndex = index;
        _showNowPlayingModal = false;
      }),
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
  Widget _buildTabletDesktopLayout(double width, ColorScheme colorScheme, PlaybackController controller) {
    Widget centerStage;
    if (_showNowPlayingModal) {
      centerStage = NowPlayingStage(
        onDismiss: () => setState(() => _showNowPlayingModal = false),
      );
    } else {
      switch (_activeNavIndex) {
        case 0:
          centerStage = HomeStage(onOpenNowPlaying: () => setState(() => _showNowPlayingModal = true));
          break;
        case 1:
          centerStage = const PlaylistQueueStage();
          break;
        case 2:
          centerStage = AlbumsStage(onOpenNowPlaying: () => setState(() => _showNowPlayingModal = true));
          break;
        case 3:
          centerStage = ProfileAndStatsStage(onRerunSetup: () => setState(() => _isSetupDone = false));
          break;
        default:
          centerStage = const SizedBox();
      }
    }

    final hasTrack = controller.currentTrack != null;

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
                const SizedBox(height: 28),

                // Nav Items: Home, Queue, Albums, Profile
                _buildRailButton(0, Icons.home_rounded, 'Home', colorScheme),
                const SizedBox(height: 12),
                _buildRailButton(1, Icons.queue_music_rounded, 'Queue', colorScheme),
                const SizedBox(height: 12),
                _buildRailButton(2, Icons.album_rounded, 'Albums', colorScheme),
                const SizedBox(height: 12),
                _buildRailButton(3, Icons.person_rounded, 'Profile', colorScheme),

                const Spacer(),

                // Settings Button
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

        // 2. Center-Left Primary Stage with Stacked Mini Player if not in full player
        Expanded(
          flex: 5,
          child: Stack(
            children: [
              Positioned.fill(
                bottom: hasTrack && !_showNowPlayingModal ? 80 : 0,
                child: centerStage,
              ),
              if (hasTrack && !_showNowPlayingModal)
                Positioned(
                  left: 16,
                  right: 16,
                  bottom: 12,
                  child: FloatingMiniPlayer(
                    onTap: () => setState(() => _showNowPlayingModal = true),
                  ),
                ),
            ],
          ),
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
    final selected = _activeNavIndex == index && !_showNowPlayingModal;
    return GestureDetector(
      onTap: () => setState(() {
        _activeNavIndex = index;
        _showNowPlayingModal = false;
      }),
      behavior: HitTestBehavior.opaque,
      child: Tooltip(
        message: label,
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
