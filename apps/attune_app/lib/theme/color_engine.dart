import 'package:flutter/material.dart';
import 'package:material_color_utilities/material_color_utilities.dart';

/// Material You Dynamic Color Engine implementing Section 2.1 of 05_UI_DESIGN_SPEC.md:
/// - HCT space palette generation from album artwork seeds
/// - Dynamic real-time SchemeVibrant derivation
/// - Real-time transition-synchronized palette morphing (Track A -> Track B)
/// - Fallback palette (#2DD4BF Mint/Teal, #A855F7 Electric Violet, #FB7185 Coral Rose, #090A10 Canvas Base)
class AttuneColorEngine {
  // Fallback HCT Seeds from Design Spec
  static const Color fallbackPrimary = Color(0xFF2DD4BF);   // Prismatic Mint/Teal
  static const Color fallbackSecondary = Color(0xFFA855F7); // Electric Violet
  static const Color fallbackTertiary = Color(0xFFFB7185);  // Coral Rose
  static const Color fallbackCanvas = Color(0xFF090A10);    // Deep Canvas Base

  /// Generates a complete Material 3 ColorScheme from a 32-bit ARGB seed
  static ColorScheme createSchemeFromSeed(Color seed, {Brightness brightness = Brightness.dark}) {
    final hct = Hct.fromInt(seed.toARGB32());
    final scheme = SchemeVibrant(
      sourceColorHct: hct,
      isDark: brightness == Brightness.dark,
      contrastLevel: 0.0,
    );

    return ColorScheme(
      brightness: brightness,
      primary: Color(scheme.primary),
      onPrimary: Color(scheme.onPrimary),
      primaryContainer: Color(scheme.primaryContainer),
      onPrimaryContainer: Color(scheme.onPrimaryContainer),
      secondary: Color(scheme.secondary),
      onSecondary: Color(scheme.onSecondary),
      secondaryContainer: Color(scheme.secondaryContainer),
      onSecondaryContainer: Color(scheme.onSecondaryContainer),
      tertiary: Color(scheme.tertiary),
      onTertiary: Color(scheme.onTertiary),
      tertiaryContainer: Color(scheme.tertiaryContainer),
      onTertiaryContainer: Color(scheme.onTertiaryContainer),
      error: Color(scheme.error),
      onError: Color(scheme.onError),
      surface: brightness == Brightness.dark ? fallbackCanvas : Colors.white,
      onSurface: Color(scheme.onSurface),
      surfaceContainerHigh: Color(scheme.surfaceContainerHigh),
      surfaceContainerLowest: Color(scheme.surfaceContainerLowest),
      surfaceTint: Color(scheme.surfaceTint),
      outline: Color(scheme.outline),
      outlineVariant: Color(scheme.outlineVariant),
    );
  }

  /// Default Attune Liquid Material Glass dark ColorScheme
  static ColorScheme get defaultDarkScheme => createSchemeFromSeed(fallbackPrimary);

  /// Transition-synchronized palette morphing:
  /// Linearly interpolates the entire ColorScheme from Track A to Track B
  /// as the DualDeckMixer progresses across the transition window.
  static ColorScheme lerpSchemes(ColorScheme a, ColorScheme b, double progress) {
    return ColorScheme.lerp(a, b, progress.clamp(0.0, 1.0));
  }
}
