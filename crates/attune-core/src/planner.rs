use crate::schema::{
    DeckAutomation, DspAutomationKeyframe, TrackAnalysisProfile, TransitionArchetype,
    TransitionRecipe, TransitionTiming, ZoneType,
};

pub struct TransitionPlanner;

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
        let bpm_diff_ratio = (bpm_a - bpm_b).abs() / bpm_a;

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

        // 2. Check Archetype 4: Large Vibe / Aggression Gap or Huge BPM Gap
        if bpm_diff_ratio > 0.20 || delta_agg >= 0.38 {
            return self.build_archetype_4_tail_freeze_recipe(
                track_a,
                track_b,
                exit_a,
                entry_b,
            );
        }

        // 3. Check Archetype 2: Instrument Riff / Drum Fill Handoff (High Aggression)
        let has_grit_or_punk = (agg_a >= 0.60 && agg_b >= 0.55)
            || (agg_a >= 0.50 && agg_b >= 0.50 && (
                track_a.global_summary.dominant_instruments.iter().any(|i| i.contains("distorted") || i.contains("grit"))
                && track_b.global_summary.dominant_instruments.iter().any(|i| i.contains("distorted") || i.contains("grit"))
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

        // 4. Default / Archetype 3: Beat-Locked 3-Band EQ Swap
        if bpm_diff_ratio <= 0.20 {
            return self.build_archetype_3_eq_swap_recipe(
                track_a,
                track_b,
                exit_a,
                entry_b,
                bpm_a,
                bpm_b,
            );
        }

        // 5. Fallback: Archetype 5 Cold Cut
        self.build_archetype_5_cold_cut_recipe(track_a, track_b, exit_a, entry_b)
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
        let punch_duration_ms = beat_ms * 2; // 2-beat pickup riff/fill

        let a_cut_point = exit_a
            .and_then(|z| z.downbeats_ms.last().copied())
            .unwrap_or(track_a.track_id.duration_ms.saturating_sub(punch_duration_ms));

        let b_entry_point = entry_b
            .and_then(|z| z.beats_ms.first().copied())
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
                    eq_sub_db: -24.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
            // Pickup punch: Deck B riff cuts in, Deck A dips midrange by 6 dB to avoid mud
            DspAutomationKeyframe {
                offset_ms: punch_duration_ms / 2,
                deck_a: DeckAutomation {
                    gain_db: -2.0,
                    eq_sub_db: -6.0,
                    eq_mid_db: -6.0, // Dynamic notch to prevent distortion clash
                    eq_high_db: -3.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
                deck_b: DeckAutomation {
                    gain_db: 0.0,
                    eq_sub_db: -6.0,
                    eq_mid_db: 0.0,
                    eq_high_db: 0.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
            // Slam Downbeat: Track A hard cuts, Track B takes over full power
            DspAutomationKeyframe {
                offset_ms: punch_duration_ms,
                deck_a: DeckAutomation {
                    gain_db: -60.0,
                    eq_sub_db: -60.0,
                    eq_mid_db: -60.0,
                    eq_high_db: -60.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.2, // 500ms short decay tail
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

    fn build_archetype_3_eq_swap_recipe(
        &self,
        track_a: &TrackAnalysisProfile,
        track_b: &TrackAnalysisProfile,
        exit_a: Option<&crate::schema::CueZone>,
        entry_b: Option<&crate::schema::CueZone>,
        bpm_a: f32,
        bpm_b: f32,
    ) -> TransitionRecipe {
        let beat_ms = (60_000.0 / bpm_a) as u64;
        let swap_window_ms = beat_ms * 16; // 4-bar phrase mix
        let half_window_ms = swap_window_ms / 2;

        let a_start = exit_a
            .and_then(|z| z.phrase_boundaries_ms.last().copied())
            .unwrap_or(track_a.track_id.duration_ms.saturating_sub(swap_window_ms));

        let b_start = entry_b
            .and_then(|z| z.phrase_boundaries_ms.first().copied())
            .unwrap_or(0);

        let tempo_ratio = bpm_a / bpm_b;

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
                    eq_sub_db: -24.0, // Sub muted
                    eq_mid_db: -12.0,
                    eq_high_db: -6.0,
                    stretch_ratio: tempo_ratio,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
            // Midpoint: Bass Crossover Swap on downbeat
            DspAutomationKeyframe {
                offset_ms: half_window_ms,
                deck_a: DeckAutomation {
                    gain_db: -2.0,
                    eq_sub_db: -24.0, // Deck A drops sub bass
                    eq_mid_db: -3.0,
                    eq_high_db: -4.0,
                    stretch_ratio: 1.0,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
                deck_b: DeckAutomation {
                    gain_db: -1.0,
                    eq_sub_db: 0.0, // Deck B kicks in with sub bass!
                    eq_mid_db: -2.0,
                    eq_high_db: -1.0,
                    stretch_ratio: tempo_ratio,
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
            // Complete mix out
            DspAutomationKeyframe {
                offset_ms: swap_window_ms,
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
                    stretch_ratio: 1.0, // Back to native tempo
                    pitch_cents: 0,
                    reverb_freeze_send: 0.0,
                },
            },
        ];

        TransitionRecipe {
            schema_version: 1,
            track_a_hash: track_a.track_id.audio_hash.clone(),
            track_b_hash: track_b.track_id.audio_hash.clone(),
            archetype: TransitionArchetype::EqSwap,
            confidence_score: 0.88,
            timing: TransitionTiming {
                track_a_transition_start_ms: a_start,
                track_a_exit_end_ms: a_start + swap_window_ms,
                track_b_entry_start_ms: b_start,
                track_b_native_lock_ms: b_start + swap_window_ms,
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

        let a_freeze_point = exit_a
            .and_then(|z| z.downbeats_ms.last().copied())
            .unwrap_or(track_a.track_id.duration_ms.saturating_sub(freeze_duration_ms));

        let b_start = entry_b
            .and_then(|z| z.beats_ms.first().copied())
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
            // Immediate dry mute + Reverb Freeze Wash triggered on Deck A
            DspAutomationKeyframe {
                offset_ms: 200,
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
                    gain_db: -6.0,
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
            .map(|z| z.end_ms.saturating_sub(1000))
            .unwrap_or(track_a.track_id.duration_ms.saturating_sub(1000));

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
