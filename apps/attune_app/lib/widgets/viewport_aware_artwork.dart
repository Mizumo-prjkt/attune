import 'dart:io';
import 'package:flutter/material.dart';

/// ViewportAwareArtwork: Viewport-aware image loader and memory unloader for album artwork.
///
/// Features:
/// - When inside the user's viewport (plus pre-load margin), resolves and decodes artwork.
/// - When scrolled OUT of the viewport or unmounted/disposed, immediately unloads the image
///   and evicts its key from [PaintingBinding.instance.imageCache], releasing GPU & RAM raster memory.
/// - Supports [enabled] = true (default) for lists/grids, and [enabled] = false for persistent
///   contexts like Bubblegrid ([BubbleNavigationStage]) where 2D panning requires persistent textures.
/// - Downsamples images via [cacheWidth] and [cacheHeight] to prevent decoding full multi-megabyte
///   original scans into uncompressed raw 32-bit RGBA buffers.
class ViewportAwareArtwork extends StatefulWidget {
  final String? artworkFile;
  final String? artworkAsset;
  final String? artworkUrl;
  final double? width;
  final double? height;
  final BoxFit fit;
  final BorderRadius? borderRadius;
  final Widget? placeholder;
  final int? cacheWidth;
  final int? cacheHeight;
  final bool enabled;

  const ViewportAwareArtwork({
    super.key,
    this.artworkFile,
    this.artworkAsset,
    this.artworkUrl,
    this.width,
    this.height,
    this.fit = BoxFit.cover,
    this.borderRadius,
    this.placeholder,
    this.cacheWidth,
    this.cacheHeight,
    this.enabled = true,
  });

  @override
  State<ViewportAwareArtwork> createState() => _ViewportAwareArtworkState();
}

class _ViewportAwareArtworkState extends State<ViewportAwareArtwork> {
  bool _isInView = false;
  ImageProvider? _imageProvider;
  ScrollPosition? _scrollPosition;
  bool _postFrameScheduled = false;

  bool get hasArtwork =>
      (widget.artworkFile != null && widget.artworkFile!.isNotEmpty) ||
      (widget.artworkUrl != null && widget.artworkUrl!.isNotEmpty) ||
      (widget.artworkAsset != null && widget.artworkAsset!.isNotEmpty);

  @override
  void initState() {
    super.initState();
    if (!widget.enabled) {
      _isInView = true;
      _loadImage();
    } else {
      _scheduleVisibilityCheck();
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (widget.enabled) {
      final scrollable = Scrollable.maybeOf(context);
      final newPos = scrollable?.position;
      if (_scrollPosition != newPos) {
        _scrollPosition?.removeListener(_onScrollChanged);
        _scrollPosition = newPos;
        _scrollPosition?.addListener(_onScrollChanged);
      }
      _scheduleVisibilityCheck();
    }
  }

  @override
  void didUpdateWidget(ViewportAwareArtwork oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.artworkFile != widget.artworkFile ||
        oldWidget.artworkUrl != widget.artworkUrl ||
        oldWidget.artworkAsset != widget.artworkAsset ||
        oldWidget.enabled != widget.enabled) {
      _unloadImage();
      if (!widget.enabled) {
        _isInView = true;
        _loadImage();
      } else {
        _scheduleVisibilityCheck();
      }
    }
  }

  void _onScrollChanged() {
    if (!mounted || !widget.enabled) return;
    _checkVisibility();
  }

  void _scheduleVisibilityCheck() {
    if (_postFrameScheduled) return;
    _postFrameScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _postFrameScheduled = false;
      if (mounted) {
        _checkVisibility();
      }
    });
  }

  void _checkVisibility() {
    if (!mounted) return;
    if (!widget.enabled) {
      if (!_isInView) {
        setState(() => _isInView = true);
        _loadImage();
      }
      return;
    }

    final renderBox = context.findRenderObject();
    if (renderBox is! RenderBox || !renderBox.hasSize || !renderBox.attached) {
      // If not yet laid out, default to loading if not in scrollable or in tests
      if (_scrollPosition == null && !_isInView) {
        setState(() => _isInView = true);
        _loadImage();
      }
      return;
    }

    try {
      final offset = renderBox.localToGlobal(Offset.zero);
      final size = renderBox.size;
      final mediaQuery = MediaQuery.maybeOf(context);
      final screenSize = mediaQuery?.size ?? const Size(1920, 1080);

      // 80px pre-load buffer around screen viewport
      final viewport = Rect.fromLTWH(
        -80,
        -80,
        screenSize.width + 160,
        screenSize.height + 160,
      );

      final itemRect = offset & size;
      final inView = itemRect.overlaps(viewport);

      if (inView != _isInView) {
        setState(() {
          _isInView = inView;
        });
        if (inView) {
          _loadImage();
        } else {
          _unloadImage();
        }
      }
    } catch (_) {
      if (!_isInView) {
        setState(() => _isInView = true);
        _loadImage();
      }
    }
  }

  void _loadImage() {
    if (!hasArtwork) return;
    if (_imageProvider != null) return;

    ImageProvider? provider;
    if (widget.artworkFile != null && widget.artworkFile!.isNotEmpty) {
      final file = File(widget.artworkFile!);
      if (file.existsSync()) {
        provider = FileImage(file);
      }
    } else if (widget.artworkUrl != null && widget.artworkUrl!.isNotEmpty) {
      provider = NetworkImage(widget.artworkUrl!);
    } else if (widget.artworkAsset != null && widget.artworkAsset!.isNotEmpty) {
      provider = AssetImage(widget.artworkAsset!);
    }

    if (provider != null) {
      if (widget.cacheWidth != null || widget.cacheHeight != null) {
        provider = ResizeImage(
          provider,
          width: widget.cacheWidth,
          height: widget.cacheHeight,
        );
      }
      _imageProvider = provider;
    }
  }

  void _unloadImage() {
    if (_imageProvider != null) {
      try {
        _imageProvider!.evict();
      } catch (_) {}
      _imageProvider = null;
    }
  }

  @override
  void dispose() {
    _scrollPosition?.removeListener(_onScrollChanged);
    _scrollPosition = null;
    _unloadImage();
    super.dispose();
  }

  Widget _buildDefaultPlaceholder(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      width: widget.width,
      height: widget.height,
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.25),
      child: Center(
        child: Icon(
          Icons.music_note_rounded,
          size: (widget.width != null && widget.width! > 0)
              ? (widget.width! * 0.45).clamp(16.0, 36.0)
              : 20.0,
          color: colorScheme.onSurface.withValues(alpha: 0.35),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final placeholderWidget = widget.placeholder ?? _buildDefaultPlaceholder(context);

    if (!_isInView || _imageProvider == null) {
      return SizedBox(
        width: widget.width,
        height: widget.height,
        child: ClipRRect(
          borderRadius: widget.borderRadius ?? BorderRadius.zero,
          child: placeholderWidget,
        ),
      );
    }

    return SizedBox(
      width: widget.width,
      height: widget.height,
      child: ClipRRect(
        borderRadius: widget.borderRadius ?? BorderRadius.zero,
        child: Image(
          image: _imageProvider!,
          fit: widget.fit,
          width: widget.width,
          height: widget.height,
          gaplessPlayback: true,
          errorBuilder: (context, error, stackTrace) => placeholderWidget,
          frameBuilder: (context, child, frame, wasSynchronouslyLoaded) {
            if (wasSynchronouslyLoaded || frame != null) {
              return child;
            }
            return placeholderWidget;
          },
        ),
      ),
    );
  }
}
