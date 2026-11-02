import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'environment_delegator.dart';

/// Audio focus hook sensitivity levels
enum AudioFocusLevel {
  standard,
  sensitive,
  ultraSensitive,
  ahFuckAlerts,
  ahFuckEverything;

  String get displayName {
    switch (this) {
      case AudioFocusLevel.standard:
        return 'Standard (Default)';
      case AudioFocusLevel.sensitive:
        return 'Sensitive';
      case AudioFocusLevel.ultraSensitive:
        return 'Ultra Sensitive';
      case AudioFocusLevel.ahFuckAlerts:
        return 'Ah f#ck alerts';
      case AudioFocusLevel.ahFuckEverything:
        return 'Ah F#ck everything';
    }
  }

  String get description {
    switch (this) {
      case AudioFocusLevel.standard:
        return 'Pause music during calls, resume when done. Pause music when another app requests focus for long or depending on severity. Lower audio level when there is a notification (hit or miss depending on 3rd party apps).';
      case AudioFocusLevel.sensitive:
        return 'Includes Standard, but any sound poke immediately pauses all playback (notifications, calls, mini sound blings).';
      case AudioFocusLevel.ultraSensitive:
        return 'Same as Sensitive, plus shaking your device can pause playback (ideal when you cannot easily reach your device).';
      case AudioFocusLevel.ahFuckAlerts:
        return 'Standard for calls, but will not yield audio focus to any app wanting to make a sound. (Not recommended).';
      case AudioFocusLevel.ahFuckEverything:
        return 'Same as Ah f#ck alerts, but also ignores incoming phone calls. (Not recommended).';
    }
  }
}

/// Dynamic Home Section configuration model for non-hardcoded customization
enum ForYouDesignMode {
  m3Expressive,
  betaCards;

  String get label {
    switch (this) {
      case ForYouDesignMode.m3Expressive:
        return 'M3 Expressive';
      case ForYouDesignMode.betaCards:
        return 'Beta Cards';
    }
  }
}

/// Now Playing screen layout & presentation skins
enum NowPlayingSkin {
  defaultSkin,
  materialYouFreeshapes,
  normalSquareBig;

  String get displayName {
    switch (this) {
      case NowPlayingSkin.defaultSkin:
        return 'Liquid Glass (Default)';
      case NowPlayingSkin.materialYouFreeshapes:
        return 'Material You Freeshapes';
      case NowPlayingSkin.normalSquareBig:
        return 'Normal Square Big';
    }
  }

  String get description {
    switch (this) {
      case NowPlayingSkin.defaultSkin:
        return 'Signature Attune liquid glass player with reactive glow and layered glass curvature.';
      case NowPlayingSkin.materialYouFreeshapes:
        return 'Expressive dynamic organic shapes morphing with each track, with silhouette backdrop lyrics.';
      case NowPlayingSkin.normalSquareBig:
        return 'Expansive edge-to-edge square artwork with quick side subtitle/lyrics access and clean controls.';
    }
  }
}

class HomeSectionConfig {
  final String id;
  final String titleKey;
  final String descriptionKey;
  final IconData icon;
  bool enabled;

  HomeSectionConfig({
    required this.id,
    required this.titleKey,
    required this.descriptionKey,
    required this.icon,
    this.enabled = true,
  });

  Map<String, dynamic> toJson() => {
    'id': id,
    'enabled': enabled,
  };

  HomeSectionConfig copyWith({bool? enabled}) {
    return HomeSectionConfig(
      id: id,
      titleKey: titleKey,
      descriptionKey: descriptionKey,
      icon: icon,
      enabled: enabled ?? this.enabled,
    );
  }
}

/// Audio decoding engine modes
enum AudioDecoderMode {
  attuneRust,
  systemDefault;

  // Backwards compatibility alias
  static AudioDecoderMode get androidDefault => systemDefault;

  String get displayName {
    switch (this) {
      case AudioDecoderMode.attuneRust:
        return "Attune's Rust Codec (Recommended)";
      case AudioDecoderMode.systemDefault:
        return Platform.isAndroid ? 'Android Default' : 'System Default';
    }
  }

  String get description {
    switch (this) {
      case AudioDecoderMode.attuneRust:
        return 'Bit-perfect native Rust audio engine with 4-point Hermite resampling, 5-layer profiling, and smart shuffle transitions.';
      case AudioDecoderMode.systemDefault:
        return Platform.isAndroid
            ? 'Standard OS MediaCodec. You will lose smart shuffle transitions and unsupported formats may not play well.'
            : 'Standard OS audio decoder. You will lose smart shuffle transitions and unsupported formats may not play well.';
    }
  }
}

/// Equalizer modes
enum EqualizerMode {
  attuneEq,
  systemDefault,
  off;

  String get displayName {
    switch (this) {
      case EqualizerMode.attuneEq:
        return "Attune's EQ Implementation";
      case EqualizerMode.systemDefault:
        return 'System Default';
      case EqualizerMode.off:
        return 'Off';
    }
  }

  String get description {
    switch (this) {
      case EqualizerMode.attuneEq:
        return "Active 3-band parametric DSP equalizer running inside Attune's native audio pipeline.";
      case EqualizerMode.systemDefault:
        return 'Pass audio through global system audio equalizer effects.';
      case EqualizerMode.off:
        return 'Bypass all equalization filters for uncolored bit-perfect audio reproduction.';
    }
  }
}

/// Central reactive settings service managing application preferences,
/// audio focus behaviors, dynamic UI material glass parameters,
/// filesystem scan directories, and developer diagnostics.
class SettingsService extends ChangeNotifier {
  static final SettingsService instance = SettingsService._internal();

  SettingsService._internal() {
    _loadFromDisk();
  }

  // General
  String _language = 'en';
  bool _wakelockHeld = true;
  bool _notificationsAllowed = true;

  // Audio
  AudioFocusLevel _audioFocusLevel = AudioFocusLevel.standard;
  AudioDecoderMode _decoderMode = AudioDecoderMode.attuneRust;
  EqualizerMode _equalizerMode = EqualizerMode.attuneEq;
  double _eqLowDb = 0.0;
  double _eqMidDb = 0.0;
  double _eqHighDb = 0.0;

  // Lyrics
  bool _lrclibFetchEnabled = false;

  // Skin & Liquid Glass
  NowPlayingSkin _nowPlayingSkin = NowPlayingSkin.defaultSkin;
  double _glassLensBlurSigma = 36.0;
  double _glassSurfaceAlpha = 0.14;
  bool _glowBlobFollowsRhythm = true;
  bool _liquidGlass3dMirror = true;
  bool _pendingLiquidGlass3dMirror = true;

  // Home Layout Customization
  List<HomeSectionConfig> _homeSections = [];
  ForYouDesignMode _forYouDesignMode = ForYouDesignMode.m3Expressive;

  static List<HomeSectionConfig> get defaultHomeSections => [
    HomeSectionConfig(
      id: 'search_bar',
      titleKey: 'home_section_search',
      descriptionKey: 'home_section_search_desc',
      icon: Icons.search_rounded,
      enabled: true,
    ),
    HomeSectionConfig(
      id: 'for_you_mixes',
      titleKey: 'home_section_for_you',
      descriptionKey: 'home_section_for_you_desc',
      icon: Icons.auto_awesome_rounded,
      enabled: true,
    ),
    HomeSectionConfig(
      id: 'genre_chips',
      titleKey: 'home_section_genres',
      descriptionKey: 'home_section_genres_desc',
      icon: Icons.music_note_rounded,
      enabled: true,
    ),
    HomeSectionConfig(
      id: 'quick_picks',
      titleKey: 'home_section_quick_picks',
      descriptionKey: 'home_section_quick_picks_desc',
      icon: Icons.bolt_rounded,
      enabled: true,
    ),
    HomeSectionConfig(
      id: 'recent_tracks',
      titleKey: 'home_section_recent',
      descriptionKey: 'home_section_recent_desc',
      icon: Icons.history_rounded,
      enabled: true,
    ),
    HomeSectionConfig(
      id: 'all_tracks',
      titleKey: 'home_section_all_tracks',
      descriptionKey: 'home_section_all_tracks_desc',
      icon: Icons.library_music_rounded,
      enabled: true,
    ),
  ];

  // Files
  final Set<String> _allowedDirectories = {};
  final Set<String> _excludedSongIds = {};
  int _minSongDurationSec = 60;

  // Profile
  String _profileName = '';

  // Getters
  String get language => _language;
  String get profileName {
    if (_profileName.trim().isNotEmpty) return _profileName.trim();
    final envUser = AppEnvironmentDelegator.current.defaultUsername;
    if (envUser.isNotEmpty) {
      return envUser[0].toUpperCase() + envUser.substring(1);
    }
    return 'Listener';
  }

  bool get wakelockHeld => _wakelockHeld;
  bool get notificationsAllowed => _notificationsAllowed;

  AudioFocusLevel get audioFocusLevel => _audioFocusLevel;
  AudioDecoderMode get decoderMode => _decoderMode;
  EqualizerMode get equalizerMode => _equalizerMode;
  double get eqLowDb => _eqLowDb;
  double get eqMidDb => _eqMidDb;
  double get eqHighDb => _eqHighDb;

  bool get lrclibFetchEnabled => _lrclibFetchEnabled;

  double get glassLensBlurSigma => _glassLensBlurSigma;
  double get glassSurfaceAlpha => _glassSurfaceAlpha;
  bool get glowBlobFollowsRhythm => _glowBlobFollowsRhythm;
  bool get liquidGlass3dMirror => _liquidGlass3dMirror;
  bool get pendingLiquidGlass3dMirror => _pendingLiquidGlass3dMirror;
  bool get restartRequiredForGlass => _pendingLiquidGlass3dMirror != _liquidGlass3dMirror;
  NowPlayingSkin get nowPlayingSkin => _nowPlayingSkin;

  List<HomeSectionConfig> get homeSections => List.unmodifiable(_homeSections);
  ForYouDesignMode get forYouDesignMode => _forYouDesignMode;

  Set<String> get allowedDirectories => Set.unmodifiable(_allowedDirectories);
  Set<String> get excludedSongIds => Set.unmodifiable(_excludedSongIds);
  int get minSongDurationSec => _minSongDurationSec;

  // Setters & Modifiers
  void setNowPlayingSkin(NowPlayingSkin skin) {
    if (_nowPlayingSkin == skin) return;
    _nowPlayingSkin = skin;
    notifyListeners();
    _saveToDisk();
  }

  void setForYouDesignMode(ForYouDesignMode mode) {
    if (_forYouDesignMode == mode) return;
    _forYouDesignMode = mode;
    notifyListeners();
    _saveToDisk();
  }

  void setProfileName(String name) {
    if (_profileName == name.trim()) return;
    _profileName = name.trim();
    notifyListeners();
    _saveToDisk();
  }

  void setLanguage(String lang) {
    if (_language == lang) return;
    _language = lang;
    notifyListeners();
    _saveToDisk();
  }

  void setWakelockHeld(bool val) {
    if (_wakelockHeld == val) return;
    _wakelockHeld = val;
    notifyListeners();
    _saveToDisk();
  }

  void setNotificationsAllowed(bool val) {
    if (_notificationsAllowed == val) return;
    _notificationsAllowed = val;
    notifyListeners();
    _saveToDisk();
  }

  void setAudioFocusLevel(AudioFocusLevel level) {
    if (_audioFocusLevel == level) return;
    _audioFocusLevel = level;
    notifyListeners();
    _saveToDisk();
  }

  void setDecoderMode(AudioDecoderMode mode) {
    if (_decoderMode == mode) return;
    _decoderMode = mode;
    notifyListeners();
    _saveToDisk();
  }

  void setEqualizerMode(EqualizerMode mode) {
    if (_equalizerMode == mode) return;
    _equalizerMode = mode;
    notifyListeners();
    _saveToDisk();
  }

  void setEqBands({double? lowDb, double? midDb, double? highDb}) {
    if (lowDb != null) _eqLowDb = lowDb.clamp(-12.0, 12.0);
    if (midDb != null) _eqMidDb = midDb.clamp(-12.0, 12.0);
    if (highDb != null) _eqHighDb = highDb.clamp(-12.0, 12.0);
    notifyListeners();
    _saveToDisk();
  }

  void resetEqualizer() {
    _eqLowDb = 0.0;
    _eqMidDb = 0.0;
    _eqHighDb = 0.0;
    notifyListeners();
    _saveToDisk();
  }

  void setLrclibFetchEnabled(bool val) {
    if (_lrclibFetchEnabled == val) return;
    _lrclibFetchEnabled = val;
    notifyListeners();
    _saveToDisk();
  }

  void setGlassLensBlurSigma(double val) {
    final clamped = val.clamp(8.0, 64.0);
    if (_glassLensBlurSigma == clamped) return;
    _glassLensBlurSigma = clamped;
    notifyListeners();
    _saveToDisk();
  }

  void setGlassSurfaceAlpha(double val) {
    final clamped = val.clamp(0.04, 0.45);
    if (_glassSurfaceAlpha == clamped) return;
    _glassSurfaceAlpha = clamped;
    notifyListeners();
    _saveToDisk();
  }

  void setGlowBlobFollowsRhythm(bool val) {
    if (_glowBlobFollowsRhythm == val) return;
    _glowBlobFollowsRhythm = val;
    notifyListeners();
    _saveToDisk();
  }

  void setPendingLiquidGlass3dMirror(bool val) {
    if (_pendingLiquidGlass3dMirror == val) return;
    _pendingLiquidGlass3dMirror = val;
    notifyListeners();
    _saveToDisk();
  }

  void reorderHomeSections(int oldIndex, int newIndex) {
    if (oldIndex < 0 || oldIndex >= _homeSections.length) return;
    if (newIndex < 0 || newIndex > _homeSections.length) return;
    if (oldIndex < newIndex) {
      newIndex -= 1;
    }
    final item = _homeSections.removeAt(oldIndex);
    _homeSections.insert(newIndex, item);
    notifyListeners();
    _saveToDisk();
  }

  void toggleHomeSection(String id, bool enabled) {
    final idx = _homeSections.indexWhere((s) => s.id == id);
    if (idx != -1) {
      _homeSections[idx] = _homeSections[idx].copyWith(enabled: enabled);
      notifyListeners();
      _saveToDisk();
    }
  }

  void resetHomeSectionsToDefault() {
    _homeSections.clear();
    for (final def in defaultHomeSections) {
      _homeSections.add(def.copyWith());
    }
    notifyListeners();
    _saveToDisk();
  }

  void toggleDirectoryAllowed(String path, bool allowed) {
    if (allowed) {
      _allowedDirectories.add(path);
    } else {
      _allowedDirectories.remove(path);
    }
    notifyListeners();
    _saveToDisk();
  }

  void addCustomDirectory(String path) {
    if (path.trim().isEmpty) return;
    _allowedDirectories.add(path.trim());
    notifyListeners();
    _saveToDisk();
  }

  void toggleSongExcluded(String songId) {
    if (_excludedSongIds.contains(songId)) {
      _excludedSongIds.remove(songId);
    } else {
      _excludedSongIds.add(songId);
    }
    notifyListeners();
    _saveToDisk();
  }

  void removeExcludedSong(String songId) {
    if (_excludedSongIds.remove(songId)) {
      notifyListeners();
      _saveToDisk();
    }
  }

  void setMinSongDurationSec(int seconds) {
    final clamped = seconds.clamp(5, 300);
    if (_minSongDurationSec == clamped) return;
    _minSongDurationSec = clamped;
    notifyListeners();
    _saveToDisk();
  }

  File get _settingsFile {
    final dir = AppEnvironmentDelegator.current.appDataDirectory;
    return File('${dir.path}/app_settings.json');
  }


  void _loadFromDisk() {
    try {
      // Initialize allowed directories dynamically from real existing candidate roots
      final candidates = AppEnvironmentDelegator.current.getMusicCandidateRoots();
      for (final root in candidates) {
        if (Directory(root).existsSync()) {
          _allowedDirectories.add(root);
        }
      }

      final file = _settingsFile;
      if (file.existsSync()) {
        final content = file.readAsStringSync();
        if (content.isNotEmpty) {
          final map = jsonDecode(content) as Map<String, dynamic>;
          _language = map['language'] as String? ?? _language;
          _wakelockHeld = map['wakelock_held'] as bool? ?? _wakelockHeld;
          _notificationsAllowed = map['notifications_allowed'] as bool? ?? _notificationsAllowed;

          final focusIdx = map['audio_focus_level'] as int?;
          if (focusIdx != null && focusIdx >= 0 && focusIdx < AudioFocusLevel.values.length) {
            _audioFocusLevel = AudioFocusLevel.values[focusIdx];
          }

          final decoderIdx = map['decoder_mode'] as int?;
          if (decoderIdx != null && decoderIdx >= 0 && decoderIdx < AudioDecoderMode.values.length) {
            _decoderMode = AudioDecoderMode.values[decoderIdx];
          }

          final eqIdx = map['equalizer_mode'] as int?;
          if (eqIdx != null && eqIdx >= 0 && eqIdx < EqualizerMode.values.length) {
            _equalizerMode = EqualizerMode.values[eqIdx];
          }

          _eqLowDb = (map['eq_low_db'] as num?)?.toDouble() ?? _eqLowDb;
          _eqMidDb = (map['eq_mid_db'] as num?)?.toDouble() ?? _eqMidDb;
          _eqHighDb = (map['eq_high_db'] as num?)?.toDouble() ?? _eqHighDb;

          _lrclibFetchEnabled = map['lrclib_fetch_enabled'] as bool? ?? _lrclibFetchEnabled;
          _glassLensBlurSigma = (map['glass_lens_blur_sigma'] as num?)?.toDouble() ?? _glassLensBlurSigma;
          _glassSurfaceAlpha = (map['glass_surface_alpha'] as num?)?.toDouble() ?? _glassSurfaceAlpha;
          _glowBlobFollowsRhythm = map['glow_blob_follows_rhythm'] as bool? ?? _glowBlobFollowsRhythm;

          if (map['allowed_directories'] is List) {
            final list = (map['allowed_directories'] as List).cast<String>();
            if (list.isNotEmpty) {
              _allowedDirectories.clear();
              _allowedDirectories.addAll(list);
            }
          }

          if (map['excluded_song_ids'] is List) {
            final list = (map['excluded_song_ids'] as List).cast<String>();
            _excludedSongIds.clear();
            _excludedSongIds.addAll(list);
          }

          _minSongDurationSec = map['min_song_duration_sec'] as int? ?? _minSongDurationSec;
          _profileName = map['profile_name'] as String? ?? _profileName;

          _liquidGlass3dMirror = map['liquid_glass_3d_mirror'] as bool? ?? _liquidGlass3dMirror;
          _pendingLiquidGlass3dMirror = _liquidGlass3dMirror;

          final forYouIdx = map['for_you_design_mode'] as int?;
          if (forYouIdx != null && forYouIdx >= 0 && forYouIdx < ForYouDesignMode.values.length) {
            _forYouDesignMode = ForYouDesignMode.values[forYouIdx];
          }

          final skinIdx = map['now_playing_skin'] as int?;
          if (skinIdx != null && skinIdx >= 0 && skinIdx < NowPlayingSkin.values.length) {
            _nowPlayingSkin = NowPlayingSkin.values[skinIdx];
          }

          if (map['home_sections'] is List) {
            final savedList = (map['home_sections'] as List).whereType<Map<String, dynamic>>().toList();
            final loaded = <HomeSectionConfig>[];
            final defaultMap = {for (final d in defaultHomeSections) d.id: d};

            for (final item in savedList) {
              final id = item['id'] as String?;
              if (id != null && defaultMap.containsKey(id)) {
                final def = defaultMap.remove(id)!;
                loaded.add(def.copyWith(enabled: item['enabled'] as bool? ?? true));
              }
            }
            for (final remaining in defaultMap.values) {
              loaded.add(remaining.copyWith());
            }
            _homeSections = loaded;
          } else {
            _homeSections = defaultHomeSections.map((d) => d.copyWith()).toList();
          }
        }
      }
      if (_homeSections.isEmpty) {
        _homeSections = defaultHomeSections.map((d) => d.copyWith()).toList();
      }
    } catch (e) {
      debugPrint('[SettingsService] Notice loading settings: $e');
      if (_homeSections.isEmpty) {
        _homeSections = defaultHomeSections.map((d) => d.copyWith()).toList();
      }
    }
  }

  void _saveToDisk() {
    try {
      final file = _settingsFile;
      final map = {
        'profile_name': _profileName,
        'language': _language,
        'wakelock_held': _wakelockHeld,
        'notifications_allowed': _notificationsAllowed,
        'audio_focus_level': _audioFocusLevel.index,
        'decoder_mode': _decoderMode.index,
        'equalizer_mode': _equalizerMode.index,
        'eq_low_db': _eqLowDb,
        'eq_mid_db': _eqMidDb,
        'eq_high_db': _eqHighDb,
        'lrclib_fetch_enabled': _lrclibFetchEnabled,
        'glass_lens_blur_sigma': _glassLensBlurSigma,
        'glass_surface_alpha': _glassSurfaceAlpha,
        'glow_blob_follows_rhythm': _glowBlobFollowsRhythm,
        'liquid_glass_3d_mirror': _pendingLiquidGlass3dMirror,
        'now_playing_skin': _nowPlayingSkin.index,
        'for_you_design_mode': _forYouDesignMode.index,
        'home_sections': _homeSections.map((s) => s.toJson()).toList(),
        'allowed_directories': _allowedDirectories.toList(),
        'excluded_song_ids': _excludedSongIds.toList(),
        'min_song_duration_sec': _minSongDurationSec,
        'updated_at': DateTime.now().toIso8601String(),
      };
      file.writeAsStringSync(jsonEncode(map));
    } catch (e) {
      debugPrint('[SettingsService] Notice saving settings: $e');
    }
  }
}
