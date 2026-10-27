import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'engine_bridge.dart';

const List<double> kEqualizerFrequencies = [
  31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0
];

const Map<String, List<double>> kEqualizerPresets = {
  'Flat': [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
  'Rock': [4.5, 3.0, 1.5, 0.0, -1.0, 0.0, 2.0, 3.5, 4.0, 4.5],
  'Pop': [-1.0, 1.0, 2.5, 3.5, 2.0, 0.0, 1.5, 2.5, 3.0, 2.0],
  'Electronic': [5.0, 4.0, 2.0, 0.0, -1.5, 1.0, 2.0, 3.5, 4.5, 5.0],
  'Vocal Boost': [-2.0, -1.5, 0.0, 1.5, 3.5, 4.5, 3.0, 1.5, 0.0, -1.0],
  'Acoustic': [3.0, 2.5, 1.5, 0.5, 1.0, 1.5, 2.5, 3.0, 3.5, 3.0],
  'Bass Punch': [6.5, 5.5, 3.5, 1.0, -0.5, -1.0, 0.0, 1.0, 1.5, 2.0],
};

/// Attune Studio Master FX Rack & Equalizer Service
/// Manages state, presets, psychoacoustic bass, clarity exciter, stereo widening,
/// external DSP conflict detection, and hardware synchronization.
class EqualizerService extends ChangeNotifier {
  static EqualizerService? _instance;
  static EqualizerService get instance => _instance ??= EqualizerService._();

  bool _initialized = false;
  bool _enabled = true;
  double _preampDb = -3.0; // 3 dB headroom pad preventing filter clipping
  List<double> _bandGainsDb = List.filled(10, 0.0);
  String _activePreset = 'Flat';

  // Attune Pure Bass+
  bool _bassEnabled = false;
  double _bassGainDb = 4.0;
  double _bassFreqHz = 80.0;

  // Attune Clarity & Vocal Air
  bool _clarityEnabled = false;
  double _clarityWet = 0.12;

  // Stereo Field Expander
  double _stereoWidth = 1.0;

  // External DSP Detection
  bool _externalDspDetected = false;
  String _externalDspName = 'None';
  String _externalDspDetails = '';
  String _externalDspRecommendedAction = '';
  bool _bypassExternalDspWarning = false;

  EqualizerService._() {
    init();
  }

  bool get isInitialized => _initialized;
  bool get enabled => _enabled;
  double get preampDb => _preampDb;
  List<double> get bandGainsDb => List.unmodifiable(_bandGainsDb);
  String get activePreset => _activePreset;

  bool get bassEnabled => _bassEnabled;
  double get bassGainDb => _bassGainDb;
  double get bassFreqHz => _bassFreqHz;

  bool get clarityEnabled => _clarityEnabled;
  double get clarityWet => _clarityWet;

  double get stereoWidth => _stereoWidth;
  bool get stereoExpanderEnabled => (_stereoWidth - 1.0).abs() > 0.05;

  bool get externalDspDetected => _externalDspDetected;
  String get externalDspName => _externalDspName;
  String get externalDspDetails => _externalDspDetails;
  String get externalDspRecommendedAction => _externalDspRecommendedAction;
  bool get bypassExternalDspWarning => _bypassExternalDspWarning;

  File get _configFile {
    final home = Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'] ?? '.';
    final dir = Directory('$home/.config/attune');
    if (!dir.existsSync()) {
      try {
        dir.createSync(recursive: true);
      } catch (_) {}
    }
    return File('${dir.path}/equalizer_profile.json');
  }

  void init() {
    if (_initialized) return;
    _loadFromDisk();
    _probeExternalDsp();
    _syncWithNativeBridge();
    _initialized = true;
    notifyListeners();
  }

  void _loadFromDisk() {
    try {
      final file = _configFile;
      if (file.existsSync()) {
        final content = file.readAsStringSync();
        final map = jsonDecode(content) as Map<String, dynamic>;

        _enabled = map['enabled'] as bool? ?? true;
        _preampDb = (map['preamp_db'] as num?)?.toDouble().clamp(-12.0, 6.0) ?? -3.0;

        if (map['band_gains_db'] is List) {
          final list = (map['band_gains_db'] as List)
              .map((e) => (e as num).toDouble().clamp(-12.0, 12.0))
              .toList();
          if (list.length == 10) {
            _bandGainsDb = list;
          }
        }

        _activePreset = map['active_preset'] as String? ?? 'Custom';
        _bassEnabled = map['bass_enabled'] as bool? ?? false;
        _bassGainDb = (map['bass_gain_db'] as num?)?.toDouble().clamp(0.0, 14.0) ?? 4.0;
        _bassFreqHz = (map['bass_freq_hz'] as num?)?.toDouble().clamp(40.0, 150.0) ?? 80.0;

        _clarityEnabled = map['clarity_enabled'] as bool? ?? false;
        _clarityWet = (map['clarity_wet'] as num?)?.toDouble().clamp(0.0, 0.40) ?? 0.12;

        _stereoWidth = (map['stereo_width'] as num?)?.toDouble().clamp(0.0, 2.5) ?? 1.0;
        _bypassExternalDspWarning = map['bypass_external_dsp_warning'] as bool? ?? false;
      }
    } catch (e) {
      debugPrint('[EqualizerService] Failed to load equalizer profile: $e');
    }
  }

  void _saveToDisk() {
    try {
      final file = _configFile;
      final map = {
        'enabled': _enabled,
        'preamp_db': _preampDb,
        'band_gains_db': _bandGainsDb,
        'active_preset': _activePreset,
        'bass_enabled': _bassEnabled,
        'bass_gain_db': _bassGainDb,
        'bass_freq_hz': _bassFreqHz,
        'clarity_enabled': _clarityEnabled,
        'clarity_wet': _clarityWet,
        'stereo_width': _stereoWidth,
        'bypass_external_dsp_warning': _bypassExternalDspWarning,
      };
      file.writeAsStringSync(const JsonEncoder.withIndent('  ').convert(map));
    } catch (e) {
      debugPrint('[EqualizerService] Failed to save equalizer profile: $e');
    }
  }

  void _syncWithNativeBridge() {
    final bridge = AttuneEngineBridge.instance;
    bridge.masterFxSetEnabled(_enabled);
    bridge.masterFxSetPreampDb(_preampDb);
    bridge.masterFxSetAllBands(_bandGainsDb);
    bridge.masterFxSetBass(_bassEnabled, _bassGainDb, _bassFreqHz);
    bridge.masterFxSetClarity(_clarityEnabled, _clarityWet);
    bridge.masterFxSetStereoWidth(_stereoWidth);
  }

  void _probeExternalDsp() {
    try {
      final status = AttuneEngineBridge.instance.probeExternalDsp();
      if (status != null) {
        _externalDspDetected = status['detected'] as bool? ?? false;
        _externalDspName = status['engine_name'] as String? ?? 'None';
        _externalDspDetails = status['details'] as String? ?? '';
        _externalDspRecommendedAction = status['recommended_action'] as String? ?? '';
      }
    } catch (e) {
      debugPrint('[EqualizerService] External DSP probe error: $e');
    }
  }

  void setEnabled(bool value) {
    if (_enabled == value) return;
    _enabled = value;
    AttuneEngineBridge.instance.masterFxSetEnabled(value);
    notifyListeners();
    _saveToDisk();
  }

  void setPreampDb(double value) {
    final clamped = value.clamp(-12.0, 6.0);
    if ((_preampDb - clamped).abs() < 0.05) return;
    _preampDb = clamped;
    AttuneEngineBridge.instance.masterFxSetPreampDb(clamped);
    notifyListeners();
    _saveToDisk();
  }

  void setBandGain(int index, double gainDb) {
    if (index < 0 || index >= 10) return;
    final clamped = gainDb.clamp(-12.0, 12.0);
    if ((_bandGainsDb[index] - clamped).abs() < 0.05) return;

    _bandGainsDb[index] = clamped;
    _activePreset = 'Custom';
    AttuneEngineBridge.instance.masterFxSetBandGain(index, clamped);
    notifyListeners();
    _saveToDisk();
  }

  void applyPreset(String presetName) {
    if (!kEqualizerPresets.containsKey(presetName)) return;
    final gains = kEqualizerPresets[presetName]!;
    _bandGainsDb = List<double>.from(gains);
    _activePreset = presetName;
    AttuneEngineBridge.instance.masterFxSetAllBands(_bandGainsDb);
    notifyListeners();
    _saveToDisk();
  }

  void setBass({bool? enabled, double? gainDb, double? freqHz}) {
    if (enabled != null) _bassEnabled = enabled;
    if (gainDb != null) _bassGainDb = gainDb.clamp(0.0, 14.0);
    if (freqHz != null) _bassFreqHz = freqHz.clamp(40.0, 150.0);

    AttuneEngineBridge.instance.masterFxSetBass(_bassEnabled, _bassGainDb, _bassFreqHz);
    notifyListeners();
    _saveToDisk();
  }

  void setClarity({bool? enabled, double? wet}) {
    if (enabled != null) _clarityEnabled = enabled;
    if (wet != null) _clarityWet = wet.clamp(0.0, 0.40);

    AttuneEngineBridge.instance.masterFxSetClarity(_clarityEnabled, _clarityWet);
    notifyListeners();
    _saveToDisk();
  }

  void setStereoWidth(double width) {
    final clamped = width.clamp(0.0, 2.5);
    if ((_stereoWidth - clamped).abs() < 0.01) return;
    _stereoWidth = clamped;
    AttuneEngineBridge.instance.masterFxSetStereoWidth(clamped);
    notifyListeners();
    _saveToDisk();
  }

  void setBypassExternalDspWarning(bool bypass) {
    _bypassExternalDspWarning = bypass;
    notifyListeners();
    _saveToDisk();
  }

  void resetToFlat() {
    applyPreset('Flat');
    _preampDb = -3.0;
    _bassEnabled = false;
    _clarityEnabled = false;
    _stereoWidth = 1.0;
    _syncWithNativeBridge();
    notifyListeners();
    _saveToDisk();
  }
}
