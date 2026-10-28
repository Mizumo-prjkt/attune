import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../services/engine_bridge.dart';
import '../theme/color_engine.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Full-screen Liquid Material Glass About Stage (`/about`).
/// Transparently displays credits, AI pair programming disclosure,
/// donation status (coming soon), MIT license and third-party notices,
/// and dynamically etched compiler / build environment metadata.
class AboutStage extends StatefulWidget {
  final bool isStandaloneActivity;

  const AboutStage({super.key, this.isStandaloneActivity = false});

  @override
  State<AboutStage> createState() => _AboutStageState();
}

class _AboutStageState extends State<AboutStage> {
  Map<String, dynamic>? _buildInfo;
  bool _isLoadingBuildInfo = true;

  @override
  void initState() {
    super.initState();
    _loadBuildInfo();
  }

  Future<void> _loadBuildInfo() async {
    try {
      final jsonString = await rootBundle.loadString('assets/build_info.json');
      final data = json.decode(jsonString);
      if (mounted && data is Map<String, dynamic>) {
        setState(() {
          _buildInfo = data;
          _isLoadingBuildInfo = false;
        });
        return;
      }
    } catch (_) {
      // Fallback if build_info.json is not yet generated or running in raw dev mode
    }
    if (mounted) {
      setState(() {
        _isLoadingBuildInfo = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final engineVersion = AttuneEngineBridge.instance.version;

    final appVersion = _buildInfo?['version'] as String? ?? '0.1.0';
    final localVersion = _buildInfo?['localversion'] as String? ?? '';
    final fullVersionString = localVersion.isNotEmpty ? 'v$appVersion$localVersion' : 'v$appVersion';
    final gitCommit = _buildInfo?['git_commit'] as String? ?? 'dev';
    final buildTimestamp = _buildInfo?['build_timestamp'] as String? ?? '';
    final compilerRust = _buildInfo?['compiler_rustc'] as String? ?? '';
    final compilerFlutter = _buildInfo?['compiler_flutter'] as String? ?? '';

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Stack(
        children: [
          // Ambient living background
          const AmbientMeshBackground(),

          SafeArea(
            child: Column(
              children: [
                // 1. Top Navigation Bar
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16.0, vertical: 8.0),
                  child: Row(
                    children: [
                      LiquidGlassContainer(
                        borderRadius: 20,
                        padding: EdgeInsets.zero,
                        surfaceAlpha: 0.15,
                        child: IconButton(
                          icon: const Icon(Icons.arrow_back_rounded, color: Colors.white),
                          tooltip: 'Back',
                          onPressed: () {
                            if (Navigator.of(context).canPop()) {
                              Navigator.of(context).pop();
                            } else if (widget.isStandaloneActivity) {
                              SystemNavigator.pop();
                            }
                          },
                        ),
                      ),
                      const SizedBox(width: 16),
                      Text(
                        'About Attune',
                        style: AttuneTypography.soraTitleMedium(color: colorScheme.onSurface),
                      ),
                    ],
                  ),
                ),

                // 2. Scrollable Body
                Expanded(
                  child: ListView(
                    physics: const BouncingScrollPhysics(),
                    padding: const EdgeInsets.symmetric(horizontal: 20.0, vertical: 8.0),
                    children: [
                      // HERO BANNER
                      LiquidGlassContainer(
                        borderRadius: 28,
                        surfaceAlpha: 0.16,
                        padding: const EdgeInsets.all(24.0),
                        child: Column(
                          children: [
                            Container(
                              width: 88,
                              height: 88,
                              padding: const EdgeInsets.all(10),
                              decoration: BoxDecoration(
                                shape: BoxShape.circle,
                                color: colorScheme.primary.withValues(alpha: 0.15),
                                border: Border.all(
                                  color: colorScheme.primary.withValues(alpha: 0.35),
                                  width: 1.5,
                                ),
                                boxShadow: [
                                  BoxShadow(
                                    color: colorScheme.primary.withValues(alpha: 0.25),
                                    blurRadius: 24,
                                    spreadRadius: 2,
                                  ),
                                ],
                              ),
                              child: Image.asset(
                                'assets/icon/icon_transparent.png',
                                fit: BoxFit.contain,
                              ),
                            ),
                            const SizedBox(height: 16),
                            Text(
                              'Attune',
                              style: AttuneTypography.soraDisplayLarge(
                                fontSize: 28,
                                color: colorScheme.onSurface,
                              ),
                            ),
                            const SizedBox(height: 6),
                            Text(
                              'Intelligent Audio-Profile Driven Transition Engine',
                              textAlign: TextAlign.center,
                              style: AttuneTypography.jakartaBody(
                                fontSize: 13,
                                color: colorScheme.onSurface.withValues(alpha: 0.7),
                              ),
                            ),
                            const SizedBox(height: 12),
                            Wrap(
                              alignment: WrapAlignment.center,
                              spacing: 8,
                              runSpacing: 8,
                              children: [
                                _buildBadge(
                                  label: fullVersionString,
                                  color: colorScheme.primary,
                                  icon: Icons.tag_rounded,
                                ),
                                _buildBadge(
                                  label: 'Core v$engineVersion',
                                  color: colorScheme.secondary,
                                  icon: Icons.memory_rounded,
                                ),
                                _buildBadge(
                                  label: 'Git #$gitCommit',
                                  color: Colors.white70,
                                  icon: Icons.commit_rounded,
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),

                      const SizedBox(height: 24),

                      // SECTION 1: CREDITS & ARCHITECTURE
                      _buildSectionTitle('CREDITS & ARCHITECTURE', colorScheme),
                      LiquidGlassContainer(
                        borderRadius: 22,
                        surfaceAlpha: 0.12,
                        padding: const EdgeInsets.all(18.0),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _buildCreditRow(
                              role: 'Author & Project Creator',
                              name: 'Mizumo-prjkt',
                              icon: Icons.person_outline_rounded,
                              colorScheme: colorScheme,
                            ),
                            const Divider(height: 24, color: Colors.white10),
                            _buildCreditRow(
                              role: 'Audio Transition & DSP Engine',
                              name: 'Attune Native Dual-Deck Architecture',
                              icon: Icons.graphic_eq_rounded,
                              colorScheme: colorScheme,
                              description:
                                  '5-Layer Audio Profiling (f0 Sustain, 12-bin CQT Chroma, Beat Grid, 4-Band Energy, Instrument/Vibe Aggression Index), Dynamic Outro/Intro Cue Zones, Attune Studio Master FX Rack (10-Band Biquad EQ, Pure Bass+, Clarity, Stereo Expander, Soft Limiter).',
                            ),
                            const Divider(height: 24, color: Colors.white10),
                            _buildCreditRow(
                              role: 'Open-Source DSP & Foundations',
                              name: 'Signalsmith, Symphonia, Cpal & Hound',
                              icon: Icons.layers_outlined,
                              colorScheme: colorScheme,
                              description:
                                  'Signalsmith Stretch (C++ time-stretch/pitch-shift), Symphonia (pure Rust multimedia decoding), Hound (WAV streaming), Cpal (cross-platform audio hardware I/O), RealFFT, Rubato (resampling), and Google Flutter / Dart UI framework.',
                            ),
                          ],
                        ),
                      ),

                      const SizedBox(height: 24),

                      // SECTION 2: AI PAIR PROGRAMMING
                      _buildSectionTitle('AI PAIR PROGRAMMING INVOLVEMENT', colorScheme),
                      LiquidGlassContainer(
                        borderRadius: 22,
                        surfaceAlpha: 0.12,
                        padding: const EdgeInsets.all(18.0),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              children: [
                                Container(
                                  padding: const EdgeInsets.all(8),
                                  decoration: BoxDecoration(
                                    color: colorScheme.primary.withValues(alpha: 0.18),
                                    shape: BoxShape.circle,
                                  ),
                                  child: Icon(Icons.auto_awesome_rounded, color: colorScheme.primary, size: 20),
                                ),
                                const SizedBox(width: 12),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    children: [
                                      Text(
                                        'Google DeepMind Antigravity AI Assistant',
                                        style: AttuneTypography.soraSubheading(
                                          fontSize: 14,
                                          color: colorScheme.onSurface,
                                        ),
                                      ),
                                      Text(
                                        'Autonomous Coding & Mathematical Pair Programming',
                                        style: AttuneTypography.jakartaBody(
                                          fontSize: 12,
                                          color: colorScheme.primary.withValues(alpha: 0.8),
                                        ),
                                      ),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 12),
                            Text(
                              'Attune is engineered through transparent pair programming with Google DeepMind’s Antigravity advanced AI assistant under the Knowledge Ledger Protocol.\n\n'
                              '• Knowledge Ledgering: Every architectural decision is recorded in Architecture Decision Records (ADRs 001–033 in ledger/01_ADR_LOG.md).\n'
                              '• Zero Hardcoding Mandate: Algorithmic parameters, DSP lookahead gates, and hardware bounds derive from real audio analysis and live OS measurements rather than arbitrary constant tables.\n'
                              '• Empirical Transition Lab: Benchmark song pairs and tuning observations are logged continuously in ledger/04_TRANSITION_LAB_LOG.md.',
                              style: AttuneTypography.jakartaBody(
                                fontSize: 13,
                                color: colorScheme.onSurface.withValues(alpha: 0.78),
                              ),
                            ),
                          ],
                        ),
                      ),

                      const SizedBox(height: 24),

                      // SECTION 3: DONATIONS (COMING SOON)
                      _buildSectionTitle('SUPPORT & DONATIONS', colorScheme),
                      LiquidGlassContainer(
                        borderRadius: 22,
                        surfaceAlpha: 0.12,
                        padding: const EdgeInsets.all(18.0),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Container(
                              padding: const EdgeInsets.all(10),
                              decoration: BoxDecoration(
                                color: Colors.pinkAccent.withValues(alpha: 0.18),
                                shape: BoxShape.circle,
                              ),
                              child: const Icon(Icons.favorite_rounded, color: Colors.pinkAccent, size: 22),
                            ),
                            const SizedBox(width: 14),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Row(
                                    children: [
                                      Text(
                                        'Community Sponsorship',
                                        style: AttuneTypography.soraSubheading(
                                          fontSize: 14,
                                          color: colorScheme.onSurface,
                                        ),
                                      ),
                                      const SizedBox(width: 8),
                                      Container(
                                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                                        decoration: BoxDecoration(
                                          color: Colors.amber.withValues(alpha: 0.2),
                                          borderRadius: BorderRadius.circular(10),
                                          border: Border.all(color: Colors.amber.withValues(alpha: 0.5), width: 1),
                                        ),
                                        child: Text(
                                          'COMING SOON',
                                          style: AttuneTypography.soraBadge(
                                            fontSize: 9,
                                            color: Colors.amberAccent,
                                          ),
                                        ),
                                      ),
                                    ],
                                  ),
                                  const SizedBox(height: 6),
                                  Text(
                                    'Attune is free and open-source software. Community sponsorship avenues (GitHub Sponsors, Ko-fi, and Liberapay) are coming soon to support server infrastructure, offloading clusters, and continuous DSP research. Thank you for your support!',
                                    style: AttuneTypography.jakartaBody(
                                      fontSize: 13,
                                      color: colorScheme.onSurface.withValues(alpha: 0.72),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ],
                        ),
                      ),

                      const SizedBox(height: 24),

                      // SECTION 4: LICENSE & LEGAL NOTICE
                      _buildSectionTitle('LICENSE & LEGAL NOTICES', colorScheme),
                      LiquidGlassContainer(
                        borderRadius: 22,
                        surfaceAlpha: 0.12,
                        padding: const EdgeInsets.all(18.0),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              children: [
                                Icon(Icons.gavel_rounded, color: colorScheme.primary, size: 20),
                                const SizedBox(width: 10),
                                Text(
                                  'MIT License (Attune Player & Core)',
                                  style: AttuneTypography.soraSubheading(
                                    fontSize: 14,
                                    color: colorScheme.onSurface,
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 10),
                            Text(
                              'Attune is licensed under the MIT License:\n'
                              'Copyright (c) 2026 Mizumo-prjkt and Attune contributors.\n\n'
                              'Permission is hereby granted, free of charge, to any person obtaining a copy '
                              'of this software and associated documentation files, to deal in the Software without restriction.\n\n'
                              'NOTE ON THIRD-PARTY LIBRARIES:\n'
                              'Attune bundles and links against third-party open-source components and DSP libraries '
                              'which carry their own respective licenses, including BSD-2-Clause, BSD-3-Clause, Apache-2.0, '
                              'MIT, MPL-2.0, LGPL, and GPL. All copyrights, patents, and trademarks belong to their respective holders.',
                              style: AttuneTypography.jakartaBody(
                                fontSize: 12,
                                color: colorScheme.onSurface.withValues(alpha: 0.75),
                              ),
                            ),
                            const SizedBox(height: 14),
                            OutlinedButton.icon(
                              style: OutlinedButton.styleFrom(
                                side: BorderSide(color: colorScheme.primary.withValues(alpha: 0.4)),
                                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                                padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                              ),
                              icon: Icon(Icons.description_outlined, size: 18, color: colorScheme.primary),
                              label: Text(
                                'View Complete Open-Source Licenses',
                                style: AttuneTypography.soraTitleSmall(
                                  fontSize: 13,
                                  color: colorScheme.primary,
                                ),
                              ),
                              onPressed: () {
                                showLicensePage(
                                  context: context,
                                  applicationName: 'Attune',
                                  applicationVersion: fullVersionString,
                                  applicationIcon: Container(
                                    width: 44,
                                    height: 44,
                                    padding: const EdgeInsets.all(6),
                                    decoration: BoxDecoration(
                                      color: colorScheme.primary.withValues(alpha: 0.15),
                                      shape: BoxShape.circle,
                                    ),
                                    child: Image.asset(
                                      'assets/icon/icon_transparent.png',
                                      fit: BoxFit.contain,
                                    ),
                                  ),
                                  applicationLegalese:
                                      'Distributed under the MIT License.\nThird-party crates and libraries are licensed under BSD-2-Clause, BSD-3-Clause, Apache-2.0, MIT, MPL-2.0, LGPL, and GPL.',
                                );
                              },
                            ),
                          ],
                        ),
                      ),

                      const SizedBox(height: 24),

                      // SECTION 5: COMPILER & BUILD ENVIRONMENT
                      _buildSectionTitle('BUILD ENVIRONMENT & COMPILER METADATA', colorScheme),
                      LiquidGlassContainer(
                        borderRadius: 22,
                        surfaceAlpha: 0.12,
                        padding: const EdgeInsets.all(18.0),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _buildEnvRow('Target OS', '${Platform.operatingSystem} (${Platform.operatingSystemVersion})'),
                            const SizedBox(height: 6),
                            _buildEnvRow('Architecture', _buildInfo?['target_arch'] as String? ?? 'x86_64 / arm64'),
                            const SizedBox(height: 6),
                            _buildEnvRow('Build Mode', _buildInfo?['build_mode'] as String? ?? (const bool.fromEnvironment('dart.vm.product') ? 'Release' : 'Debug / Dev')),
                            if (compilerRust.isNotEmpty) ...[
                              const SizedBox(height: 6),
                              _buildEnvRow('Rustc', compilerRust),
                            ],
                            if (compilerFlutter.isNotEmpty) ...[
                              const SizedBox(height: 6),
                              _buildEnvRow('Flutter Toolchain', compilerFlutter),
                            ],
                            if (buildTimestamp.isNotEmpty) ...[
                              const SizedBox(height: 6),
                              _buildEnvRow('Build Timestamp', buildTimestamp),
                            ],
                          ],
                        ),
                      ),

                      const SizedBox(height: 36),
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

  Widget _buildSectionTitle(String title, ColorScheme colorScheme) {
    return Padding(
      padding: const EdgeInsets.only(left: 4.0, bottom: 8.0),
      child: Text(
        title,
        style: AttuneTypography.soraSectionHeader(color: colorScheme.primary),
      ),
    );
  }

  Widget _buildBadge({required String label, required Color color, required IconData icon}) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.35), width: 1),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 13, color: color),
          const SizedBox(width: 5),
          Text(
            label,
            style: AttuneTypography.soraBadge(color: color, fontSize: 11),
          ),
        ],
      ),
    );
  }

  Widget _buildCreditRow({
    required String role,
    required String name,
    required IconData icon,
    required ColorScheme colorScheme,
    String? description,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Icon(icon, size: 18, color: colorScheme.primary),
            const SizedBox(width: 8),
            Text(
              role,
              style: AttuneTypography.soraBadge(color: colorScheme.primary, fontSize: 11),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Text(
          name,
          style: AttuneTypography.soraSubheading(fontSize: 15, color: colorScheme.onSurface),
        ),
        if (description != null) ...[
          const SizedBox(height: 4),
          Text(
            description,
            style: AttuneTypography.jakartaBody(
              fontSize: 12,
              color: colorScheme.onSurface.withValues(alpha: 0.7),
            ),
          ),
        ],
      ],
    );
  }

  Widget _buildEnvRow(String key, String value) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 110,
          child: Text(
            key,
            style: AttuneTypography.jakartaBody(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: Colors.white70,
            ),
          ),
        ),
        Expanded(
          child: Text(
            value,
            style: AttuneTypography.jakartaBody(
              fontSize: 12,
              color: Colors.white54,
            ),
          ),
        ),
      ],
    );
  }
}

/// Floating Liquid Material Glass dialog variant for quick invocation
void showAttuneAboutDialog(BuildContext context) {
  final colorScheme = Theme.of(context).colorScheme;
  final engineVersion = AttuneEngineBridge.instance.version;

  showDialog(
    context: context,
    builder: (dialogContext) {
      return Dialog(
        backgroundColor: Colors.transparent,
        insetPadding: const EdgeInsets.symmetric(horizontal: 24, vertical: 24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 480),
          child: LiquidGlassContainer(
            borderRadius: 28,
            surfaceAlpha: 0.18,
            padding: const EdgeInsets.all(24.0),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Container(
                  width: 64,
                  height: 64,
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: colorScheme.primary.withValues(alpha: 0.15),
                    border: Border.all(color: colorScheme.primary.withValues(alpha: 0.35)),
                  ),
                  child: Image.asset('assets/icon/icon_transparent.png', fit: BoxFit.contain),
                ),
                const SizedBox(height: 12),
                Text(
                  'Attune',
                  style: AttuneTypography.soraHeading(color: colorScheme.onSurface),
                ),
                const SizedBox(height: 4),
                Text(
                  'Version 0.1.0 • Native Core v$engineVersion',
                  style: AttuneTypography.jakartaBody(fontSize: 12, color: colorScheme.primary),
                ),
                const SizedBox(height: 14),
                Text(
                  'Open-source, intelligent music player with a DJ-grade, audio-profile-driven transition and remixing engine.\n\n'
                  'Developed by Mizumo-prjkt in collaborative pair programming with Google DeepMind Antigravity AI assistant.',
                  textAlign: TextAlign.center,
                  style: AttuneTypography.jakartaBody(fontSize: 13, color: colorScheme.onSurface.withValues(alpha: 0.8)),
                ),
                const SizedBox(height: 20),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    TextButton(
                      onPressed: () => Navigator.of(dialogContext).pop(),
                      child: Text(
                        'Close',
                        style: AttuneTypography.soraSubheading(fontSize: 13, color: Colors.white70),
                      ),
                    ),
                    FilledButton.icon(
                      style: FilledButton.styleFrom(
                        backgroundColor: colorScheme.primary,
                        foregroundColor: colorScheme.onPrimary,
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
                      ),
                      icon: const Icon(Icons.info_outline_rounded, size: 16),
                      label: Text(
                        'Full About & Credits',
                        style: AttuneTypography.soraSubheading(fontSize: 13, color: colorScheme.onPrimary),
                      ),
                      onPressed: () {
                        Navigator.of(dialogContext).pop();
                        Navigator.of(context).push(
                          MaterialPageRoute(
                            builder: (_) => const AboutStage(),
                          ),
                        );
                      },
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      );
    },
  );
}
