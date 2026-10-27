import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

/// Typography System conforming to Section 1 of 05_UI_DESIGN_SPEC.md:
/// - Display, Headlines, Track Titles, Section Headers & Badges: Sora
/// - Body, Artist Subtitles, Queue Lists, Search & Lyrics: Plus Jakarta Sans
/// - Tabular figures enabled on timestamps and DSP metrics to prevent digit jitter
class AttuneTypography {
  // Sora (Display, Titles, Badges, Metrics)
  static TextStyle soraDisplayLarge({Color? color}) => GoogleFonts.sora(
        fontSize: 32,
        fontWeight: FontWeight.w700,
        letterSpacing: -0.64, // -0.02em
        color: color,
      );

  static TextStyle soraTitleLarge({Color? color}) => GoogleFonts.sora(
        fontSize: 22,
        fontWeight: FontWeight.w700,
        letterSpacing: -0.44, // -0.02em
        color: color,
      );

  static TextStyle soraTitleMedium({Color? color}) => GoogleFonts.sora(
        fontSize: 18,
        fontWeight: FontWeight.w600,
        letterSpacing: -0.36,
        color: color,
      );

  static TextStyle soraTitleSmall({Color? color}) => GoogleFonts.sora(
        fontSize: 15,
        fontWeight: FontWeight.w600,
        color: color,
      );

  static TextStyle soraSectionHeader({Color? color}) => GoogleFonts.sora(
        fontSize: 12,
        fontWeight: FontWeight.w700,
        letterSpacing: 0.48, // +0.04em
        color: color,
      );

  static TextStyle soraBadge({Color? color}) => GoogleFonts.sora(
        fontSize: 11,
        fontWeight: FontWeight.w600,
        letterSpacing: 0.22,
        color: color,
      );

  /// Tabular figures for timestamps (e.g. 02:44), countdowns, and BPM metrics (171.4 BPM)
  static TextStyle soraMetric({Color? color, double fontSize = 14, FontWeight weight = FontWeight.w600}) =>
      GoogleFonts.sora(
        fontSize: fontSize,
        fontWeight: weight,
        fontFeatures: const [FontFeature.tabularFigures()],
        color: color,
      );

  // Plus Jakarta Sans (Body, Subtitles, Lyrics, Queue, Search)
  static TextStyle jakartaBodyLarge({Color? color}) => GoogleFonts.plusJakartaSans(
        fontSize: 16,
        fontWeight: FontWeight.w400,
        color: color,
      );

  static TextStyle jakartaBodyMedium({Color? color, FontWeight weight = FontWeight.w400}) =>
      GoogleFonts.plusJakartaSans(
        fontSize: 14,
        fontWeight: weight,
        color: color,
      );

  static TextStyle jakartaBodySmall({Color? color}) => GoogleFonts.plusJakartaSans(
        fontSize: 12,
        fontWeight: FontWeight.w500,
        color: color,
      );

  static TextStyle jakartaArtistSubtitle({Color? color}) => GoogleFonts.plusJakartaSans(
        fontSize: 15,
        fontWeight: FontWeight.w500,
        color: color,
      );

  static TextStyle jakartaLyricsActive({Color? color}) => GoogleFonts.plusJakartaSans(
        fontSize: 24,
        fontWeight: FontWeight.w700,
        height: 1.35,
        color: color,
      );

  static TextStyle jakartaLyricsInactive({Color? color}) => GoogleFonts.plusJakartaSans(
        fontSize: 20,
        fontWeight: FontWeight.w500,
        height: 1.4,
        color: color,
      );
}
