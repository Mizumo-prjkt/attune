import 'package:flutter/material.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// SynchronizedLyricsStage implementing Stage 3 of 05_UI_DESIGN_SPEC.md:
/// Walled garden music algorithm-style scrolling lyrics with blurred glass active singing card
/// and Instrumental Wave indicator.
class SynchronizedLyricsStage extends StatefulWidget {
  const SynchronizedLyricsStage({super.key});

  @override
  State<SynchronizedLyricsStage> createState() => _SynchronizedLyricsStageState();
}

class _SynchronizedLyricsStageState extends State<SynchronizedLyricsStage> {
  int _activeLineIndex = 3;

  final List<String> _lyrics = [
    'Purei bakka de mou',
    'Iyana koto bakka da na',
    'Kimi no koto shiranai',
    'Ra - bi - tto - hoo - ru!',
    'Kotoba ni naranai kimochi dake',
    'Zutto kakaete hashitta',
    'Kimi ga waratta shunkan ni',
    'Sekai ga irodzuite iku',
  ];

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return ListView.builder(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.symmetric(horizontal: 24.0, vertical: 32.0),
      itemCount: _lyrics.length + 1,
      itemBuilder: (context, index) {
        if (index == 0) {
          // Instrumental Wave Indicator pill at beginning of section
          return Center(
            child: Padding(
              padding: const EdgeInsets.only(bottom: 28.0),
              child: LiquidGlassContainer(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                borderRadius: 16,
                surfaceAlpha: 0.12,
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.waves_rounded, size: 16, color: colorScheme.secondary),
                    const SizedBox(width: 8),
                    Text(
                      'Instrumental Wave • Intro Synth Riff',
                      style: AttuneTypography.soraBadge(color: colorScheme.secondary),
                    ),
                  ],
                ),
              ),
            ),
          );
        }

        final lineIndex = index - 1;
        final isActive = lineIndex == _activeLineIndex;
        final text = _lyrics[lineIndex];

        return GestureDetector(
          onTap: () => setState(() => _activeLineIndex = lineIndex),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 14.0),
            child: isActive
                ? LiquidGlassContainer(
                    padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 16),
                    borderRadius: 20,
                    surfaceAlpha: 0.22,
                    tintOverride: colorScheme.primary.withValues(alpha: 0.20),
                    child: Text(
                      text,
                      style: AttuneTypography.jakartaLyricsActive(color: Colors.white),
                    ),
                  )
                : Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 12.0),
                    child: Text(
                      text,
                      style: AttuneTypography.jakartaLyricsInactive(
                        color: colorScheme.onSurface.withValues(alpha: 0.40),
                      ),
                    ),
                  ),
          ),
        );
      },
    );
  }
}
