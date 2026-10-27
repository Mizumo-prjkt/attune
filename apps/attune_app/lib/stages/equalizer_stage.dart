import 'package:flutter/material.dart';
import '../services/equalizer_service.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Full-screen Attune Studio Master FX Rack and Equalizer Stage
class EqualizerStage extends StatefulWidget {
  const EqualizerStage({super.key});

  @override
  State<EqualizerStage> createState() => _EqualizerStageState();
}

class _EqualizerStageState extends State<EqualizerStage> {
  final EqualizerService _eq = EqualizerService.instance;

  @override
  void initState() {
    super.initState();
    _eq.addListener(_onServiceChanged);
  }

  @override
  void dispose() {
    _eq.removeListener(_onServiceChanged);
    super.dispose();
  }

  void _onServiceChanged() {
    if (mounted) setState(() {});
  }

  String _formatFreq(double freq) {
    if (freq >= 1000) {
      final k = freq / 1000;
      return k == k.roundToDouble() ? '${k.toInt()}k' : '${k.toStringAsFixed(1)}k';
    }
    return '${freq.toInt()}';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Stack(
        children: [
          // Background Liquid Glass ambient atmosphere
          Container(
            decoration: const BoxDecoration(
              gradient: RadialGradient(
                center: Alignment(-0.6, -0.7),
                radius: 1.4,
                colors: [
                  Color(0xFF1E1035),
                  Color(0xFF0D0B18),
                  Color(0xFF05050A),
                ],
              ),
            ),
          ),

          SafeArea(
            child: Column(
              children: [
                // Top App Bar
                _buildHeader(context, colorScheme),

                // Main Scrollable Mastering Rack
                Expanded(
                  child: ListView(
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                    children: [
                      // External DSP Conflict Warning Banner
                      if (_eq.externalDspDetected && !_eq.bypassExternalDspWarning) ...[
                        _buildExternalDspWarningCard(context, colorScheme),
                        const SizedBox(height: 16),
                      ],

                      // Preamp Headroom Pad & Master Switch
                      _buildPreampCard(context, colorScheme),
                      const SizedBox(height: 16),

                      // Presets Bar
                      _buildPresetsSelector(context, colorScheme),
                      const SizedBox(height: 16),

                      // 10-Band Graphic EQ Rack & Curve Visualizer
                      _buildGraphicEqCard(context, colorScheme),
                      const SizedBox(height: 16),

                      // Attune Pure Bass+ Sub-Harmonic Synthesizer
                      _buildAttuneBassCard(context, colorScheme),
                      const SizedBox(height: 16),

                      // Attune Clarity & Vocal Air Harmonic Exciter
                      _buildAttuneClarityCard(context, colorScheme),
                      const SizedBox(height: 16),

                      // Stereo Field Expander / Differential Surround
                      _buildStereoExpanderCard(context, colorScheme),
                      const SizedBox(height: 32),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildHeader(BuildContext context, ColorScheme colorScheme) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.3),
        border: const Border(bottom: BorderSide(color: Colors.white10)),
      ),
      child: Row(
        children: [
          IconButton(
            icon: const Icon(Icons.arrow_back_ios_new_rounded, color: Colors.white, size: 20),
            tooltip: 'Back',
            onPressed: () {
              if (Navigator.of(context).canPop()) {
                Navigator.of(context).pop();
              } else {
                Navigator.of(context).pushReplacementNamed('/');
              }
            },
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Equalizer & Master FX',
                  style: AttuneTypography.soraTitleMedium(color: Colors.white),
                ),
                Text(
                  'Studio-Grade 6-Stage Mastering Rack',
                  style: AttuneTypography.jakartaBodySmall(color: Colors.white60),
                ),
              ],
            ),
          ),
          TextButton.icon(
            icon: const Icon(Icons.restart_alt_rounded, color: Colors.white70, size: 18),
            label: Text('Reset', style: AttuneTypography.jakartaBodySmall(color: Colors.white70)),
            onPressed: () => _eq.resetToFlat(),
          ),
          const SizedBox(width: 8),
          Transform.scale(
            scale: 0.85,
            child: Switch(
              value: _eq.enabled,
              activeThumbColor: colorScheme.primary,
              onChanged: (v) => _eq.setEnabled(v),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildExternalDspWarningCard(BuildContext context, ColorScheme colorScheme) {
    return LiquidGlassContainer(
      borderRadius: 16,
      surfaceAlpha: 0.16,
      borderColor: Colors.amber.withValues(alpha: 0.35),
      child: Container(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(16),
          border: Border.all(color: Colors.amber.withValues(alpha: 0.4)),
          gradient: LinearGradient(
            colors: [
              Colors.amber.withValues(alpha: 0.15),
              Colors.amber.withValues(alpha: 0.05),
            ],
          ),
        ),
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.warning_amber_rounded, color: Colors.amberAccent, size: 24),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    '${_eq.externalDspName} Detected',
                    style: AttuneTypography.soraTitleSmall(color: Colors.amberAccent),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text(
              _eq.externalDspDetails.isNotEmpty
                  ? _eq.externalDspDetails
                  : 'An external audio DSP engine is active on this system. Running both engines concurrently may cause double-equalization or phase distortion.',
              style: AttuneTypography.jakartaBodySmall(color: Colors.white70),
            ),
            const SizedBox(height: 12),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                OutlinedButton(
                  style: OutlinedButton.styleFrom(
                    side: BorderSide(color: Colors.amber.withValues(alpha: 0.4)),
                    foregroundColor: Colors.white,
                  ),
                  onPressed: () => _eq.setBypassExternalDspWarning(true),
                  child: const Text('Enable Built-in FX Anyway'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildPreampCard(BuildContext context, ColorScheme colorScheme) {
    final isNegative = _eq.preampDb < 0;
    final isPositive = _eq.preampDb > 0;
    final sign = isPositive ? '+' : '';
    final preampText = '$sign${_eq.preampDb.toStringAsFixed(1)} dB';

    return LiquidGlassContainer(
      borderRadius: 20,
      surfaceAlpha: 0.12,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    Icon(Icons.volume_up_rounded, color: colorScheme.primary, size: 20),
                    const SizedBox(width: 8),
                    Text(
                      'Pre-Amp Headroom Pad',
                      style: AttuneTypography.soraTitleSmall(color: Colors.white),
                    ),
                  ],
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                  decoration: BoxDecoration(
                    color: isPositive
                        ? Colors.red.withValues(alpha: 0.25)
                        : isNegative
                            ? Colors.green.withValues(alpha: 0.2)
                            : Colors.white10,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    preampText,
                    style: AttuneTypography.jakartaBodySmall(
                      color: isPositive ? Colors.redAccent : Colors.white,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              'Compensates for boosted EQ bands to maintain transparent headroom and prevent digital clipping before output.',
              style: AttuneTypography.jakartaBodySmall(color: Colors.white60),
            ),
            const SizedBox(height: 12),
            SliderTheme(
              data: SliderTheme.of(context).copyWith(
                activeTrackColor: colorScheme.primary,
                thumbColor: Colors.white,
                inactiveTrackColor: Colors.white12,
                trackHeight: 4,
              ),
              child: Slider(
                value: _eq.preampDb,
                min: -12.0,
                max: 6.0,
                divisions: 36,
                onChanged: _eq.enabled ? (v) => _eq.setPreampDb(v) : null,
              ),
            ),
            if (isPositive)
              Padding(
                padding: const EdgeInsets.only(top: 4),
                child: Text(
                  'Caution: Positive pre-amp gain may cause limiter saturation or filter clipping.',
                  style: AttuneTypography.jakartaBodySmall(color: Colors.redAccent),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildPresetsSelector(BuildContext context, ColorScheme colorScheme) {
    final presetNames = kEqualizerPresets.keys.toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(left: 4, bottom: 8),
          child: Text(
            'PRESETS',
            style: AttuneTypography.jakartaBodySmall(color: Colors.white54),
          ),
        ),
        SizedBox(
          height: 38,
          child: ListView.separated(
            scrollDirection: Axis.horizontal,
            itemCount: presetNames.length,
            separatorBuilder: (context, index) => const SizedBox(width: 8),
            itemBuilder: (ctx, idx) {
              final name = presetNames[idx];
              final isSelected = _eq.activePreset == name;
              return InkWell(
                borderRadius: BorderRadius.circular(19),
                onTap: _eq.enabled ? () => _eq.applyPreset(name) : null,
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 200),
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(19),
                    gradient: isSelected
                        ? LinearGradient(
                            colors: [
                              colorScheme.primary,
                              colorScheme.secondary,
                            ],
                          )
                        : null,
                    color: isSelected ? null : Colors.white.withValues(alpha: 0.08),
                    border: Border.all(
                      color: isSelected ? Colors.transparent : Colors.white12,
                    ),
                  ),
                  child: Text(
                    name,
                    style: AttuneTypography.jakartaBodySmall(
                      color: isSelected ? Colors.white : Colors.white70,
                    ),
                  ),
                ),
              );
            },
          ),
        ),
      ],
    );
  }

  Widget _buildGraphicEqCard(BuildContext context, ColorScheme colorScheme) {
    return LiquidGlassContainer(
      borderRadius: 20,
      surfaceAlpha: 0.12,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    Icon(Icons.graphic_eq_rounded, color: colorScheme.secondary, size: 20),
                    const SizedBox(width: 8),
                    Text(
                      '10-Band Graphic Equalizer',
                      style: AttuneTypography.soraTitleSmall(color: Colors.white),
                    ),
                  ],
                ),
                Text(
                  _eq.activePreset,
                  style: AttuneTypography.jakartaBodySmall(color: colorScheme.secondary),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // EQ Curve Visualizer Canvas
            SizedBox(
              height: 100,
              width: double.infinity,
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: CustomPaint(
                  painter: _EqCurvePainter(
                    bandGains: _eq.bandGainsDb,
                    lineColor: colorScheme.primary,
                    fillColor: colorScheme.primary.withValues(alpha: 0.2),
                  ),
                ),
              ),
            ),
            const SizedBox(height: 20),

            // 10 Vertical Sliders
            SizedBox(
              height: 180,
              child: Row(
                children: List.generate(kEqualizerFrequencies.length, (idx) {
                  final freq = kEqualizerFrequencies[idx];
                  final gain = _eq.bandGainsDb[idx];
                  final freqStr = _formatFreq(freq);
                  final gainStr = gain > 0 ? '+${gain.toStringAsFixed(1)}' : gain.toStringAsFixed(1);

                  return Expanded(
                    child: Column(
                      children: [
                        // Gain readout
                        Text(
                          gainStr,
                          style: const TextStyle(
                            fontSize: 10,
                            fontFamily: 'Plus Jakarta Sans',
                            color: Colors.white70,
                          ),
                        ),
                        const SizedBox(height: 6),

                        // Vertical Slider
                        Expanded(
                          child: RotatedBox(
                            quarterTurns: 3,
                            child: SliderTheme(
                              data: SliderTheme.of(context).copyWith(
                                activeTrackColor: colorScheme.primary,
                                thumbColor: Colors.white,
                                inactiveTrackColor: Colors.white12,
                                trackHeight: 3,
                                thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                                overlayShape: const RoundSliderOverlayShape(overlayRadius: 12),
                              ),
                              child: Slider(
                                value: gain,
                                min: -12.0,
                                max: 12.0,
                                divisions: 48,
                                onChanged: _eq.enabled ? (v) => _eq.setBandGain(idx, v) : null,
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(height: 6),

                        // Frequency label
                        Text(
                          freqStr,
                          style: const TextStyle(
                            fontSize: 10,
                            fontWeight: FontWeight.w600,
                            fontFamily: 'Plus Jakarta Sans',
                            color: Colors.white54,
                          ),
                        ),
                      ],
                    ),
                  );
                }),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildAttuneBassCard(BuildContext context, ColorScheme colorScheme) {
    final gainText = '+${_eq.bassGainDb.toStringAsFixed(1)} dB';

    return LiquidGlassContainer(
      borderRadius: 20,
      surfaceAlpha: 0.12,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    Icon(Icons.speaker_group_rounded, color: Colors.deepOrangeAccent, size: 20),
                    const SizedBox(width: 8),
                    Text(
                      'Attune Pure Bass+',
                      style: AttuneTypography.soraTitleSmall(color: Colors.white),
                    ),
                  ],
                ),
                Transform.scale(
                  scale: 0.85,
                  child: Switch(
                    value: _eq.bassEnabled,
                    activeThumbColor: Colors.deepOrangeAccent,
                    onChanged: _eq.enabled ? (v) => _eq.setBass(enabled: v) : null,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              'Psychoacoustic sub-harmonic synthesizer generates 2nd/3rd harmonics (< 100 Hz) to deliver deep, physical bass on earphones without driver distortion.',
              style: AttuneTypography.jakartaBodySmall(color: Colors.white60),
            ),
            const SizedBox(height: 16),

            // Bass Gain Slider
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Bass Level', style: AttuneTypography.jakartaBodySmall(color: Colors.white70)),
                Text(gainText, style: AttuneTypography.jakartaBodySmall(color: Colors.deepOrangeAccent)),
              ],
            ),
            SliderTheme(
              data: SliderTheme.of(context).copyWith(
                activeTrackColor: Colors.deepOrangeAccent,
                thumbColor: Colors.white,
                inactiveTrackColor: Colors.white12,
                trackHeight: 3,
              ),
              child: Slider(
                value: _eq.bassGainDb,
                min: 0.0,
                max: 14.0,
                divisions: 28,
                onChanged: _eq.enabled && _eq.bassEnabled ? (v) => _eq.setBass(gainDb: v) : null,
              ),
            ),

            // Cutoff frequency chips
            const SizedBox(height: 8),
            Row(
              children: [
                Text('Target Sub Frequency: ', style: AttuneTypography.jakartaBodySmall(color: Colors.white54)),
                const SizedBox(width: 8),
                Wrap(
                  spacing: 6,
                  children: [60.0, 80.0, 100.0, 120.0].map((f) {
                    final isSel = (_eq.bassFreqHz - f).abs() < 1.0;
                    return InkWell(
                      borderRadius: BorderRadius.circular(8),
                      onTap: _eq.enabled && _eq.bassEnabled ? () => _eq.setBass(freqHz: f) : null,
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                        decoration: BoxDecoration(
                          color: isSel ? Colors.deepOrangeAccent.withValues(alpha: 0.3) : Colors.white.withValues(alpha: 0.06),
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(color: isSel ? Colors.deepOrangeAccent : Colors.white12),
                        ),
                        child: Text(
                          '${f.toInt()} Hz',
                          style: TextStyle(
                            fontSize: 11,
                            color: isSel ? Colors.white : Colors.white60,
                          ),
                        ),
                      ),
                    );
                  }).toList(),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildAttuneClarityCard(BuildContext context, ColorScheme colorScheme) {
    final wetPercent = '${(_eq.clarityWet * 100).toStringAsFixed(0)}%';

    return LiquidGlassContainer(
      borderRadius: 20,
      surfaceAlpha: 0.12,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    Icon(Icons.auto_awesome_rounded, color: Colors.cyanAccent, size: 20),
                    const SizedBox(width: 8),
                    Text(
                      'Attune Clarity & Vocal Air',
                      style: AttuneTypography.soraTitleSmall(color: Colors.white),
                    ),
                  ],
                ),
                Transform.scale(
                  scale: 0.85,
                  child: Switch(
                    value: _eq.clarityEnabled,
                    activeThumbColor: Colors.cyanAccent,
                    onChanged: _eq.enabled ? (v) => _eq.setClarity(enabled: v) : null,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              'High-shelf harmonic exciter isolates frequencies above 3.5 kHz and applies soft polynomial excitation, adding vocal sparkle, breath, and spatial air.',
              style: AttuneTypography.jakartaBodySmall(color: Colors.white60),
            ),
            const SizedBox(height: 16),

            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Clarity Exciter Intensity', style: AttuneTypography.jakartaBodySmall(color: Colors.white70)),
                Text(wetPercent, style: AttuneTypography.jakartaBodySmall(color: Colors.cyanAccent)),
              ],
            ),
            SliderTheme(
              data: SliderTheme.of(context).copyWith(
                activeTrackColor: Colors.cyanAccent,
                thumbColor: Colors.white,
                inactiveTrackColor: Colors.white12,
                trackHeight: 3,
              ),
              child: Slider(
                value: _eq.clarityWet,
                min: 0.0,
                max: 0.40,
                divisions: 40,
                onChanged: _eq.enabled && _eq.clarityEnabled ? (v) => _eq.setClarity(wet: v) : null,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStereoExpanderCard(BuildContext context, ColorScheme colorScheme) {
    final widthText = '${_eq.stereoWidth.toStringAsFixed(2)}x';
    final widthMode = _eq.stereoWidth == 0.0
        ? 'Mono'
        : _eq.stereoWidth < 1.0
            ? 'Narrow'
            : _eq.stereoWidth == 1.0
                ? 'Original Stereo'
                : 'Ultra-Wide Surround';

    return LiquidGlassContainer(
      borderRadius: 20,
      surfaceAlpha: 0.12,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    Icon(Icons.surround_sound_rounded, color: Colors.purpleAccent, size: 20),
                    const SizedBox(width: 8),
                    Text(
                      'Stereo Field Expander',
                      style: AttuneTypography.soraTitleSmall(color: Colors.white),
                    ),
                  ],
                ),
                Text(
                  '$widthText ($widthMode)',
                  style: AttuneTypography.jakartaBodySmall(color: Colors.purpleAccent),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              'Mid-Side differential matrix expansion. Frequencies below 150 Hz are strictly forced to mono to prevent destructive sub-bass phase cancellation.',
              style: AttuneTypography.jakartaBodySmall(color: Colors.white60),
            ),
            const SizedBox(height: 16),

            SliderTheme(
              data: SliderTheme.of(context).copyWith(
                activeTrackColor: Colors.purpleAccent,
                thumbColor: Colors.white,
                inactiveTrackColor: Colors.white12,
                trackHeight: 3,
              ),
              child: Slider(
                value: _eq.stereoWidth,
                min: 0.0,
                max: 2.0,
                divisions: 40,
                onChanged: _eq.enabled ? (v) => _eq.setStereoWidth(v) : null,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Smooth Bezier spline painter for 10-band frequency response curve
class _EqCurvePainter extends CustomPainter {
  final List<double> bandGains;
  final Color lineColor;
  final Color fillColor;

  _EqCurvePainter({
    required this.bandGains,
    required this.lineColor,
    required this.fillColor,
  });

  @override
  void paint(Canvas canvas, Size size) {
    if (bandGains.isEmpty) return;

    final centerY = size.height / 2.0;
    final gridPaint = Paint()
      ..color = Colors.white.withValues(alpha: 0.08)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.0;

    // Draw center 0 dB reference line
    canvas.drawLine(Offset(0, centerY), Offset(size.width, centerY), gridPaint);

    final points = <Offset>[];
    final stepX = size.width / (bandGains.length - 1);

    for (var i = 0; i < bandGains.length; i++) {
      final x = i * stepX;
      // Map gain [-12, +12] to Y [height, 0]
      final norm = (bandGains[i].clamp(-12.0, 12.0) + 12.0) / 24.0;
      final y = size.height - (norm * size.height);
      points.add(Offset(x, y));
    }

    // Build smooth cubic bezier path
    final path = Path();
    path.moveTo(points.first.dx, points.first.dy);

    for (var i = 0; i < points.length - 1; i++) {
      final p0 = points[i];
      final p1 = points[i + 1];
      final controlX = (p0.dx + p1.dx) / 2.0;
      path.cubicTo(controlX, p0.dy, controlX, p1.dy, p1.dx, p1.dy);
    }

    // Draw gradient fill
    final fillPath = Path.from(path)
      ..lineTo(size.width, centerY)
      ..lineTo(0, centerY)
      ..close();

    final fillPaint = Paint()
      ..color = fillColor
      ..style = PaintingStyle.fill;
    canvas.drawPath(fillPath, fillPaint);

    // Draw curve line
    final linePaint = Paint()
      ..color = lineColor
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.5
      ..strokeCap = StrokeCap.round;
    canvas.drawPath(path, linePaint);

    // Draw control node dots
    final dotPaint = Paint()..color = Colors.white;
    for (final p in points) {
      canvas.drawCircle(p, 3.0, dotPaint);
    }
  }

  @override
  bool shouldRepaint(covariant _EqCurvePainter oldDelegate) {
    return oldDelegate.bandGains != bandGains ||
        oldDelegate.lineColor != lineColor ||
        oldDelegate.fillColor != fillColor;
  }
}
