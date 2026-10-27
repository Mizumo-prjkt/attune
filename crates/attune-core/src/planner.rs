use crate::schema::{
    DeckAutomation, DspAutomationKeyframe, TrackAnalysisProfile, TransitionArchetype,
    TransitionRecipe, TransitionTiming, ZoneType,
};

pub struct TransitionPlanner;

impl Default for TransitionPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl TransitionPlanner {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates two .tap profiles and compiles a .ptr Transition Recipe
    pub fn plan_transition(
        &self,
        track_a: &TrackAnalysisProfile,
        track_b: &TrackAnalysisProfile,
    ) -> TransitionRecipe {
        let exit_a = track_a
            .cue_zones
            .iter()
            .find(|z| z.zone_type == ZoneType::EXIT)
            .or_else(|| track_a.cue_zones.last());

        let entry_b = track_b
            .cue_zones
            .iter()
            .find(|z| z.zone_type == ZoneType::ENTRY)
            .or_else(|| track_b.cue_zones.first());

        let bpm_a = track_a.global_summary.nominal_bpm.max(40.0);
        let bpm_b = track_b.global_summary.nominal_bpm.max(40.0);

        let agg_a = track_a.global_summary.mean_aggression_index;
        let agg_b = track_b.global_summary.mean_aggression_index;
        let delta_agg = (agg_a - agg_b).abs();

        // 1. Check Archetype 1: Sustained-Pitch & Formant Morph
        let sustain_a = track_a
            .cue_zones
            .iter()
            .rev()
            .flat_map(|z| z.sustained_pitch_regions.iter().rev())
            .next();

        let sustain_b = track_b
            .cue_zones
            .iter()
            .flat_map(|z| z.sustained_pitch_regions.iter())
            .next();

        if agg_a < 0.55 && agg_b < 0.55 {
            if let (Some(s_a), Some(s_b)) = (sustain_a, sustain_b) {
                let diff_cents = (s_a.mean_midi_cents - s_b.mean_midi_cents).round();
                let mod_cents = diff_cents.rem_euclid(1200.0);
                let min_dist = if mod_cents > 600.0 { 1200.0 - mod_cents } else { mod_cents };

                if min_dist <= 250.0 {
                    let pitch_shift_cents = if mod_cents > 600.0 {
                        -(1200.0 - mod_cents) as i32
                    } else {
                        mod_cents as i32
                    };

                    return self.build_archetype_1_sustain_recipe(
                        track_a,
                        track_b,
                        s_a.start_ms,
                        s_a.end_ms,
                        s_b.start_ms,
                        pitch_shift_cents,
                    );
                }
            }
        }

        // Compute folded octave tempos into upbeat range [125.0, 220.0]
        let mut eff_bpm_a = bpm_a;
        while eff_bpm_a < 125.0 {
            eff_bpm_a *= 2.0;
        }
        while eff_bpm_a > 220.0 {
            eff_bpm_a /= 2.0;
        }

        let mut eff_bpm_b = bpm_b;
        while eff_bpm_b < 125.0 {
            eff_bpm_b *= 2.0;
        }
        while eff_bpm_b > 220.0 {
            eff_bpm_b /= 2.0;
        }

        let eff_bpm_diff_ratio = (eff_bpm_a - eff_bpm_b).abs() / eff_bpm_a;

        // 2. Outro Ending Check: If Track A ends in an abrupt Cold End and tempos clash
        if let Some(zone) = exit_a {
            if zone.detect_ending_type() == crate::schema::EndingType::ColdEnd
                && delta_agg < 0.38
                && eff_bpm_diff_ratio > 0.06
                && agg_a < 0.60
            {
                return self.build_archetype_5_cold_cut_recipe(track_a, track_b, exit_a, entry_b);
            }
        }

        // 3. Check Archetype 4: Large Vibe / Aggression Gap or Huge BPM Gap
        if eff_bpm_diff_ratio > 0.20 || delta_agg >= 0.38 {
            return self.build_archetype_4_tail_freeze_recipe(
                track_a,
                track_b,
                exit_a,
                entry_b,
            );
        }

        // 4. Check Archetype 2: Instrument Riff / Drum Fill Handoff (High Aggression Rock/Punk)
        let is_rock_or_punk = track_a.global_summary.macro_vibe_family.contains("ROCK")
            || track_a.global_summary.macro_vibe_family.contains("PUNK")
            || track_b.global_summary.macro_vibe_family.contains("ROCK")
            || track_b.global_summary.macro_vibe_family.contains("PUNK");

        let has_grit_or_punk = (is_rock_or_punk && agg_a >= 0.60 && agg_b >= 0.55)
            || (agg_a >= 0.72 && agg_b >= 0.70)
            || (agg_a >= 0.50 && agg_b >= 0.50 && (
                track_a.global_summary.dominant_instruments.iter().any(|i| i.contains("distorted") || i.contains("guitar"))
                && track_b.global_summary.dominant_instruments.iter().any(|i| i.contains("distorted") || i.contains("guitar"))
            ));

        if has_grit_or_punk {
            return self.build_archetype_2_riff_handoff_recipe(
                track_a,
                track_b,
                exit_a,
                entry_b,
                bpm_a,
            );
        }

        // 5. Default / Primary Mix: Walled Garden Music Algorithm 4-Stage DSP Lookahead Pipeline (Archetype 3 EqSwap)
        self.build_archetype_3_eq_swap_recipe(
            track_a,
            track_b,
            exit_a,
            entry_b,
            bpm_a,
            bpm_b,
        )
    }

    // -----------------------------------------------------------------------
    // Archetype Recipe Builders
    // -----------------------------------------------------------------------

    fn build_archetype_1_sustain_recipe(
        &self,
        track_a: &TrackAnalysisProfile,
        track_b: &TrackAnalysisProfile,
        sustain_a_start: u64,
        sustain_a_end: u64,
        sustain_b_start: u64,
        pitch_shift_cents: i32,
    ) -> TransitionRecipe {
        let duration_ms = sustain_a_end.saturating_sub(sustain_a_start).clamp(1200, 4000);
        let morph_mid_ms = duration_ms / 2;

        let keyframes = vec![
            DspAutomationKeyframe {
                offset_ms: 0,
                deck_a: DeckAutomation {
                    gain_db: 0.0,
                    eq_sub_db: 0.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
                deck_b: DeckAutomation {
                    gain_db: -60.0,
                    eq_sub_db: -24.0,
                    eq_mid_db: -12.0,
                    eq_high_db: -6.0,
                    stretch_ratio: 1.0,
                    pitch_cents: pitch_shift_cents,
                    reverb_freeze_send: 0.0,
                },
            },
            DspAutomationKeyframe {
                offset_ms: morph_mid_ms,
                deck_a: DeckAutomation {
                    gain_db: -3.0,
                    eq_sub_db: -18.0,
                    eq_mid_db: -2.0,
                    eq_high_db: -6.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.15,
                },
                deck_b: DeckAutomation {
                    gain_db: -1.5,
                    eq_sub_db: -12.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: pitch_shift_cents,
                    reverb_freeze_send: 0.0,
                },
            },
            DspAutomationKeyframe {
                offset_ms: duration_ms,
                deck_a: DeckAutomation {
                    gain_db: -60.0,
                    eq_sub_db: -60.0,
                    eq_mid_db: -60.0,
                    eq_high_db: -60.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
                deck_b: DeckAutomation {
                    gain_db: 0.0,
                    eq_sub_db: 0.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0, // Pitch returned to native
                    reverb_freeze_send: 0.0,
                },
            },
        ];

        TransitionRecipe {
            schema_version: 1,
            track_a_hash: track_a.track_id.audio_hash.clone(),
            track_b_hash: track_b.track_id.audio_hash.clone(),
            archetype: TransitionArchetype::SustainBridge,
            confidence_score: 0.94,
            timing: TransitionTiming {
                track_a_transition_start_ms: sustain_a_start,
                track_a_exit_end_ms: sustain_a_start + duration_ms,
                track_b_entry_start_ms: sustain_b_start,
                track_b_native_lock_ms: sustain_b_start + duration_ms + 1000,
            },
            optional_stem_bridge_uri: None,
            dsp_automation_keyframes: keyframes,
        }
    }

    fn build_archetype_2_riff_handoff_recipe(
        &self,
        track_a: &TrackAnalysisProfile,
        track_b: &TrackAnalysisProfile,
        exit_a: Option<&crate::schema::CueZone>,
        entry_b: Option<&crate::schema::CueZone>,
        bpm_a: f32,
    ) -> TransitionRecipe {
        let beat_ms = (60_000.0 / bpm_a) as u64;
        let punch_duration_ms = beat_ms * 4; // 1-bar (4-beat) pickup riff/fill

        let vocal_end = exit_a.and_then(|z| z.last_vocal_end_ms());
        let a_cut_point = exit_a
            .and_then(|z| {
                if let Some(v_end) = vocal_end {
                    // Golden Rule: Never cut off an active vocal phrase before it concludes!
                    // Look for a downbeat at or after vocal_end, or anchor strictly to z.end_ms
                    z.downbeats_ms
                        .iter()
                        .copied()
                        .find(|&db| db >= v_end && db <= z.end_ms)
                        .or_else(|| Some(z.end_ms))
                } else {
                    z.downbeats_ms
                        .iter()
                        .copied()
                        .filter(|&db| db <= z.end_ms)
                        .next_back()
                        .or_else(|| z.downbeats_ms.last().copied())
                }
            })
            .unwrap_or(track_a.track_id.duration_ms.saturating_sub(punch_duration_ms));

        let b_entry_point = entry_b
            .map(|z| z.start_ms)
            .unwrap_or(0);

        let trans_start_ms = a_cut_point.saturating_sub(punch_duration_ms);
        let has_vocals_in_handoff = exit_a
            .map(|z| z.has_vocal_between_ms(trans_start_ms, a_cut_point))
            .unwrap_or(false);

        let keyframes = if has_vocals_in_handoff {
            // Vocal-Preserving Handoff: Keep Deck A's vocal 100% full (0 dB mid/high), duck sub bass only
            vec![
                DspAutomationKeyframe {
                    offset_ms: 0,
                    deck_a: DeckAutomation {
                        gain_db: 0.0,
                        eq_sub_db: 0.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                    deck_b: DeckAutomation {
                        gain_db: -16.0,
                        eq_sub_db: -24.0,
                        eq_mid_db: -4.0,
                        eq_high_db: -2.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
                DspAutomationKeyframe {
                    offset_ms: punch_duration_ms / 4,
                    deck_a: DeckAutomation {
                        gain_db: 0.0,
                        eq_sub_db: -6.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                    deck_b: DeckAutomation {
                        gain_db: -8.0,
                        eq_sub_db: -12.0,
                        eq_mid_db: -2.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
                DspAutomationKeyframe {
                    offset_ms: punch_duration_ms / 2,
                    deck_a: DeckAutomation {
                        gain_db: 0.0,
                        eq_sub_db: -12.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                    deck_b: DeckAutomation {
                        gain_db: -4.0,
                        eq_sub_db: -6.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
                DspAutomationKeyframe {
                    offset_ms: (punch_duration_ms * 3) / 4,
                    deck_a: DeckAutomation {
                        gain_db: -0.5,
                        eq_sub_db: -18.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                    deck_b: DeckAutomation {
                        gain_db: -1.5,
                        eq_sub_db: -2.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
                // Downbeat / Vocal Resolution: The singer finishes the final syllable completely!
                // Reverb freeze splash catches the resolution to ring out smoothly over Deck B's drop
                DspAutomationKeyframe {
                    offset_ms: punch_duration_ms,
                    deck_a: DeckAutomation {
                        gain_db: -60.0,
                        eq_sub_db: -60.0,
                        eq_mid_db: -60.0,
                        eq_high_db: -60.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.45, // Lush ring-out for the final vocal resolution
                    },
                    deck_b: DeckAutomation {
                        gain_db: 0.0,
                        eq_sub_db: 0.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
            ]
        } else {
            // Instrumental Handoff: Dynamic notch carving
            vec![
                DspAutomationKeyframe {
                    offset_ms: 0,
                    deck_a: DeckAutomation {
                        gain_db: 0.0,
                        eq_sub_db: 0.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                    deck_b: DeckAutomation {
                        gain_db: -18.0,
                        eq_sub_db: -24.0,
                        eq_mid_db: -6.0,
                        eq_high_db: -3.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
                DspAutomationKeyframe {
                    offset_ms: punch_duration_ms / 4,
                    deck_a: DeckAutomation {
                        gain_db: -1.5,
                        eq_sub_db: -6.0,
                        eq_mid_db: -3.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                    deck_b: DeckAutomation {
                        gain_db: -6.0,
                        eq_sub_db: -12.0,
                        eq_mid_db: -2.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
                DspAutomationKeyframe {
                    offset_ms: punch_duration_ms / 2,
                    deck_a: DeckAutomation {
                        gain_db: -3.5,
                        eq_sub_db: -12.0,
                        eq_mid_db: -6.0,
                        eq_high_db: -3.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                    deck_b: DeckAutomation {
                        gain_db: -2.0,
                        eq_sub_db: -3.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
                DspAutomationKeyframe {
                    offset_ms: (punch_duration_ms * 3) / 4,
                    deck_a: DeckAutomation {
                        gain_db: -8.0,
                        eq_sub_db: -24.0,
                        eq_mid_db: -12.0,
                        eq_high_db: -6.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                    deck_b: DeckAutomation {
                        gain_db: 0.0,
                        eq_sub_db: 0.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
                DspAutomationKeyframe {
                    offset_ms: punch_duration_ms,
                    deck_a: DeckAutomation {
                        gain_db: -60.0,
                        eq_sub_db: -60.0,
                        eq_mid_db: -60.0,
                        eq_high_db: -60.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.20,
                    },
                    deck_b: DeckAutomation {
                        gain_db: 0.0,
                        eq_sub_db: 0.0,
                        eq_mid_db: 0.0,
                        eq_high_db: 0.0,
                        stretch_ratio: 1.0,
                        pitch_cents: 0,
                        reverb_freeze_send: 0.0,
                    },
                },
            ]
        };

        TransitionRecipe {
            schema_version: 1,
            track_a_hash: track_a.track_id.audio_hash.clone(),
            track_b_hash: track_b.track_id.audio_hash.clone(),
            archetype: TransitionArchetype::RiffHandoff,
            confidence_score: 0.91,
            timing: TransitionTiming {
                track_a_transition_start_ms: a_cut_point.saturating_sub(punch_duration_ms),
                track_a_exit_end_ms: a_cut_point,
                track_b_entry_start_ms: b_entry_point,
                track_b_native_lock_ms: b_entry_point + punch_duration_ms,
            },
            optional_stem_bridge_uri: None,
            dsp_automation_keyframes: keyframes,
        }
    }

    /// Walled Garden Music Algorithm Lookahead Pipeline (Archetype 3: EqSwap)
    /// 1. Dynamic Cue-Point Detection (Fade-out vs. Cold End, -18 dB outro drop, -24 dB intro rise)
    /// 2. Constant-Power Cosine/Sine Gain Curves (GA^2 + GB^2 = 1.0, 0 dB perceived dip at midpoint)
    /// 3. Frequency-Split DJ Bass Swap (180 Hz crossover with sub muted on Deck B until tight 0.5s-0.75s midpoint swap)
    /// 4. Conditional Beat-Grid Alignment & Camelot Dissonance Damping
    fn build_archetype_3_eq_swap_recipe(
        &self,
        track_a: &TrackAnalysisProfile,
        track_b: &TrackAnalysisProfile,
        exit_a: Option<&crate::schema::CueZone>,
        entry_b: Option<&crate::schema::CueZone>,
        bpm_a: f32,
        bpm_b: f32,
    ) -> TransitionRecipe {
        // Compute folded octave tempos into upbeat range [125.0, 220.0]
        let mut eff_bpm_a = bpm_a;
        while eff_bpm_a < 125.0 {
            eff_bpm_a *= 2.0;
        }
        while eff_bpm_a > 220.0 {
            eff_bpm_a /= 2.0;
        }

        let mut eff_bpm_b = bpm_b;
        while eff_bpm_b < 125.0 {
            eff_bpm_b *= 2.0;
        }
        while eff_bpm_b > 220.0 {
            eff_bpm_b /= 2.0;
        }

        let bpm_diff_ratio = (bpm_a - bpm_b).abs() / bpm_a;
        let folded_bpm_diff = (eff_bpm_a - eff_bpm_b).abs() / eff_bpm_a;
        let effective_bpm_diff = bpm_diff_ratio.min(folded_bpm_diff);

        let is_dissonant = are_keys_dissonant(
            &track_a.global_summary.primary_key,
            &track_b.global_summary.primary_key,
        );

        // Stage 1: Dynamic Cue-Point Detection
        // Find exact outro trigger (energy drops permanently below -18 dB relative to average, >= vocal_end)
        let a_cue_ms = exit_a
            .map(|z| z.find_outro_cue_trigger_ms(18.0))
            .unwrap_or_else(|| track_a.track_id.duration_ms.saturating_sub(6000));

        let b_intro_ms = entry_b
            .map(|z| z.find_intro_energy_rise_ms(-24.0))
            .unwrap_or(0);

        let beat_ms = (60_000.0 / eff_bpm_a) as u64;

        // Stage 4: Conditional Beat-Grid Alignment Decision Gate
        let (swap_duration_ms, a_start, b_start) = if effective_bpm_diff <= 0.06 {
            // Tempos are compatible (<= 6% difference)! Lock beat phase & downbeat alignment
            let num_beats = if is_dissonant { 8 } else { 16 }; // 2 bars if harmonic clash, 4 bars if harmonious
            let dur_ms = beat_ms * num_beats;

            // Phase-align downbeats: find nearest downbeat in Track A around or preceding a_cue_ms
            let a_start_aligned = exit_a
                .and_then(|z| {
                    z.downbeats_ms
                        .iter()
                        .copied()
                        .filter(|&db| db <= a_cue_ms && db + dur_ms <= z.end_ms + 500)
                        .next_back()
                        .or_else(|| {
                            z.beats_ms
                                .iter()
                                .copied()
                                .filter(|&b| b <= a_cue_ms && b + dur_ms <= z.end_ms + 500)
                                .next_back()
                        })
                })
                .unwrap_or_else(|| a_cue_ms.saturating_sub(dur_ms / 2));

            (dur_ms, a_start_aligned, b_intro_ms)
        } else {
            // Tempos clash (> 6% difference)! Skip beat-matching and execute clean, short (1.5s - 2.5s) bass-swapped crossfade
            let dur_ms = if is_dissonant { 1500u64 } else { 2200u64 };
            let a_start_aligned = a_cue_ms.saturating_sub(dur_ms / 2);
            (dur_ms, a_start_aligned, b_intro_ms)
        };

        // Vocal protection: check if vocal ends after a_start
        let vocal_end = exit_a.and_then(|z| z.last_vocal_end_ms());
        let vocal_offset = vocal_end.and_then(|v| {
            if v > a_start {
                Some(v - a_start)
            } else {
                None
            }
        });

        // Stage 2 & 3: Equal-Power Gain Curves + 180 Hz Frequency-Split Bass Swap
        let swap_window_ms = 750u64.min(swap_duration_ms / 2);
        let keyframes = generate_automix_keyframes(swap_duration_ms, vocal_offset, swap_window_ms);

        TransitionRecipe {
            schema_version: 1,
            track_a_hash: track_a.track_id.audio_hash.clone(),
            track_b_hash: track_b.track_id.audio_hash.clone(),
            archetype: TransitionArchetype::EqSwap,
            confidence_score: if effective_bpm_diff <= 0.06 { 0.94 } else { 0.88 },
            timing: TransitionTiming {
                track_a_transition_start_ms: a_start,
                track_a_exit_end_ms: a_start + swap_duration_ms,
                track_b_entry_start_ms: b_start,
                track_b_native_lock_ms: b_start + swap_duration_ms,
            },
            optional_stem_bridge_uri: None,
            dsp_automation_keyframes: keyframes,
        }
    }


    fn build_archetype_4_tail_freeze_recipe(
        &self,
        track_a: &TrackAnalysisProfile,
        track_b: &TrackAnalysisProfile,
        exit_a: Option<&crate::schema::CueZone>,
        entry_b: Option<&crate::schema::CueZone>,
    ) -> TransitionRecipe {
        let freeze_duration_ms = 3500u64; // 3.5s lush reverb decay

        let vocal_end = exit_a.and_then(|z| z.last_vocal_end_ms());
        let a_freeze_point = exit_a
            .and_then(|z| {
                if let Some(v_end) = vocal_end {
                    z.downbeats_ms
                        .iter()
                        .copied()
                        .find(|&db| db >= v_end && db <= z.end_ms)
                        .or_else(|| Some(z.end_ms))
                } else {
                    z.downbeats_ms
                        .iter()
                        .copied()
                        .filter(|&db| db + 300 <= z.end_ms)
                        .next_back()
                        .or_else(|| {
                            z.beats_ms
                                .iter()
                                .copied()
                                .filter(|&b| b + 300 <= z.end_ms)
                                .next_back()
                        })
                        .or_else(|| z.downbeats_ms.last().copied())
                }
            })
            .unwrap_or(track_a.track_id.duration_ms.saturating_sub(freeze_duration_ms));

        let b_start = entry_b
            .map(|z| z.start_ms)
            .unwrap_or(0);

        let keyframes = vec![
            DspAutomationKeyframe {
                offset_ms: 0,
                deck_a: DeckAutomation {
                    gain_db: 0.0,
                    eq_sub_db: 0.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
                deck_b: DeckAutomation {
                    gain_db: -60.0,
                    eq_sub_db: 0.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
            // Fast cut on Deck A + Reverb Freeze Wash triggered; Deck B enters cleanly
            DspAutomationKeyframe {
                offset_ms: 100,
                deck_a: DeckAutomation {
                    gain_db: -60.0,
                    eq_sub_db: -60.0,
                    eq_mid_db: -6.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 1.0, // Full freeze wash
                },
                deck_b: DeckAutomation {
                    gain_db: -9.0, // Clean smooth entry curve
                    eq_sub_db: 0.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
            DspAutomationKeyframe {
                offset_ms: 500,
                deck_a: DeckAutomation {
                    gain_db: -60.0,
                    eq_sub_db: -60.0,
                    eq_mid_db: -12.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.8,
                },
                deck_b: DeckAutomation {
                    gain_db: -2.0,
                    eq_sub_db: 0.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
            DspAutomationKeyframe {
                offset_ms: 1500,
                deck_a: DeckAutomation {
                    gain_db: -60.0,
                    eq_sub_db: -60.0,
                    eq_mid_db: -24.0,
                    eq_high_db: -6.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.4,
                },
                deck_b: DeckAutomation {
                    gain_db: 0.0,
                    eq_sub_db: 0.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
            DspAutomationKeyframe {
                offset_ms: freeze_duration_ms,
                deck_a: DeckAutomation {
                    gain_db: -60.0,
                    eq_sub_db: -60.0,
                    eq_mid_db: -60.0,
                    eq_high_db: -60.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
                deck_b: DeckAutomation {
                    gain_db: 0.0,
                    eq_sub_db: 0.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
        ];

        TransitionRecipe {
            schema_version: 1,
            track_a_hash: track_a.track_id.audio_hash.clone(),
            track_b_hash: track_b.track_id.audio_hash.clone(),
            archetype: TransitionArchetype::TailFreeze,
            confidence_score: 0.89,
            timing: TransitionTiming {
                track_a_transition_start_ms: a_freeze_point,
                track_a_exit_end_ms: a_freeze_point + freeze_duration_ms,
                track_b_entry_start_ms: b_start,
                track_b_native_lock_ms: b_start + freeze_duration_ms,
            },
            optional_stem_bridge_uri: None,
            dsp_automation_keyframes: keyframes,
        }
    }

    fn build_archetype_5_cold_cut_recipe(
        &self,
        track_a: &TrackAnalysisProfile,
        track_b: &TrackAnalysisProfile,
        exit_a: Option<&crate::schema::CueZone>,
        entry_b: Option<&crate::schema::CueZone>,
    ) -> TransitionRecipe {
        let a_cut = exit_a
            .and_then(|z| {
                z.downbeats_ms
                    .iter()
                    .copied()
                    .filter(|&db| db <= z.end_ms)
                    .next_back()
                    .or_else(|| z.beats_ms.last().copied())
            })
            .unwrap_or_else(|| exit_a.map(|z| z.end_ms).unwrap_or(track_a.track_id.duration_ms.saturating_sub(100)));

        let b_start = entry_b.map(|z| z.start_ms).unwrap_or(0);

        let keyframes = vec![
            DspAutomationKeyframe {
                offset_ms: 0,
                deck_a: DeckAutomation::default(),
                deck_b: DeckAutomation {
                    gain_db: -60.0,
                    ..DeckAutomation::default()
                },
            },
            DspAutomationKeyframe {
                offset_ms: 100,
                deck_a: DeckAutomation {
                    gain_db: -60.0,
                    ..DeckAutomation::default()
                },
                deck_b: DeckAutomation::default(),
            },
        ];

        TransitionRecipe {
            schema_version: 1,
            track_a_hash: track_a.track_id.audio_hash.clone(),
            track_b_hash: track_b.track_id.audio_hash.clone(),
            archetype: TransitionArchetype::ColdCut,
            confidence_score: 0.75,
            timing: TransitionTiming {
                track_a_transition_start_ms: a_cut,
                track_a_exit_end_ms: a_cut + 100,
                track_b_entry_start_ms: b_start,
                track_b_native_lock_ms: b_start + 100,
            },
            optional_stem_bridge_uri: None,
            dsp_automation_keyframes: keyframes,
        }
    }

}

/// Maps a musical key string like "C Major", "A Minor", "F#", "Db min" to Camelot position (1..=12, 'A' or 'B')
pub fn parse_camelot_key(key: &str) -> Option<(u8, char)> {
    let lower = key.to_lowercase();
    let is_minor = lower.contains("min") || (lower.contains('m') && !lower.contains("maj"));

    let root = if lower.starts_with("c#") || lower.starts_with("db") {
        "c#"
    } else if lower.starts_with("d#") || lower.starts_with("eb") {
        "eb"
    } else if lower.starts_with("f#") || lower.starts_with("gb") {
        "f#"
    } else if lower.starts_with("g#") || lower.starts_with("ab") {
        "ab"
    } else if lower.starts_with("a#") || lower.starts_with("bb") {
        "bb"
    } else if lower.starts_with('c') {
        "c"
    } else if lower.starts_with('d') {
        "d"
    } else if lower.starts_with('e') {
        "e"
    } else if lower.starts_with('f') {
        "f"
    } else if lower.starts_with('g') {
        "g"
    } else if lower.starts_with('a') {
        "a"
    } else if lower.starts_with('b') {
        "b"
    } else {
        return None;
    };

    let (num, mode) = match (root, is_minor) {
        ("ab", false) | ("g#", false) => (1, 'B'),
        ("f", true) => (1, 'A'),
        ("eb", false) | ("d#", false) => (2, 'B'),
        ("c", true) => (2, 'A'),
        ("bb", false) | ("a#", false) => (3, 'B'),
        ("g", true) => (3, 'A'),
        ("f", false) => (4, 'B'),
        ("d", true) => (4, 'A'),
        ("c", false) => (5, 'B'),
        ("a", true) => (5, 'A'),
        ("g", false) => (6, 'B'),
        ("e", true) => (6, 'A'),
        ("d", false) => (7, 'B'),
        ("b", true) => (7, 'A'),
        ("a", false) => (8, 'B'),
        ("f#", true) => (8, 'A'),
        ("e", false) => (9, 'B'),
        ("c#", true) => (9, 'A'),
        ("b", false) => (10, 'B'),
        ("g#", true) | ("ab", true) => (10, 'A'),
        ("f#", false) | ("gb", false) => (11, 'B'),
        ("d#", true) | ("eb", true) => (11, 'A'),
        ("c#", false) | ("db", false) => (12, 'B'),
        ("bb", true) | ("a#", true) => (12, 'A'),
        _ => return None,
    };
    Some((num, mode))
}

/// Returns true if two keys clash harmonically (distance on Camelot wheel >= 3 or discordant mode switch)
pub fn are_keys_dissonant(key_a: &str, key_b: &str) -> bool {
    let (num_a, mode_a) = match parse_camelot_key(key_a) {
        Some(c) => c,
        None => return false,
    };
    let (num_b, mode_b) = match parse_camelot_key(key_b) {
        Some(c) => c,
        None => return false,
    };

    let diff = (num_a as i32 - num_b as i32).abs();
    let circle_dist = diff.min(12 - diff);

    if circle_dist == 0 {
        return false;
    }
    if circle_dist == 1 && mode_a == mode_b {
        return false;
    }
    circle_dist >= 3 || (circle_dist >= 2 && mode_a != mode_b)
}

/// Generates sample-accurate walled garden music algorithm automation keyframes:
/// - Stage 2: Constant-Power (Cosine / Sine) Gain Curves (GA^2 + GB^2 = 1.0, 0 dB perceived volume dip)
/// - Stage 3: Frequency-Split DJ Bass Swap at 180 Hz with sub ducked until tight midpoint crossover
pub fn generate_automix_keyframes(
    duration_ms: u64,
    vocal_end_offset_ms: Option<u64>,
    swap_window_ms: u64,
) -> Vec<DspAutomationKeyframe> {
    let num_steps = 16.max((duration_ms / 100) as usize).min(64);
    let mut keyframes = Vec::with_capacity(num_steps + 1);

    let half_duration = duration_ms as f32 / 2.0;
    let swap_half = (swap_window_ms as f32 / 2.0).min(half_duration * 0.8);
    let swap_start_ms = (half_duration - swap_half).max(0.0);
    let swap_end_ms = (half_duration + swap_half).min(duration_ms as f32);

    for step in 0..=num_steps {
        let t = step as f32 / num_steps as f32; // 0.0 to 1.0
        let offset_ms = (duration_ms as f32 * t).round() as u64;

        // Stage 2: Constant-Power (Cosine / Sine) Gain Curves
        // G_A(t) = cos(pi/2 * t), G_B(t) = sin(pi/2 * t)
        // Total acoustic power: G_A^2 + G_B^2 = 1.0 (0 dB perceived dip at midpoint!)
        let angle = t * (std::f32::consts::PI / 2.0);
        let g_a = angle.cos();
        let g_b = angle.sin();

        let gain_db_a = if g_a <= 1e-3 { -60.0 } else { 20.0 * g_a.log10() };
        let gain_db_b = if g_b <= 1e-3 { -60.0 } else { 20.0 * g_b.log10() };

        // Stage 3: Frequency-Split DJ Bass Swap (180 Hz)
        // Sub frequencies swap rapidly across a tight 0.5s - 1.0s window around the midpoint
        let offset_f = offset_ms as f32;
        let (eq_sub_a, eq_sub_b) = if offset_f < swap_start_ms {
            (0.0, -24.0) // Deck A sub active, Deck B sub muted
        } else if offset_f > swap_end_ms {
            (-24.0, 0.0) // Deck A sub ducked, Deck B sub active
        } else {
            // Tight crossover in the swap window
            let s = (offset_f - swap_start_ms) / (swap_end_ms - swap_start_ms).max(1.0);
            let sub_a = -24.0 * s;
            let sub_b = -24.0 * (1.0 - s);
            (sub_a, sub_b)
        };

        // Vocal presence protection on Deck A: keep mid EQ flat (0.0 dB) if vocals are singing
        let eq_mid_a = if let Some(vocal_end) = vocal_end_offset_ms {
            if offset_ms <= vocal_end {
                0.0
            } else {
                -2.0 * t
            }
        } else {
            -2.0 * t
        };

        keyframes.push(DspAutomationKeyframe {
            offset_ms,
            deck_a: DeckAutomation {
                gain_db: gain_db_a,
                eq_sub_db: eq_sub_a,
                eq_mid_db: eq_mid_a,
                eq_high_db: 0.0,
                stretch_ratio: 1.0,
                pitch_cents: 0,
                reverb_freeze_send: 0.0,
            },
            deck_b: DeckAutomation {
                gain_db: gain_db_b,
                eq_sub_db: eq_sub_b,
                eq_mid_db: 0.0,
                eq_high_db: 0.0,
                stretch_ratio: 1.0,
                pitch_cents: 0,
                reverb_freeze_send: 0.0,
            },
        });
    }

    keyframes
}

