import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

/// Typography System conforming to Section 1 of 05_UI_DESIGN_SPEC.md:
/// - Display, Headlines, Track Titles, Section Headers & Badges: Sora
/// - Body, Artist Subtitles, Queue Lists, Search & Lyrics: Plus Jakarta Sans
/// - Tabular figures enabled on timestamps and DSP metrics to prevent digit jitter
class AttuneTypography {
  // Sora (Display, Titles, Badges, Metrics)
  static TextStyle soraDisplayLarge({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.sora(
        fontSize: fontSize ?? 32,
        fontWeight: fontWeight ?? FontWeight.w700,
        letterSpacing: -0.64, // -0.02em
        color: color,
      );

  static TextStyle soraHeading({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.sora(
        fontSize: fontSize ?? 22,
        fontWeight: fontWeight ?? FontWeight.w700,
        letterSpacing: -0.44,
        color: color,
      );

  static TextStyle soraTitleLarge({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.sora(
        fontSize: fontSize ?? 22,
        fontWeight: fontWeight ?? FontWeight.w700,
        letterSpacing: -0.44, // -0.02em
        color: color,
      );

  static TextStyle soraSubheading({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.sora(
        fontSize: fontSize ?? 16,
        fontWeight: fontWeight ?? FontWeight.w600,
        letterSpacing: -0.36,
        color: color,
      );

  static TextStyle soraTitleMedium({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.sora(
        fontSize: fontSize ?? 18,
        fontWeight: fontWeight ?? FontWeight.w600,
        letterSpacing: -0.36,
        color: color,
      );

  static TextStyle soraTitleSmall({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.sora(
        fontSize: fontSize ?? 15,
        fontWeight: fontWeight ?? FontWeight.w600,
        color: color,
      );

  static TextStyle soraSectionHeader({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.sora(
        fontSize: fontSize ?? 12,
        fontWeight: fontWeight ?? FontWeight.w700,
        letterSpacing: 0.48, // +0.04em
        color: color,
      );

  static TextStyle soraBadge({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.sora(
        fontSize: fontSize ?? 11,
        fontWeight: fontWeight ?? FontWeight.w600,
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

  static TextStyle soraTabularTimestamp({Color? color, double fontSize = 12, FontWeight weight = FontWeight.w600}) =>
      soraMetric(color: color, fontSize: fontSize, weight: weight);

  // Plus Jakarta Sans (Body, Subtitles, Lyrics, Queue, Search)
  static TextStyle jakartaBody({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.plusJakartaSans(
        fontSize: fontSize ?? 14,
        fontWeight: fontWeight ?? FontWeight.w400,
        color: color,
      );

  static TextStyle jakartaBodyLarge({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.plusJakartaSans(
        fontSize: fontSize ?? 16,
        fontWeight: fontWeight ?? FontWeight.w400,
        color: color,
      );

  static TextStyle jakartaBodyMedium({Color? color, double? fontSize, FontWeight? weight}) =>
      GoogleFonts.plusJakartaSans(
        fontSize: fontSize ?? 14,
        fontWeight: weight ?? FontWeight.w400,
        color: color,
      );

  static TextStyle jakartaBodySmall({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.plusJakartaSans(
        fontSize: fontSize ?? 12,
        fontWeight: fontWeight ?? FontWeight.w500,
        color: color,
      );

  static TextStyle jakartaArtistSubtitle({Color? color, double? fontSize, FontWeight? fontWeight}) => GoogleFonts.plusJakartaSans(
        fontSize: fontSize ?? 15,
        fontWeight: fontWeight ?? FontWeight.w500,
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
