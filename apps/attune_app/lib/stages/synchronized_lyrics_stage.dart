import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import '../models/track_item.dart';
import '../services/lyrics_service.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/audio_quality_badge.dart';

/// SynchronizedLyricsStage implementing Phase 4 UI Specification:
/// High-legibility scrolling synchronized lyrics powered by real embedded metadata
/// (FLAC Vorbis, MP3 ID3) and companion .lrc files from local storage.
/// Zero hardcoded lyrics. Supports both full-page mobile mode and desktop docked side-pane.
class SynchronizedLyricsStage extends StatefulWidget {
  final bool isFullPage;
  final VoidCallback? onBackToArtwork;

  const SynchronizedLyricsStage({
    super.key,
    this.isFullPage = false,
    this.onBackToArtwork,
  });

  @override
  State<SynchronizedLyricsStage> createState() => _SynchronizedLyricsStageState();
}

class _SynchronizedLyricsStageState extends State<SynchronizedLyricsStage> {
  final ScrollController _scrollController = ScrollController();
  final Map<int, GlobalKey> _lineKeys = {};
  bool _isLockedToActive = true;
  int _selectedSourceIndex = 0; // 0: Embedded, 1: .LRC File, 2: Ultra-Sync
  int _lastActiveIndex = -1;

  String? _loadedTrackId;
  LyricSourceType? _loadedSource;
  LyricLoadResult? _lyricResult;
  bool _isLoading = false;

  @override
  void initState() {
    super.initState();
    _checkAndLoadLyrics();
  }

  LyricSourceType get _currentSource {
    switch (_selectedSourceIndex) {
      case 0:
        return LyricSourceType.embedded;
      case 1:
        return LyricSourceType.lrcFile;
      case 2:
      default:
        return LyricSourceType.ultraSync;
    }
  }

  void _checkAndLoadLyrics() {
    final track = PlaybackController.instance.currentTrack;
    if (track == null) return;

    final source = _currentSource;
    if (_loadedTrackId == track.id && _loadedSource == source && _lyricResult != null) {
      return;
    }

    _loadLyricsForTrack(track, source);
  }

  Future<void> _loadLyricsForTrack(TrackItem track, LyricSourceType source) async {
    setState(() {
      _isLoading = true;
      _loadedTrackId = track.id;
      _loadedSource = source;
      _lineKeys.clear();
      _isLockedToActive = true;
      _lastActiveIndex = -1;
    });

    final result = await LyricsService.instance.loadLyrics(track, source);
    if (!mounted) return;

    setState(() {
      _lyricResult = result;
      _isLoading = false;
      _lastActiveIndex = -1;
    });
  }

  int _computeActiveIndex(List<LyricLine> lines, double currentPosSec) {
    if (lines.isEmpty) return 0;
    int active = 0;
    for (int i = 0; i < lines.length; i++) {
      if (currentPosSec >= lines[i].timeSec) {
        active = i;
      } else {
        break;
      }
    }
    return active;
  }

  void _scrollToActive(int index, {bool force = false}) {
    if (!_isLockedToActive && !force) return;
    if (!_scrollController.hasClients) return;
    if (index == _lastActiveIndex && !force) return;
    _lastActiveIndex = index;

    final key = _lineKeys[index];
    final targetContext = key?.currentContext;
    if (targetContext != null) {
      Scrollable.ensureVisible(
        targetContext,
        alignment: 0.5, // Precisely center the active singing line in the viewport
        duration: const Duration(milliseconds: 380),
        curve: Curves.easeOutCubic,
      );
    } else {
      // Graceful fallback if far out of immediate viewport
      final lines = _lyricResult?.lines ?? const [];
      if (lines.isNotEmpty && _scrollController.position.hasContentDimensions) {
        final ratio = (index / lines.length).clamp(0.0, 1.0);
        final targetOffset = (ratio * _scrollController.position.maxScrollExtent);
        _scrollController.animateTo(
          targetOffset,
          duration: const Duration(milliseconds: 380),
          curve: Curves.easeOutCubic,
        );
      }
    }
  }

  void _handleUserScroll() {
    if (_isLockedToActive) {
      setState(() {
        _isLockedToActive = false;
      });
    }
  }

  Widget _buildSyncToCurrentButton(int activeIndex, ColorScheme colorScheme) {
    return Center(
      child: GestureDetector(
        onTap: () {
          setState(() {
            _isLockedToActive = true;
          });
          _scrollToActive(activeIndex, force: true);
        },
        behavior: HitTestBehavior.opaque,
        child: LiquidGlassContainer(
          padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 10),
          borderRadius: 24,
          surfaceAlpha: 0.32,
          tintOverride: colorScheme.primary.withValues(alpha: 0.30),
          boxShadow: [
            BoxShadow(
              color: Colors.black.withValues(alpha: 0.40),
              blurRadius: 18,
              offset: const Offset(0, 4),
            ),
            BoxShadow(
              color: colorScheme.primary.withValues(alpha: 0.28),
              blurRadius: 12,
              spreadRadius: 1,
            ),
          ],
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.vertical_align_center_rounded, size: 16, color: colorScheme.primary),
              const SizedBox(width: 8),
              Text(
                'Sync to Current Line',
                style: AttuneTypography.soraBadge(
                  color: Colors.white,
                  fontSize: 12,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListenableBuilder(
      listenable: PlaybackController.instance,
      builder: (context, _) {
        final controller = PlaybackController.instance;
        final track = controller.currentTrack;

        // Auto-refresh when active track changes
        if (track != null && (_loadedTrackId != track.id || _loadedSource != _currentSource)) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (mounted) _checkAndLoadLyrics();
          });
        }

        final lines = _lyricResult?.lines ?? const [];
        final hasLyrics = _lyricResult?.hasLyrics ?? false;
        final activeIndex = _computeActiveIndex(lines, controller.positionSec);

        if (lines.isNotEmpty) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            _scrollToActive(activeIndex);
          });
        }

        if (widget.isFullPage) {
          return _buildFullPageLayout(
            context,
            controller,
            track,
            lines,
            hasLyrics,
            activeIndex,
            colorScheme,
          );
        } else {
          return _buildDockedList(
            context,
            controller,
            track,
            lines,
            hasLyrics,
            activeIndex,
            colorScheme,
          );
        }
      },
    );
  }

  // ---------------------------------------------------------------------------
  // FULL PAGE VIEW (Mobile Large Lyrics Layout)
  // ---------------------------------------------------------------------------
  Widget _buildFullPageLayout(
    BuildContext context,
    PlaybackController controller,
    TrackItem? track,
    List<LyricLine> lines,
    bool hasLyrics,
    int activeIndex,
    ColorScheme colorScheme,
  ) {
    final posSec = controller.positionSec;
    final durSec = controller.durationSec > 0 ? controller.durationSec : 180.0;
    final currentMin = (posSec / 60).floor();
    final currentSec = (posSec % 60).floor();
    final totalMin = (durSec / 60).floor();
    final totalSec = (durSec % 60).floor();

    return SafeArea(
      child: Stack(
        fit: StackFit.expand,
        children: [
          // Background living frosted glass refraction
          const AmbientMeshBackground(),

          Column(
            children: [
              // 1. Top Header: Back/Dismiss button + Track Title + Quality Badge
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 10.0),
                child: Row(
                  children: [
                    IconButton(
                      icon: const Icon(Icons.keyboard_arrow_down_rounded, size: 30),
                      color: colorScheme.onSurface,
                      tooltip: 'Back to player',
                      onPressed: widget.onBackToArtwork,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            track?.title ?? 'Now Playing',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: AttuneTypography.soraHeading(
                              fontSize: 18,
                              fontWeight: FontWeight.bold,
                              color: colorScheme.onSurface,
                            ),
                          ),
                          Text(
                            track?.artist ?? 'Attune Engine',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: AttuneTypography.jakartaBodySmall(
                              color: colorScheme.onSurface.withValues(alpha: 0.65),
                            ),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                    if (track != null)
                      AudioQualityBadge(
                        format: track.format,
                        sampleRate: track.sampleRate,
                        bitDepth: track.bitDepth,
                        isProprietary: track.isProprietary,
                      ),
                  ],
                ),
              ),

              const Divider(color: Colors.white10, height: 1),

              // 2. Center: Large Scrolling Lyrics with Ambient Refraction OR Empty State
              Expanded(
                child: _isLoading
                    ? _buildLoadingState(colorScheme)
                    : hasLyrics
                        ? _buildLyricsList(lines, activeIndex, controller, colorScheme)
                        : _buildEmptyState(track, colorScheme),
              ),

              // 3. Bottom: Source Switcher Segment + Playback Transport Controls
              LiquidGlassContainer(
                margin: const EdgeInsets.all(16),
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
                borderRadius: 28,
                surfaceAlpha: 0.24,
                blurSigma: 40,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    // Lyric Source Switcher Tab
                    _buildSourceSwitcherTab(colorScheme),

                    const SizedBox(height: 12),

                    // Progress Bar & Tabular Timestamps
                    SliderTheme(
                      data: SliderTheme.of(context).copyWith(
                        trackHeight: 4,
                        thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                        activeTrackColor: colorScheme.primary,
                        inactiveTrackColor: Colors.white.withValues(alpha: 0.15),
                        thumbColor: Colors.white,
                        overlayShape: const RoundSliderOverlayShape(overlayRadius: 14),
                      ),
                      child: Slider(
                        value: posSec.clamp(0.0, durSec),
                        min: 0.0,
                        max: durSec,
                        onChanged: (val) => controller.seek(val),
                      ),
                    ),

                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 8.0),
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Text(
                            '${currentMin.toString().padLeft(2, '0')}:${currentSec.toString().padLeft(2, '0')}',
                            style: AttuneTypography.soraTabularTimestamp(
                              fontSize: 11,
                              color: colorScheme.onSurface.withValues(alpha: 0.6),
                            ),
                          ),
                          Text(
                            '${totalMin.toString().padLeft(2, '0')}:${totalSec.toString().padLeft(2, '0')}',
                            style: AttuneTypography.soraTabularTimestamp(
                              fontSize: 11,
                              color: colorScheme.onSurface.withValues(alpha: 0.6),
                            ),
                          ),
                        ],
                      ),
                    ),

                    const SizedBox(height: 8),

                    // Compact Transport Controls
                    Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        IconButton(
                          icon: const Icon(Icons.skip_previous_rounded),
                          iconSize: 30,
                          color: colorScheme.onSurface,
                          onPressed: controller.skipPrevious,
                        ),
                        const SizedBox(width: 16),
                        Container(
                          width: 52,
                          height: 52,
                          decoration: BoxDecoration(
                            shape: BoxShape.circle,
                            color: colorScheme.primary.withValues(alpha: 0.32),
                            border: Border.all(
                              color: colorScheme.primary.withValues(alpha: 0.8),
                              width: 1.5,
                            ),
                            boxShadow: [
                              BoxShadow(
                                color: colorScheme.primary.withValues(alpha: 0.4),
                                blurRadius: 18,
                                spreadRadius: 1,
                              ),
                            ],
                          ),
                          child: IconButton(
                            icon: Icon(
                              controller.isPlaying ? Icons.pause_rounded : Icons.play_arrow_rounded,
                            ),
                            iconSize: 32,
                            color: Colors.white,
                            onPressed: controller.togglePlay,
                          ),
                        ),
                        const SizedBox(width: 16),
                        IconButton(
                          icon: const Icon(Icons.skip_next_rounded),
                          iconSize: 30,
                          color: colorScheme.onSurface,
                          onPressed: controller.skipNext,
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildLyricsList(
    List<LyricLine> lines,
    int activeIndex,
    PlaybackController controller,
    ColorScheme colorScheme,
  ) {
    return Stack(
      children: [
        NotificationListener<ScrollNotification>(
          onNotification: (notification) {
            if (notification is UserScrollNotification && notification.direction != ScrollDirection.idle) {
              _handleUserScroll();
            } else if (notification is ScrollStartNotification && notification.dragDetails != null) {
              _handleUserScroll();
            }
            return false;
          },
          child: ListView.builder(
            controller: _scrollController,
            physics: const BouncingScrollPhysics(),
            scrollCacheExtent: const ScrollCacheExtent.pixels(2000.0),
            padding: const EdgeInsets.symmetric(horizontal: 28.0, vertical: 36.0),
            itemCount: lines.length,
            itemBuilder: (context, index) {
              final line = lines[index];
              final isActive = index == activeIndex;
              final key = _lineKeys.putIfAbsent(index, () => GlobalKey(debugLabel: 'full_lyric_$index'));

              if (line.isInstrumental) {
                return Center(
                  key: key,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(vertical: 24.0),
                    child: LiquidGlassContainer(
                      padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 8),
                      borderRadius: 16,
                      surfaceAlpha: isActive ? 0.24 : 0.10,
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(Icons.waves_rounded, size: 16, color: colorScheme.primary),
                          const SizedBox(width: 8),
                          Text(
                            line.text,
                            style: AttuneTypography.soraBadge(
                              color: colorScheme.primary,
                              fontSize: 11,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                );
              }

              return GestureDetector(
                key: key,
                onTap: () {
                  controller.seek(line.timeSec);
                  setState(() => _isLockedToActive = true);
                  _scrollToActive(index, force: true);
                },
                behavior: HitTestBehavior.opaque,
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 14.0),
                  child: isActive
                      ? LiquidGlassContainer(
                          padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 16),
                          borderRadius: 22,
                          surfaceAlpha: 0.28,
                          tintOverride: colorScheme.primary.withValues(alpha: 0.26),
                          child: Text(
                            line.text,
                            style: AttuneTypography.soraHeading(
                              fontSize: 22,
                              fontWeight: FontWeight.bold,
                              color: Colors.white,
                            ),
                          ),
                        )
                      : Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 14.0),
                          child: Text(
                            line.text,
                            style: AttuneTypography.jakartaLyricsInactive(
                              color: colorScheme.onSurface.withValues(alpha: 0.40),
                            ).copyWith(fontSize: 18),
                          ),
                        ),
                ),
              );
            },
          ),
        ),

        // Floating Liquid Glass Sync-to-Current-Line button when user is in free-scroll mode
        if (!_isLockedToActive && lines.isNotEmpty)
          Positioned(
            bottom: 16,
            left: 0,
            right: 0,
            child: _buildSyncToCurrentButton(activeIndex, colorScheme),
          ),
      ],
    );
  }

  // ---------------------------------------------------------------------------
  // DOCKED LIST VIEW (Tablet / Desktop Side-Pane)
  // ---------------------------------------------------------------------------
  Widget _buildDockedList(
    BuildContext context,
    PlaybackController controller,
    TrackItem? track,
    List<LyricLine> lines,
    bool hasLyrics,
    int activeIndex,
    ColorScheme colorScheme,
  ) {
    return Column(
      children: [
        // Top Source Switcher for Side Pane
        Padding(
          padding: const EdgeInsets.only(bottom: 12.0),
          child: _buildSourceSwitcherTab(colorScheme),
        ),

        // Lyrics List OR Empty State
        Expanded(
          child: _isLoading
              ? _buildLoadingState(colorScheme)
              : hasLyrics
                  ? Stack(
                      children: [
                        NotificationListener<ScrollNotification>(
                          onNotification: (notification) {
                            if (notification is UserScrollNotification && notification.direction != ScrollDirection.idle) {
                              _handleUserScroll();
                            } else if (notification is ScrollStartNotification && notification.dragDetails != null) {
                              _handleUserScroll();
                            }
                            return false;
                          },
                          child: ListView.builder(
                            controller: _scrollController,
                            physics: const BouncingScrollPhysics(),
                            scrollCacheExtent: const ScrollCacheExtent.pixels(2000.0),
                            padding: const EdgeInsets.symmetric(horizontal: 12.0, vertical: 16.0),
                            itemCount: lines.length,
                            itemBuilder: (context, index) {
                              final line = lines[index];
                              final isActive = index == activeIndex;
                              final key = _lineKeys.putIfAbsent(index, () => GlobalKey(debugLabel: 'dock_lyric_$index'));

                              if (line.isInstrumental) {
                                return Center(
                                  key: key,
                                  child: Padding(
                                    padding: const EdgeInsets.symmetric(vertical: 16.0),
                                    child: LiquidGlassContainer(
                                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
                                      borderRadius: 14,
                                      surfaceAlpha: 0.12,
                                      child: Row(
                                        mainAxisSize: MainAxisSize.min,
                                        children: [
                                          Icon(Icons.waves_rounded, size: 14, color: colorScheme.secondary),
                                          const SizedBox(width: 6),
                                          Text(
                                            line.text,
                                            style: AttuneTypography.soraBadge(color: colorScheme.secondary, fontSize: 10),
                                          ),
                                        ],
                                      ),
                                    ),
                                  ),
                                );
                              }

                              return GestureDetector(
                                key: key,
                                onTap: () {
                                  controller.seek(line.timeSec);
                                  setState(() => _isLockedToActive = true);
                                  _scrollToActive(index, force: true);
                                },
                                behavior: HitTestBehavior.opaque,
                                child: Padding(
                                  padding: const EdgeInsets.symmetric(vertical: 10.0),
                                  child: isActive
                                      ? LiquidGlassContainer(
                                          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                                          borderRadius: 16,
                                          surfaceAlpha: 0.22,
                                          tintOverride: colorScheme.primary.withValues(alpha: 0.20),
                                          child: Text(
                                            line.text,
                                            style: AttuneTypography.jakartaLyricsActive(color: Colors.white).copyWith(fontSize: 16),
                                          ),
                                        )
                                      : Padding(
                                          padding: const EdgeInsets.symmetric(horizontal: 10.0),
                                          child: Text(
                                            line.text,
                                            style: AttuneTypography.jakartaLyricsInactive(
                                              color: colorScheme.onSurface.withValues(alpha: 0.40),
                                            ).copyWith(fontSize: 14),
                                          ),
                                        ),
                                ),
                              );
                            },
                          ),
                        ),

                        // Floating Liquid Glass Sync-to-Current-Line button when user is in free-scroll mode
                        if (!_isLockedToActive && lines.isNotEmpty)
                          Positioned(
                            bottom: 12,
                            left: 0,
                            right: 0,
                            child: _buildSyncToCurrentButton(activeIndex, colorScheme),
                          ),
                      ],
                    )
                  : _buildEmptyState(track, colorScheme),
        ),
      ],
    );
  }

  // ---------------------------------------------------------------------------
  // EMPTY & LOADING STATES (Real Source Awareness)
  // ---------------------------------------------------------------------------
  Widget _buildLoadingState(ColorScheme colorScheme) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            width: 32,
            height: 32,
            child: CircularProgressIndicator(
              strokeWidth: 2.5,
              color: colorScheme.primary,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            'Reading real lyrics source...',
            style: AttuneTypography.jakartaBodySmall(
              color: colorScheme.onSurface.withValues(alpha: 0.6),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildEmptyState(TrackItem? track, ColorScheme colorScheme) {
    String sourceName;
    String detailHint;

    switch (_currentSource) {
      case LyricSourceType.embedded:
        sourceName = 'Embedded Tags';
        detailHint = 'This audio file contains no Vorbis or ID3 lyric frames.';
        break;
      case LyricSourceType.lrcFile:
        sourceName = '.LRC Companion File';
        detailHint = 'No matching .lrc file found alongside "${track?.title ?? 'this track'}".';
        break;
      case LyricSourceType.ultraSync:
        sourceName = 'Ultra-Sync Neural Voice';
        detailHint = 'Multi-stem phoneme-aligned lyrics engine in active development.';
        break;
    }

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32.0),
        child: LiquidGlassContainer(
          padding: const EdgeInsets.all(24),
          borderRadius: 24,
          surfaceAlpha: 0.14,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.lyrics_outlined,
                size: 48,
                color: colorScheme.primary.withValues(alpha: 0.7),
              ),
              const SizedBox(height: 16),
              Text(
                'No $sourceName Lyrics Found',
                style: AttuneTypography.soraHeading(
                  fontSize: 16,
                  fontWeight: FontWeight.bold,
                  color: colorScheme.onSurface,
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 8),
              Text(
                detailHint,
                style: AttuneTypography.jakartaBodySmall(
                  color: colorScheme.onSurface.withValues(alpha: 0.6),
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 20),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                alignment: WrapAlignment.center,
                children: [
                  if (_selectedSourceIndex != 1)
                    TextButton.icon(
                      icon: const Icon(Icons.text_snippet_outlined, size: 16),
                      label: const Text('Try .LRC File'),
                      style: TextButton.styleFrom(foregroundColor: colorScheme.primary),
                      onPressed: () {
                        setState(() => _selectedSourceIndex = 1);
                        _checkAndLoadLyrics();
                      },
                    ),
                  if (_selectedSourceIndex != 0)
                    TextButton.icon(
                      icon: const Icon(Icons.music_note_rounded, size: 16),
                      label: const Text('Try Embedded'),
                      style: TextButton.styleFrom(foregroundColor: colorScheme.primary),
                      onPressed: () {
                        setState(() => _selectedSourceIndex = 0);
                        _checkAndLoadLyrics();
                      },
                    ),
                  IconButton(
                    icon: const Icon(Icons.refresh_rounded, size: 20),
                    tooltip: 'Rescan',
                    onPressed: () {
                      LyricsService.instance.invalidateCache();
                      _checkAndLoadLyrics();
                    },
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // SOURCE SWITCHER SEGMENT
  // ---------------------------------------------------------------------------
  Widget _buildSourceSwitcherTab(ColorScheme colorScheme) {
    return LiquidGlassContainer(
      padding: const EdgeInsets.all(3),
      borderRadius: 16,
      surfaceAlpha: 0.16,
      child: Row(
        children: [
          Expanded(
            child: _buildSourceItem(0, 'Embedded', Icons.music_note_rounded, colorScheme),
          ),
          Expanded(
            child: _buildSourceItem(1, '.LRC File', Icons.text_snippet_outlined, colorScheme),
          ),
          Expanded(
            child: _buildSourceItem(2, 'Ultra-Sync (Soon)', Icons.auto_awesome_rounded, colorScheme),
          ),
        ],
      ),
    );
  }

  Widget _buildSourceItem(int index, String label, IconData icon, ColorScheme colorScheme) {
    final isSelected = _selectedSourceIndex == index;
    return GestureDetector(
      onTap: () {
        setState(() {
          _selectedSourceIndex = index;
          _lineKeys.clear();
          _isLockedToActive = true;
          _lastActiveIndex = -1;
        });
        _checkAndLoadLyrics();
      },
      behavior: HitTestBehavior.opaque,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 180),
        padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 4),
        decoration: BoxDecoration(
          color: isSelected ? colorScheme.primary.withValues(alpha: 0.26) : Colors.transparent,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              icon,
              size: 13,
              color: isSelected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.5),
            ),
            const SizedBox(width: 4),
            Flexible(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: AttuneTypography.soraBadge(
                  fontSize: 10,
                  color: isSelected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.55),
                  fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
