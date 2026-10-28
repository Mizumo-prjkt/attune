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

  // Skin
  double _glassLensBlurSigma = 36.0;
  double _glassSurfaceAlpha = 0.14;
  bool _glowBlobFollowsRhythm = true;

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

  Set<String> get allowedDirectories => Set.unmodifiable(_allowedDirectories);
  Set<String> get excludedSongIds => Set.unmodifiable(_excludedSongIds);
  int get minSongDurationSec => _minSongDurationSec;

  // Setters & Modifiers
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
        }
      }
    } catch (e) {
      debugPrint('[SettingsService] Notice loading settings: $e');
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
