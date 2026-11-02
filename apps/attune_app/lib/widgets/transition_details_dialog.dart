import 'package:flutter/material.dart';
import '../models/smart_shuffle_verdict.dart';
import '../services/platform_navigation.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Shows the Transition Selection Rationale & Smart Shuffle Verdict Dialog
/// Displays the winning candidate breakdown, factor metrics, and runner-up alternatives
/// allowing the user to inspect and promote alternatives directly from Now Playing or Queue.
void showTransitionDetailsDialog(
  BuildContext context,
  PlaybackController controller,
  ColorScheme colorScheme,
) {
  int activeTab = 0; // 0: Smart Shuffle Verdict, 1: Transition DSP Recipe

  showDialog(
    context: context,
    barrierColor: Colors.black.withValues(alpha: 0.45),
    builder: (context) {
      return StatefulBuilder(
        builder: (context, setDialogState) {
          final verdict = controller.getOrComputeSmartShuffleVerdict();
          final recipe = controller.pendingRecipe;
          final archetype = controller.activeArchetype.isNotEmpty ? controller.activeArchetype : '3-Band EQ Crossfade';
          final conf = (recipe?['confidence_score'] as num?)?.toDouble() ?? (verdict.winner.matchPercentage / 100.0);
          final matchPct = (conf * 100).toInt();

          return Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 500, maxHeight: 680),
              child: LiquidGlassContainer(
                margin: const EdgeInsets.symmetric(horizontal: 20, vertical: 24),
                padding: const EdgeInsets.all(22),
                borderRadius: 28,
                surfaceAlpha: 0.94,
                tintOverride: const Color(0xF4141924),
                blurSigma: 32,
                child: Material(
                  color: Colors.transparent,
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      // Top Header Row
                      Row(
                        children: [
                          Container(
                            padding: const EdgeInsets.all(10),
                            decoration: BoxDecoration(
                              shape: BoxShape.circle,
                              color: colorScheme.primary.withValues(alpha: 0.2),
                            ),
                            child: Icon(
                              activeTab == 0 ? Icons.auto_awesome_rounded : Icons.equalizer_rounded,
                              color: colorScheme.primary,
                              size: 22,
                            ),
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  'Transition Selection Rationale',
                                  style: AttuneTypography.soraHeading(
                                    fontSize: 16,
                                    fontWeight: FontWeight.bold,
                                    color: colorScheme.onSurface,
                                  ),
                                ),
                                Text(
                                  '$archetype • $matchPct% Harmonic Alignment',
                                  style: AttuneTypography.jakartaBodySmall(
                                    color: colorScheme.onSurface.withValues(alpha: 0.7),
                                  ),
                                ),
                              ],
                            ),
                          ),
                          IconButton(
                            icon: const Icon(Icons.close_rounded, size: 20),
                            color: colorScheme.onSurface.withValues(alpha: 0.6),
                            onPressed: () => Navigator.of(context).pop(),
                          ),
                        ],
                      ),
                      const SizedBox(height: 16),

                      // Segmented Tab Selector
                      Container(
                        padding: const EdgeInsets.all(3),
                        decoration: BoxDecoration(
                          color: colorScheme.onSurface.withValues(alpha: 0.08),
                          borderRadius: BorderRadius.circular(14),
                        ),
                        child: Row(
                          children: [
                            Expanded(
                              child: _buildVerdictTabButton(
                                label: '✦ Smart Shuffle Verdict',
                                isSelected: activeTab == 0,
                                colorScheme: colorScheme,
                                onTap: () => setDialogState(() => activeTab = 0),
                              ),
                            ),
                            Expanded(
                              child: _buildVerdictTabButton(
                                label: '∿ Transition DSP Recipe',
                                isSelected: activeTab == 1,
                                colorScheme: colorScheme,
                                onTap: () => setDialogState(() => activeTab = 1),
                              ),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),

                      // Scrollable Tab View
                      Flexible(
                        child: SingleChildScrollView(
                          physics: const BouncingScrollPhysics(),
                          child: activeTab == 0
                              ? _buildSmartShuffleVerdictView(verdict, controller, colorScheme, setDialogState)
                              : _buildDspRecipeView(colorScheme),
                        ),
                      ),
                      const SizedBox(height: 18),

                      // Footer Actions
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Flexible(
                            child: TextButton.icon(
                              icon: const Icon(Icons.analytics_outlined, size: 16),
                              label: const Text(
                                'Open DSP Diagnostics',
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                              ),
                              style: TextButton.styleFrom(
                                foregroundColor: colorScheme.onSurface.withValues(alpha: 0.7),
                                textStyle: AttuneTypography.soraBadge(fontSize: 11),
                              ),
                              onPressed: () {
                                Navigator.of(context).pop();
                                PlatformNavigation.openTransitionInspector(context);
                              },
                            ),
                          ),
                          const SizedBox(width: 12),
                          LiquidGlassContainer(
                            padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 8),
                            borderRadius: 14,
                            surfaceAlpha: 0.26,
                            tintOverride: colorScheme.primary.withValues(alpha: 0.28),
                            onTap: () => Navigator.of(context).pop(),
                            child: Text(
                              'Got it',
                              style: AttuneTypography.soraBadge(
                                color: Colors.white,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          );
        },
      );
    },
  );
}

Widget _buildVerdictTabButton({
  required String label,
  required bool isSelected,
  required ColorScheme colorScheme,
  required VoidCallback onTap,
}) {
  return GestureDetector(
    onTap: onTap,
    child: Container(
      padding: const EdgeInsets.symmetric(vertical: 8),
      decoration: BoxDecoration(
        color: isSelected ? colorScheme.primary.withValues(alpha: 0.25) : Colors.transparent,
        borderRadius: BorderRadius.circular(12),
      ),
      alignment: Alignment.center,
      child: Text(
        label,
        style: AttuneTypography.soraBadge(
          fontSize: 11,
          color: isSelected ? colorScheme.primary : colorScheme.onSurface.withValues(alpha: 0.65),
          fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
        ),
      ),
    ),
  );
}

Widget _buildSmartShuffleVerdictView(
  SmartShuffleVerdict verdict,
  PlaybackController controller,
  ColorScheme colorScheme,
  void Function(void Function()) setDialogState,
) {
  final winner = verdict.winner;
  return Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      // Winner Card
      LiquidGlassContainer(
        padding: const EdgeInsets.all(14),
        borderRadius: 18,
        surfaceAlpha: 0.18,
        tintOverride: colorScheme.primary.withValues(alpha: 0.16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.stars_rounded, size: 16, color: colorScheme.primary),
                      const SizedBox(width: 6),
                      Flexible(
                        child: Text(
                          'WINNING CANDIDATE • UP NEXT',
                          style: AttuneTypography.soraBadge(
                            fontSize: 10,
                            fontWeight: FontWeight.w700,
                            color: colorScheme.primary,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  decoration: BoxDecoration(
                    color: colorScheme.primary.withValues(alpha: 0.22),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    '${winner.matchPercentage}% Match',
                    style: AttuneTypography.soraBadge(
                      fontSize: 11,
                      fontWeight: FontWeight.bold,
                      color: colorScheme.primary,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text(
              winner.track.title,
              style: AttuneTypography.soraHeading(
                fontSize: 14,
                fontWeight: FontWeight.bold,
                color: colorScheme.onSurface,
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            Text(
              winner.track.artist,
              style: AttuneTypography.jakartaBodySmall(
                fontSize: 11,
                color: colorScheme.onSurface.withValues(alpha: 0.7),
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            const SizedBox(height: 10),
            Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: Colors.black.withValues(alpha: 0.18),
                borderRadius: BorderRadius.circular(10),
              ),
              child: Text(
                verdict.detailedRationale,
                style: AttuneTypography.jakartaBodySmall(
                  fontSize: 11,
                  color: colorScheme.onSurface.withValues(alpha: 0.85),
                ),
              ),
            ),
            const SizedBox(height: 10),
            // Factor breakdown chips
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: [
                _buildVerdictFactorChip(Icons.music_note_rounded, winner.harmonicVerdict, colorScheme),
                _buildVerdictFactorChip(Icons.speed_rounded, winner.tempoVerdict, colorScheme),
                _buildVerdictFactorChip(Icons.bolt_rounded, winner.energyVerdict, colorScheme),
                _buildVerdictFactorChip(Icons.replay_rounded, winner.rotationVerdict, colorScheme),
              ],
            ),
          ],
        ),
      ),

      const SizedBox(height: 18),

      // Alternatives Section
      Row(
        children: [
          Expanded(
            child: Text(
              'Other Evaluated Candidates (${verdict.alternatives.length})',
              style: AttuneTypography.soraSubheading(
                fontSize: 12,
                fontWeight: FontWeight.bold,
                color: colorScheme.onSurface.withValues(alpha: 0.9),
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            'Runner-ups',
            style: AttuneTypography.jakartaBodySmall(
              fontSize: 10,
              color: colorScheme.onSurface.withValues(alpha: 0.5),
            ),
          ),
        ],
      ),
      const SizedBox(height: 8),

      if (verdict.alternatives.isEmpty)
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: Text(
            'No other candidates evaluated in current queue.',
            style: AttuneTypography.jakartaBodySmall(
              fontSize: 11,
              color: colorScheme.onSurface.withValues(alpha: 0.5),
            ),
          ),
        )
      else
        ...verdict.alternatives.asMap().entries.map((entry) {
          final rank = entry.key + 2;
          final alt = entry.value;
          return Container(
            margin: const EdgeInsets.only(bottom: 8),
            child: LiquidGlassContainer(
              padding: const EdgeInsets.all(10),
              borderRadius: 14,
              surfaceAlpha: 0.12,
              child: Row(
                children: [
                  Container(
                    width: 26,
                    height: 26,
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      color: colorScheme.onSurface.withValues(alpha: 0.08),
                      shape: BoxShape.circle,
                    ),
                    child: Text(
                      '#$rank',
                      style: AttuneTypography.soraBadge(
                        fontSize: 10,
                        fontWeight: FontWeight.bold,
                        color: colorScheme.onSurface.withValues(alpha: 0.7),
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Expanded(
                              child: Text(
                                alt.track.title,
                                style: AttuneTypography.soraSubheading(
                                  fontSize: 12,
                                  fontWeight: FontWeight.w600,
                                  color: colorScheme.onSurface,
                                ),
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                            const SizedBox(width: 6),
                            Container(
                              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                              decoration: BoxDecoration(
                                color: colorScheme.secondary.withValues(alpha: 0.16),
                                borderRadius: BorderRadius.circular(6),
                              ),
                              child: Text(
                                '${alt.matchPercentage}%',
                                style: AttuneTypography.soraBadge(
                                  fontSize: 9,
                                  fontWeight: FontWeight.bold,
                                  color: colorScheme.secondary,
                                ),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '${alt.track.artist} • ${alt.primaryReason}',
                          style: AttuneTypography.jakartaBodySmall(
                            fontSize: 10,
                            color: colorScheme.onSurface.withValues(alpha: 0.6),
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 8),
                  InkWell(
                    borderRadius: BorderRadius.circular(10),
                    onTap: () {
                      controller.selectAlternativeAsUpNext(alt);
                      setDialogState(() {});
                    },
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                      decoration: BoxDecoration(
                        color: colorScheme.primary.withValues(alpha: 0.18),
                        borderRadius: BorderRadius.circular(10),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(Icons.swap_vert_rounded, size: 14, color: colorScheme.primary),
                          const SizedBox(width: 4),
                          Text(
                            'Promote',
                            style: AttuneTypography.soraBadge(
                              fontSize: 10,
                              fontWeight: FontWeight.w600,
                              color: colorScheme.primary,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          );
        }),
    ],
  );
}

Widget _buildVerdictFactorChip(IconData icon, String label, ColorScheme colorScheme) {
  return Container(
    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
    decoration: BoxDecoration(
      color: colorScheme.onSurface.withValues(alpha: 0.08),
      borderRadius: BorderRadius.circular(8),
    ),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 12, color: colorScheme.primary.withValues(alpha: 0.8)),
        const SizedBox(width: 5),
        Flexible(
          child: Text(
            label,
            style: AttuneTypography.jakartaBodySmall(
              fontSize: 10,
              color: colorScheme.onSurface.withValues(alpha: 0.8),
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    ),
  );
}

Widget _buildDspRecipeView(ColorScheme colorScheme) {
  return Column(
    children: [
      _buildRationaleItem(
        Icons.equalizer_rounded,
        '3-Band EQ Crossover (180 Hz)',
        'Bass frequencies are isolated and swapped cleanly to avoid destructive low-end phase cancellation and mud.',
        colorScheme,
      ),
      const SizedBox(height: 12),
      _buildRationaleItem(
        Icons.record_voice_over_rounded,
        'Vocal Completion Protection',
        'Transition trigger waits for the active singing phrase contour to conclude, preventing overlapping vocal clash.',
        colorScheme,
      ),
      const SizedBox(height: 12),
      _buildRationaleItem(
        Icons.music_note_rounded,
        'Phrase & Tempo Grid Alignment',
        'Locked to a 4-bar downbeat phrase boundary with micro-pitch preservation.',
        colorScheme,
      ),
    ],
  );
}

Widget _buildRationaleItem(IconData icon, String title, String desc, ColorScheme colorScheme) {
  return Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Container(
        padding: const EdgeInsets.all(8),
        decoration: BoxDecoration(
          color: colorScheme.primary.withValues(alpha: 0.15),
          borderRadius: BorderRadius.circular(10),
        ),
        child: Icon(icon, size: 18, color: colorScheme.primary),
      ),
      const SizedBox(width: 12),
      Expanded(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              title,
              style: AttuneTypography.soraSubheading(
                fontSize: 13,
                fontWeight: FontWeight.w600,
                color: colorScheme.onSurface,
              ),
            ),
            const SizedBox(height: 3),
            Text(
              desc,
              style: AttuneTypography.jakartaBodySmall(
                fontSize: 11,
                color: colorScheme.onSurface.withValues(alpha: 0.7),
              ),
            ),
          ],
        ),
      ),
    ],
  );
}
