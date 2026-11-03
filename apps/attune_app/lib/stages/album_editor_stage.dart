import 'dart:io';
import 'package:flutter/material.dart';
import '../models/album_custom_metadata.dart';
import '../models/track_item.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

enum TrackSortOption {
  original,
  artistAsc,
  artistDesc,
  titleAsc,
  bpmDesc,
  durationDesc,
}

/// Dedicated full-screen stage for modifying album metadata (Title, Artist, Year,
/// Genre, Splash Art, Lore/Description, Contributing Artists, and Online Identifiers)
/// and managing group tracks (adding, removing, and sorting songs).
/// Replaces translucent dialogs with high-contrast, fully legible Liquid Material Glass UI.
class AlbumEditorStage extends StatefulWidget {
  final String initialAlbumName;
  final List<TrackItem> initialTracks;
  final AlbumCustomMetadata? initialMetadata;
  final void Function(String oldAlbum, String newAlbum, AlbumCustomMetadata meta, List<TrackItem> finalTracks)? onSaved;

  const AlbumEditorStage({
    super.key,
    required this.initialAlbumName,
    required this.initialTracks,
    this.initialMetadata,
    this.onSaved,
  });

  @override
  State<AlbumEditorStage> createState() => _AlbumEditorStageState();
}

class _AlbumEditorStageState extends State<AlbumEditorStage> {
  late TextEditingController _titleController;
  late TextEditingController _artistController;
  late TextEditingController _newArtistController;
  late List<String> _additionalArtists;

  late TextEditingController _yearController;
  late TextEditingController _genreController;
  late TextEditingController _splashController;
  late TextEditingController _descController;

  // Online query & catalog controllers
  late TextEditingController _recordLabelController;
  late TextEditingController _catalogNumberController;
  late TextEditingController _albumTypeController;
  late TextEditingController _releaseDateController;
  late TextEditingController _languageController;
  late TextEditingController _musicBrainzIdController;
  late TextEditingController _onlineQueryUrlController;

  late List<TrackItem> _albumTracks;
  late List<TrackItem> _originalAlbumTracks;
  TrackSortOption _currentSort = TrackSortOption.original;
  late Set<String> _removedTrackIds;
  late Set<String> _addedTrackIds;

  @override
  void initState() {
    super.initState();
    _albumTracks = List.from(widget.initialTracks);
    _originalAlbumTracks = List.from(widget.initialTracks);
    _removedTrackIds = {};
    _addedTrackIds = {};

    final meta = widget.initialMetadata ?? const AlbumCustomMetadata();
    final defaultArtist = widget.initialTracks.isNotEmpty ? widget.initialTracks.first.artist : 'Unknown Artist';
    final defaultGenre = widget.initialTracks.isNotEmpty && widget.initialTracks.first.genre.isNotEmpty
        ? widget.initialTracks.first.genre
        : '';

    _additionalArtists = List.from(meta.additionalArtists);
    _newArtistController = TextEditingController();

    _titleController = TextEditingController(text: widget.initialAlbumName);
    _artistController = TextEditingController(text: defaultArtist);
    _yearController = TextEditingController(text: meta.year ?? '');
    _genreController = TextEditingController(text: meta.genre ?? defaultGenre);
    _splashController = TextEditingController(text: meta.customSplashArt ?? '');
    _descController = TextEditingController(
      text: meta.description ?? _generateDynamicAlbumDescription(widget.initialAlbumName, _albumTracks),
    );

    // Online query & catalog metadata
    _recordLabelController = TextEditingController(text: meta.recordLabel ?? '');
    _catalogNumberController = TextEditingController(text: meta.catalogNumber ?? '');
    _albumTypeController = TextEditingController(text: meta.albumType ?? 'Studio Album');
    _releaseDateController = TextEditingController(text: meta.releaseDate ?? '');
    _languageController = TextEditingController(text: meta.language ?? '');
    _musicBrainzIdController = TextEditingController(text: meta.musicBrainzId ?? '');
    _onlineQueryUrlController = TextEditingController(text: meta.onlineQueryUrl ?? '');
  }

  @override
  void dispose() {
    _titleController.dispose();
    _artistController.dispose();
    _newArtistController.dispose();
    _yearController.dispose();
    _genreController.dispose();
    _splashController.dispose();
    _descController.dispose();

    _recordLabelController.dispose();
    _catalogNumberController.dispose();
    _albumTypeController.dispose();
    _releaseDateController.dispose();
    _languageController.dispose();
    _musicBrainzIdController.dispose();
    _onlineQueryUrlController.dispose();
    super.dispose();
  }

  void _addAdditionalArtist() {
    final text = _newArtistController.text.trim();
    if (text.isNotEmpty && !_additionalArtists.contains(text)) {
      setState(() {
        _additionalArtists.add(text);
        _newArtistController.clear();
      });
    }
  }

  void _applyTrackSort(TrackSortOption sort) {
    setState(() {
      _currentSort = sort;
      switch (sort) {
        case TrackSortOption.original:
          final originalFiltered = _originalAlbumTracks.where((t) => !_removedTrackIds.contains(t.id)).toList();
          final added = _addedTrackIds
              .map((id) => _albumTracks.firstWhere((t) => t.id == id, orElse: () => _originalAlbumTracks.first))
              .where((t) => !originalFiltered.contains(t));
          _albumTracks = [...originalFiltered, ...added];
          break;
        case TrackSortOption.artistAsc:
          _albumTracks.sort((a, b) => a.artist.toLowerCase().compareTo(b.artist.toLowerCase()));
          break;
        case TrackSortOption.artistDesc:
          _albumTracks.sort((a, b) => b.artist.toLowerCase().compareTo(a.artist.toLowerCase()));
          break;
        case TrackSortOption.titleAsc:
          _albumTracks.sort((a, b) => a.title.toLowerCase().compareTo(b.title.toLowerCase()));
          break;
        case TrackSortOption.bpmDesc:
          _albumTracks.sort((a, b) => b.bpm.compareTo(a.bpm));
          break;
        case TrackSortOption.durationDesc:
          _albumTracks.sort((a, b) => b.duration.compareTo(a.duration));
          break;
      }
    });
  }

  String _sortLabel(TrackSortOption sort) {
    switch (sort) {
      case TrackSortOption.artistAsc:
        return 'Artist (A-Z)';
      case TrackSortOption.artistDesc:
        return 'Artist (Z-A)';
      case TrackSortOption.titleAsc:
        return 'Title (A-Z)';
      case TrackSortOption.bpmDesc:
        return 'BPM (Fast)';
      case TrackSortOption.durationDesc:
        return 'Duration';
      case TrackSortOption.original:
        return 'Track Index';
    }
  }

  String _generateDynamicAlbumDescription(String album, List<TrackItem> tracks) {
    if (tracks.isEmpty) {
      return 'Collection with 0 tracks. Add songs from your library to build this album.';
    }
    final artist = tracks.first.artist.isNotEmpty ? tracks.first.artist : 'Various Artists';
    final losslessCount = tracks.where((t) => t.isLossless).length;
    final totalSec = tracks.fold<int>(0, (sum, t) => sum + t.duration.inSeconds);
    final min = totalSec ~/ 60;
    final sec = totalSec % 60;

    final bpms = tracks.where((t) => t.hasBpm).map((t) => t.bpm).toList();
    String tempoStr = '';
    if (bpms.isNotEmpty) {
      final minBpm = bpms.reduce((a, b) => a < b ? a : b).toStringAsFixed(0);
      final maxBpm = bpms.reduce((a, b) => a > b ? a : b).toStringAsFixed(0);
      tempoStr = minBpm == maxBpm ? ' at $minBpm BPM' : ' spanning $minBpm to $maxBpm BPM';
    }

    final keys = tracks.where((t) => t.musicalKey.isNotEmpty && t.musicalKey != 'Unknown').map((t) => t.musicalKey).toSet();
    String keyStr = keys.isNotEmpty ? '. Key signatures include ${keys.take(3).join(', ')}' : '';

    return 'Collection of ${tracks.length} tracks ($min min $sec sec) by $artist. '
        'Features $losslessCount lossless audio master tracks$tempoStr$keyStr.';
  }

  void _saveAlbum() {
    final newAlbumName = _titleController.text.trim().isNotEmpty ? _titleController.text.trim() : widget.initialAlbumName;
    final updatedMeta = AlbumCustomMetadata(
      year: _yearController.text.trim().isNotEmpty ? _yearController.text.trim() : null,
      genre: _genreController.text.trim().isNotEmpty ? _genreController.text.trim() : null,
      customSplashArt: _splashController.text.trim().isNotEmpty ? _splashController.text.trim() : null,
      description: _descController.text.trim().isNotEmpty ? _descController.text.trim() : null,
      additionalArtists: _additionalArtists,
      recordLabel: _recordLabelController.text.trim().isNotEmpty ? _recordLabelController.text.trim() : null,
      catalogNumber: _catalogNumberController.text.trim().isNotEmpty ? _catalogNumberController.text.trim() : null,
      albumType: _albumTypeController.text.trim().isNotEmpty ? _albumTypeController.text.trim() : null,
      releaseDate: _releaseDateController.text.trim().isNotEmpty ? _releaseDateController.text.trim() : null,
      language: _languageController.text.trim().isNotEmpty ? _languageController.text.trim() : null,
      musicBrainzId: _musicBrainzIdController.text.trim().isNotEmpty ? _musicBrainzIdController.text.trim() : null,
      onlineQueryUrl: _onlineQueryUrlController.text.trim().isNotEmpty ? _onlineQueryUrlController.text.trim() : null,
    );

    final controller = PlaybackController.instance;

    // 1. If album name changed, update all current album tracks
    if (newAlbumName != widget.initialAlbumName) {
      for (final t in _albumTracks) {
        controller.updateTrackMetadata(
          trackId: t.id,
          album: newAlbumName,
          artist: _artistController.text.trim().isNotEmpty ? _artistController.text.trim() : null,
          genre: updatedMeta.genre,
        );
      }
    }

    // 2. Handle added tracks
    for (final trackId in _addedTrackIds) {
      controller.updateTrackMetadata(
        trackId: trackId,
        album: newAlbumName,
        artist: _artistController.text.trim().isNotEmpty ? _artistController.text.trim() : null,
        genre: updatedMeta.genre,
      );
    }

    // 3. Handle removed tracks (unlink album)
    for (final trackId in _removedTrackIds) {
      controller.updateTrackMetadata(
        trackId: trackId,
        album: 'Singles',
      );
    }

    widget.onSaved?.call(widget.initialAlbumName, newAlbumName, updatedMeta, _albumTracks);

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('Album "$newAlbumName" updated successfully'),
        behavior: SnackBarBehavior.floating,
        duration: const Duration(seconds: 2),
      ),
    );

    Navigator.of(context).pop(true);
  }

  void _showAddTracksModal(BuildContext context, ColorScheme colorScheme) {
    final controller = PlaybackController.instance;
    final currentIds = _albumTracks.map((t) => t.id).toSet();
    final availableTracks = controller.libraryTracks.where((t) => !currentIds.contains(t.id)).toList();

    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (context) {
        String query = '';
        return StatefulBuilder(
          builder: (context, setSheetState) {
            final filtered = availableTracks.where((t) {
              if (query.isEmpty) return true;
              final q = query.toLowerCase();
              return t.title.toLowerCase().contains(q) || t.artist.toLowerCase().contains(q);
            }).toList();

            return Container(
              height: MediaQuery.sizeOf(context).height * 0.75,
              decoration: BoxDecoration(
                color: const Color(0xFF141924),
                borderRadius: const BorderRadius.vertical(top: Radius.circular(28)),
                border: Border.all(color: colorScheme.onSurface.withValues(alpha: 0.12)),
              ),
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        'Add Songs to Album',
                        style: AttuneTypography.soraHeading(
                          fontSize: 18,
                          fontWeight: FontWeight.bold,
                          color: colorScheme.onSurface,
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.close_rounded),
                        onPressed: () => Navigator.of(context).pop(),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    onChanged: (val) => setSheetState(() => query = val),
                    style: AttuneTypography.jakartaBodySmall(color: colorScheme.onSurface),
                    decoration: InputDecoration(
                      prefixIcon: Icon(Icons.search_rounded, color: colorScheme.primary, size: 20),
                      hintText: 'Search songs from library...',
                      hintStyle: AttuneTypography.jakartaBodySmall(color: colorScheme.onSurface.withValues(alpha: 0.45)),
                      filled: true,
                      fillColor: Colors.black.withValues(alpha: 0.25),
                      border: OutlineInputBorder(borderRadius: BorderRadius.circular(14), borderSide: BorderSide.none),
                      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                    ),
                  ),
                  const SizedBox(height: 14),
                  Expanded(
                    child: filtered.isEmpty
                        ? Center(
                            child: Text(
                              'No more tracks available to add.',
                              style: AttuneTypography.jakartaBodySmall(color: colorScheme.onSurface.withValues(alpha: 0.5)),
                            ),
                          )
                        : ListView.separated(
                            physics: const BouncingScrollPhysics(),
                            itemCount: filtered.length,
                            separatorBuilder: (context, index) => const SizedBox(height: 8),
                            itemBuilder: (context, idx) {
                              final track = filtered[idx];
                              return LiquidGlassContainer(
                                padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                                borderRadius: 14,
                                surfaceAlpha: 0.15,
                                child: Row(
                                  children: [
                                    ClipRRect(
                                      borderRadius: BorderRadius.circular(8),
                                      child: SizedBox(
                                        width: 40,
                                        height: 40,
                                        child: track.artworkFile != null && File(track.artworkFile!).existsSync()
                                            ? Image.file(File(track.artworkFile!), fit: BoxFit.cover)
                                            : Container(
                                                color: colorScheme.primary.withValues(alpha: 0.2),
                                                child: Icon(Icons.music_note_rounded, color: colorScheme.primary, size: 20),
                                              ),
                                      ),
                                    ),
                                    const SizedBox(width: 12),
                                    Expanded(
                                      child: Column(
                                        crossAxisAlignment: CrossAxisAlignment.start,
                                        children: [
                                          Text(
                                            track.title,
                                            style: AttuneTypography.soraSubheading(
                                              fontSize: 13,
                                              fontWeight: FontWeight.w600,
                                              color: colorScheme.onSurface,
                                            ),
                                            maxLines: 1,
                                            overflow: TextOverflow.ellipsis,
                                          ),
                                          Text(
                                            '${track.artist} • ${track.formattedDuration}',
                                            style: AttuneTypography.jakartaBodySmall(
                                              fontSize: 11,
                                              color: colorScheme.onSurface.withValues(alpha: 0.6),
                                            ),
                                            maxLines: 1,
                                            overflow: TextOverflow.ellipsis,
                                          ),
                                        ],
                                      ),
                                    ),
                                    IconButton(
                                      icon: const Icon(Icons.add_circle_rounded),
                                      color: colorScheme.primary,
                                      onPressed: () {
                                        setState(() {
                                          _albumTracks.add(track);
                                          _addedTrackIds.add(track.id);
                                          _removedTrackIds.remove(track.id);
                                        });
                                        Navigator.of(context).pop();
                                      },
                                    ),
                                  ],
                                ),
                              );
                            },
                          ),
                  ),
                ],
              ),
            );
          },
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final trackArtworks = _albumTracks
        .map((t) => t.artworkFile)
        .where((art) => art != null && File(art).existsSync())
        .cast<String>()
        .toSet()
        .toList();

    return Scaffold(
      backgroundColor: const Color(0xFF0D111A),
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [
              colorScheme.surface.withValues(alpha: 0.96),
              const Color(0xFF090C12),
            ],
          ),
        ),
        child: SafeArea(
          child: Column(
            children: [
              // Top Action Bar
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16.0, vertical: 10.0),
                child: Row(
                  children: [
                    IconButton(
                      icon: const Icon(Icons.arrow_back_rounded),
                      color: colorScheme.onSurface,
                      tooltip: 'Cancel & Return',
                      onPressed: () => Navigator.of(context).pop(),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'EDIT ALBUM & GROUP',
                            style: AttuneTypography.soraBadge(
                              color: colorScheme.primary,
                              fontSize: 11,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                          Text(
                            widget.initialAlbumName,
                            style: AttuneTypography.soraHeading(
                              fontSize: 16,
                              fontWeight: FontWeight.bold,
                              color: colorScheme.onSurface,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                    FilledButton.icon(
                      style: FilledButton.styleFrom(
                        backgroundColor: colorScheme.primary,
                        foregroundColor: colorScheme.onPrimary,
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
                        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                      ),
                      icon: const Icon(Icons.check_rounded, size: 18),
                      label: Text('Save', style: AttuneTypography.soraBadge(color: colorScheme.onPrimary, fontSize: 13)),
                      onPressed: _saveAlbum,
                    ),
                  ],
                ),
              ),

              const Divider(color: Colors.white10, height: 1),

              // Scrollable Body
              Expanded(
                child: SingleChildScrollView(
                  physics: const BouncingScrollPhysics(),
                  padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 16.0),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      // Section 1: Core Album Metadata Card
                      _buildSectionHeader('ALBUM METADATA', Icons.album_rounded, colorScheme),
                      const SizedBox(height: 10),
                      LiquidGlassContainer(
                        padding: const EdgeInsets.all(18),
                        borderRadius: 20,
                        surfaceAlpha: 0.22,
                        tintOverride: const Color(0xFF141924),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _buildInputLabel('Album Title'),
                            _buildTextField(_titleController, 'Album Title (e.g. Super Trouper)', colorScheme),
                            const SizedBox(height: 14),

                            _buildInputLabel('Primary Artist'),
                            _buildTextField(_artistController, 'Artist Name', colorScheme),
                            const SizedBox(height: 14),

                            // Multi-artist management
                            _buildInputLabel('Contributing / Additional Artists (Collaborators, Featured)'),
                            Row(
                              children: [
                                Expanded(
                                  child: _buildTextField(
                                    _newArtistController,
                                    'Add contributing artist name...',
                                    colorScheme,
                                    onSubmitted: (_) => _addAdditionalArtist(),
                                  ),
                                ),
                                const SizedBox(width: 8),
                                FilledButton.tonalIcon(
                                  style: FilledButton.styleFrom(
                                    backgroundColor: colorScheme.primary.withValues(alpha: 0.20),
                                    foregroundColor: colorScheme.primary,
                                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                                  ),
                                  icon: const Icon(Icons.person_add_rounded, size: 16),
                                  label: const Text('Add', style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold)),
                                  onPressed: _addAdditionalArtist,
                                ),
                              ],
                            ),
                            if (_additionalArtists.isNotEmpty) ...[
                              const SizedBox(height: 10),
                              Wrap(
                                spacing: 8,
                                runSpacing: 8,
                                children: _additionalArtists.map((artist) {
                                  return Container(
                                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
                                    decoration: BoxDecoration(
                                      color: colorScheme.primary.withValues(alpha: 0.15),
                                      borderRadius: BorderRadius.circular(20),
                                      border: Border.all(
                                        color: colorScheme.primary.withValues(alpha: 0.35),
                                        width: 1,
                                      ),
                                    ),
                                    child: Row(
                                      mainAxisSize: MainAxisSize.min,
                                      children: [
                                        Icon(Icons.person_outline_rounded, size: 14, color: colorScheme.primary),
                                        const SizedBox(width: 6),
                                        Text(
                                          artist,
                                          style: AttuneTypography.jakartaBodySmall(
                                            color: colorScheme.onSurface,
                                            fontSize: 12,
                                            fontWeight: FontWeight.w600,
                                          ),
                                        ),
                                        const SizedBox(width: 4),
                                        GestureDetector(
                                          onTap: () {
                                            setState(() {
                                              _additionalArtists.remove(artist);
                                            });
                                          },
                                          child: Icon(
                                            Icons.close_rounded,
                                            size: 14,
                                            color: colorScheme.onSurface.withValues(alpha: 0.7),
                                          ),
                                        ),
                                      ],
                                    ),
                                  );
                                }).toList(),
                              ),
                            ],
                            const SizedBox(height: 14),

                            Row(
                              children: [
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      _buildInputLabel('Release Year'),
                                      _buildTextField(_yearController, 'e.g. 2026', colorScheme, isNumber: true),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 14),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      _buildInputLabel('Genre'),
                                      _buildTextField(_genreController, 'e.g. Pop, Rock, Vocaloid', colorScheme),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),

                      const SizedBox(height: 24),

                      // Section: Online Query & Catalog Identifiers (MusicBrainz, ISRC, Barcode, Label)
                      _buildSectionHeader('ONLINE QUERY & CATALOG IDENTIFIERS', Icons.hub_rounded, colorScheme),
                      const SizedBox(height: 10),
                      LiquidGlassContainer(
                        padding: const EdgeInsets.all(18),
                        borderRadius: 20,
                        surfaceAlpha: 0.22,
                        tintOverride: const Color(0xFF141924),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              children: [
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      _buildInputLabel('Record Label / Publisher'),
                                      _buildTextField(_recordLabelController, 'e.g. Victor Entertainment, Crypton', colorScheme),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 14),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      _buildInputLabel('Catalog Number / Barcode'),
                                      _buildTextField(_catalogNumberController, 'e.g. VICL-63200, 498800256...', colorScheme),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 14),
                            Row(
                              children: [
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      _buildInputLabel('Album Release Type'),
                                      _buildAlbumTypeDropdown(colorScheme),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 14),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      _buildInputLabel('Release Date (YYYY-MM-DD)'),
                                      _buildTextField(_releaseDateController, 'e.g. 2026-02-14', colorScheme),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 14),
                            Row(
                              children: [
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      _buildInputLabel('Primary Language'),
                                      _buildTextField(_languageController, 'e.g. Japanese, English, Instrumental', colorScheme),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 14),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      _buildInputLabel('MusicBrainz Release ID'),
                                      _buildTextField(_musicBrainzIdController, 'e.g. 8f6b7c2a-9e12-...', colorScheme),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 14),
                            _buildInputLabel('Online Query / Wiki Source URL'),
                            _buildTextField(_onlineQueryUrlController, 'e.g. https://musicbrainz.org/release/... or wiki URL', colorScheme),
                          ],
                        ),
                      ),

                      const SizedBox(height: 24),

                      // Section 2: Splash Art & Artwork Picker Card
                      _buildSectionHeader('ALBUM SPLASH ART', Icons.image_rounded, colorScheme),
                      const SizedBox(height: 10),
                      LiquidGlassContainer(
                        padding: const EdgeInsets.all(18),
                        borderRadius: 20,
                        surfaceAlpha: 0.22,
                        tintOverride: const Color(0xFF141924),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                // Thumbnail Preview
                                ClipRRect(
                                  borderRadius: BorderRadius.circular(14),
                                  child: Container(
                                    width: 72,
                                    height: 72,
                                    color: Colors.black.withValues(alpha: 0.35),
                                    child: _splashController.text.isNotEmpty && File(_splashController.text).existsSync()
                                        ? Image.file(File(_splashController.text), fit: BoxFit.cover)
                                        : (trackArtworks.isNotEmpty
                                            ? Image.file(File(trackArtworks.first), fit: BoxFit.cover)
                                            : Icon(Icons.album_rounded, size: 36, color: colorScheme.primary)),
                                  ),
                                ),
                                const SizedBox(width: 14),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      _buildInputLabel('Custom Splash Art (Absolute File Path)'),
                                      _buildTextField(
                                        _splashController,
                                        'Paste image file path...',
                                        colorScheme,
                                        onChanged: (val) => setState(() {}),
                                      ),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                            if (trackArtworks.isNotEmpty) ...[
                              const SizedBox(height: 14),
                              Text(
                                'Quick-Select Cover from Tracks in Album:',
                                style: AttuneTypography.jakartaBodySmall(
                                  fontSize: 11,
                                  color: colorScheme.onSurface.withValues(alpha: 0.7),
                                ),
                              ),
                              const SizedBox(height: 8),
                              SizedBox(
                                height: 56,
                                child: ListView.separated(
                                  scrollDirection: Axis.horizontal,
                                  physics: const BouncingScrollPhysics(),
                                  itemCount: trackArtworks.length,
                                  separatorBuilder: (context, index) => const SizedBox(width: 10),
                                  itemBuilder: (context, idx) {
                                    final art = trackArtworks[idx];
                                    final isSelected = _splashController.text == art;
                                    return GestureDetector(
                                      onTap: () {
                                        setState(() {
                                          _splashController.text = art;
                                        });
                                      },
                                      child: Container(
                                        width: 56,
                                        height: 56,
                                        decoration: BoxDecoration(
                                          borderRadius: BorderRadius.circular(10),
                                          border: Border.all(
                                            color: isSelected ? colorScheme.primary : Colors.white12,
                                            width: isSelected ? 2.5 : 1,
                                          ),
                                        ),
                                        child: ClipRRect(
                                          borderRadius: BorderRadius.circular(8),
                                          child: Image.file(File(art), fit: BoxFit.cover),
                                        ),
                                      ),
                                    );
                                  },
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),

                      const SizedBox(height: 24),

                      // Section 3: Album Lore & Description Card
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          _buildSectionHeader('ALBUM LORE & DESCRIPTION', Icons.article_rounded, colorScheme),
                          TextButton.icon(
                            style: TextButton.styleFrom(
                              foregroundColor: colorScheme.primary,
                              visualDensity: VisualDensity.compact,
                            ),
                            icon: const Icon(Icons.auto_awesome_rounded, size: 14),
                            label: const Text('Auto-Synthesize', style: TextStyle(fontSize: 11)),
                            onPressed: () {
                              setState(() {
                                _descController.text = _generateDynamicAlbumDescription(_titleController.text, _albumTracks);
                              });
                            },
                          ),
                        ],
                      ),
                      const SizedBox(height: 10),
                      LiquidGlassContainer(
                        padding: const EdgeInsets.all(18),
                        borderRadius: 20,
                        surfaceAlpha: 0.22,
                        tintOverride: const Color(0xFF141924),
                        child: TextField(
                          controller: _descController,
                          maxLines: 4,
                          style: AttuneTypography.jakartaBodySmall(
                            color: colorScheme.onSurface,
                            fontSize: 12,
                          ),
                          decoration: InputDecoration(
                            hintText: 'Enter album background lore, liner notes, or release details...',
                            hintStyle: AttuneTypography.jakartaBodySmall(
                              color: colorScheme.onSurface.withValues(alpha: 0.45),
                            ),
                            filled: true,
                            fillColor: Colors.black.withValues(alpha: 0.25),
                            border: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(12),
                              borderSide: BorderSide.none,
                            ),
                            contentPadding: const EdgeInsets.all(14),
                          ),
                        ),
                      ),

                      const SizedBox(height: 24),

                      // Section 4: Group Tracks Management
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          _buildSectionHeader('TRACKS IN GROUP (${_albumTracks.length})', Icons.queue_music_rounded, colorScheme),
                          Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              _buildSortMenuButton(colorScheme),
                              const SizedBox(width: 8),
                              FilledButton.tonalIcon(
                                style: FilledButton.styleFrom(
                                  backgroundColor: colorScheme.primary.withValues(alpha: 0.18),
                                  foregroundColor: colorScheme.primary,
                                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                                ),
                                icon: const Icon(Icons.add_rounded, size: 16),
                                label: const Text('Add Songs', style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold)),
                                onPressed: () => _showAddTracksModal(context, colorScheme),
                              ),
                            ],
                          ),
                        ],
                      ),
                      const SizedBox(height: 10),

                      if (_albumTracks.isEmpty)
                        LiquidGlassContainer(
                          padding: const EdgeInsets.all(24),
                          borderRadius: 20,
                          surfaceAlpha: 0.16,
                          child: Center(
                            child: Text(
                              'This album group has no tracks. Tap "+ Add Songs" above to group songs into this album.',
                              style: AttuneTypography.jakartaBodySmall(
                                color: colorScheme.onSurface.withValues(alpha: 0.55),
                              ),
                              textAlign: TextAlign.center,
                            ),
                          ),
                        )
                      else
                        ListView.separated(
                          shrinkWrap: true,
                          physics: const NeverScrollableScrollPhysics(),
                          itemCount: _albumTracks.length,
                          separatorBuilder: (context, index) => const SizedBox(height: 8),
                          itemBuilder: (context, idx) {
                            final track = _albumTracks[idx];
                            return LiquidGlassContainer(
                              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                              borderRadius: 14,
                              surfaceAlpha: 0.18,
                              child: Row(
                                children: [
                                  Text(
                                    '${idx + 1}',
                                    style: AttuneTypography.soraBadge(
                                      fontSize: 12,
                                      color: colorScheme.onSurface.withValues(alpha: 0.5),
                                    ),
                                  ),
                                  const SizedBox(width: 12),
                                  ClipRRect(
                                    borderRadius: BorderRadius.circular(8),
                                    child: SizedBox(
                                      width: 40,
                                      height: 40,
                                      child: track.artworkFile != null && File(track.artworkFile!).existsSync()
                                          ? Image.file(File(track.artworkFile!), fit: BoxFit.cover)
                                          : Container(
                                              color: colorScheme.primary.withValues(alpha: 0.2),
                                              child: Icon(Icons.music_note_rounded, color: colorScheme.primary, size: 20),
                                            ),
                                    ),
                                  ),
                                  const SizedBox(width: 12),
                                  Expanded(
                                    child: Column(
                                      crossAxisAlignment: CrossAxisAlignment.start,
                                      children: [
                                        Text(
                                          track.title,
                                          style: AttuneTypography.soraSubheading(
                                            fontSize: 13,
                                            fontWeight: FontWeight.w600,
                                            color: colorScheme.onSurface,
                                          ),
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                        Text(
                                          '${track.artist} • ${track.formattedDuration}',
                                          style: AttuneTypography.jakartaBodySmall(
                                            fontSize: 11,
                                            color: colorScheme.onSurface.withValues(alpha: 0.6),
                                          ),
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                      ],
                                    ),
                                  ),
                                  if (idx > 0)
                                    IconButton(
                                      icon: const Icon(Icons.arrow_upward_rounded, size: 16),
                                      color: colorScheme.onSurface.withValues(alpha: 0.4),
                                      padding: EdgeInsets.zero,
                                      constraints: const BoxConstraints(minWidth: 26, minHeight: 26),
                                      tooltip: 'Move up in album',
                                      onPressed: () {
                                        setState(() {
                                          final item = _albumTracks.removeAt(idx);
                                          _albumTracks.insert(idx - 1, item);
                                        });
                                      },
                                    ),
                                  if (idx < _albumTracks.length - 1)
                                    IconButton(
                                      icon: const Icon(Icons.arrow_downward_rounded, size: 16),
                                      color: colorScheme.onSurface.withValues(alpha: 0.4),
                                      padding: EdgeInsets.zero,
                                      constraints: const BoxConstraints(minWidth: 26, minHeight: 26),
                                      tooltip: 'Move down in album',
                                      onPressed: () {
                                        setState(() {
                                          final item = _albumTracks.removeAt(idx);
                                          _albumTracks.insert(idx + 1, item);
                                        });
                                      },
                                    ),
                                  IconButton(
                                    icon: const Icon(Icons.remove_circle_outline_rounded, size: 20),
                                    color: Colors.redAccent.withValues(alpha: 0.8),
                                    tooltip: 'Remove from this album',
                                    onPressed: () {
                                      setState(() {
                                        _albumTracks.removeAt(idx);
                                        _removedTrackIds.add(track.id);
                                        _addedTrackIds.remove(track.id);
                                      });
                                    },
                                  ),
                                ],
                              ),
                            );
                          },
                        ),

                      const SizedBox(height: 40),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildSortMenuButton(ColorScheme colorScheme) {
    return PopupMenuButton<TrackSortOption>(
      tooltip: 'Sort tracks by artist, title, or audio properties',
      icon: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.35),
          borderRadius: BorderRadius.circular(10),
          border: Border.all(color: Colors.white12),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.sort_rounded, size: 14, color: colorScheme.primary),
            const SizedBox(width: 4),
            Text(
              _sortLabel(_currentSort),
              style: AttuneTypography.jakartaBodySmall(fontSize: 11, color: colorScheme.onSurface),
            ),
          ],
        ),
      ),
      onSelected: _applyTrackSort,
      itemBuilder: (context) => [
        const PopupMenuItem(
          value: TrackSortOption.artistAsc,
          child: Text('Sort by Artist (A → Z)'),
        ),
        const PopupMenuItem(
          value: TrackSortOption.artistDesc,
          child: Text('Sort by Artist (Z → A)'),
        ),
        const PopupMenuItem(
          value: TrackSortOption.titleAsc,
          child: Text('Sort by Title (A → Z)'),
        ),
        const PopupMenuItem(
          value: TrackSortOption.bpmDesc,
          child: Text('Sort by BPM (Fastest First)'),
        ),
        const PopupMenuItem(
          value: TrackSortOption.durationDesc,
          child: Text('Sort by Duration (Longest First)'),
        ),
        const PopupMenuItem(
          value: TrackSortOption.original,
          child: Text('Original Track Order'),
        ),
      ],
    );
  }

  Widget _buildAlbumTypeDropdown(ColorScheme colorScheme) {
    final types = [
      'Studio Album',
      'EP',
      'Single',
      'Compilation',
      'Soundtrack / OST',
      'Live Album',
      'Remix Album',
      'Mixtape / Flow',
    ];
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12),
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.3),
        borderRadius: BorderRadius.circular(12),
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<String>(
          value: types.contains(_albumTypeController.text) ? _albumTypeController.text : types.first,
          isExpanded: true,
          dropdownColor: const Color(0xFF141924),
          icon: Icon(Icons.arrow_drop_down_rounded, color: colorScheme.primary),
          style: AttuneTypography.jakartaBodySmall(color: colorScheme.onSurface, fontSize: 13),
          onChanged: (val) {
            if (val != null) {
              setState(() {
                _albumTypeController.text = val;
              });
            }
          },
          items: types.map((t) => DropdownMenuItem(value: t, child: Text(t))).toList(),
        ),
      ),
    );
  }

  Widget _buildSectionHeader(String title, IconData icon, ColorScheme colorScheme) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 16, color: colorScheme.primary),
        const SizedBox(width: 8),
        Text(
          title,
          style: AttuneTypography.soraBadge(
            color: colorScheme.primary,
            fontSize: 11,
            fontWeight: FontWeight.bold,
          ),
        ),
      ],
    );
  }

  Widget _buildInputLabel(String label) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 6.0),
      child: Text(
        label,
        style: AttuneTypography.jakartaBodySmall(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: Colors.white70,
        ),
      ),
    );
  }

  Widget _buildTextField(
    TextEditingController controller,
    String hint,
    ColorScheme colorScheme, {
    bool isNumber = false,
    void Function(String)? onChanged,
    void Function(String)? onSubmitted,
  }) {
    return TextField(
      controller: controller,
      keyboardType: isNumber ? TextInputType.number : TextInputType.text,
      onChanged: onChanged,
      onSubmitted: onSubmitted,
      style: AttuneTypography.jakartaBodySmall(color: colorScheme.onSurface, fontSize: 13),
      decoration: InputDecoration(
        hintText: hint,
        hintStyle: AttuneTypography.jakartaBodySmall(
          color: colorScheme.onSurface.withValues(alpha: 0.4),
          fontSize: 12,
        ),
        filled: true,
        fillColor: Colors.black.withValues(alpha: 0.3),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: BorderSide.none,
        ),
        contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      ),
    );
  }
}
