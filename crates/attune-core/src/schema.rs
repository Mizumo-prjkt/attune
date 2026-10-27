use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackId {
    pub audio_hash: String,
    pub duration_ms: u64,
    pub canonical_slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalysisTier {
    #[serde(rename = "TIER_1_FAST_HEAD_TAIL")]
    FastHeadTail,
    #[serde(rename = "TIER_2_FULL_LOCAL")]
    FullLocal,
    #[serde(rename = "TIER_3_EXTERNAL_DEEP_ML")]
    ExternalDeepMl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobalSummary {
    pub nominal_bpm: f32,
    pub bpm_stability: f32,
    pub primary_key: String,
    pub mean_aggression_index: f32,
    pub macro_vibe_family: String,
    pub dominant_instruments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ZoneType {
    ENTRY,
    EXIT,
    BREAKDOWN,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SustainedPitchRegion {
    pub start_ms: u64,
    pub end_ms: u64,
    pub mean_midi_cents: f32,
    pub pitch_std_cents: f32,
    pub voicing_confidence: f32,
    pub formant_f1_hz: f32,
    pub formant_f2_hz: f32,
    pub percussive_leakage_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Frames20Hz {
    pub f0_midi_cents: Vec<f32>,
    pub chroma_12: Vec<[f32; 12]>,
    pub energy_4band_db: Vec<[f32; 4]>,
    pub aggression_curve: Vec<f32>,
    pub instrument_activations: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CueZone {
    pub zone_type: ZoneType,
    pub start_ms: u64,
    pub end_ms: u64,
    pub hop_ms: u32,
    pub beats_ms: Vec<u64>,
    pub downbeats_ms: Vec<u64>,
    pub phrase_boundaries_ms: Vec<u64>,
    pub sustained_pitch_regions: Vec<SustainedPitchRegion>,
    pub frames_20hz: Frames20Hz,
}

impl CueZone {
    /// Returns true if vocal activity is detected at or around the specified timestamp
    pub fn is_vocal_active_at_ms(&self, ms: u64) -> bool {
        if ms < self.start_ms || ms > self.end_ms || self.hop_ms == 0 {
            return false;
        }
        let frame_idx = ((ms.saturating_sub(self.start_ms)) / self.hop_ms as u64) as usize;
        let min_idx = frame_idx.saturating_sub(2);
        let max_idx = (frame_idx + 2).min(self.frames_20hz.instrument_activations.len().saturating_sub(1));
        for i in min_idx..=max_idx {
            if let Some(inst) = self.frames_20hz.instrument_activations.get(i) {
                let clean = inst.get(4).copied().unwrap_or(0.0);
                let grit = inst.get(5).copied().unwrap_or(0.0);
                let f0 = self.frames_20hz.f0_midi_cents.get(i).copied().unwrap_or(0.0);
                if clean > 0.30 || grit > 0.35 || (f0 > 0.0 && clean > 0.15) {
                    return true;
                }
            }
        }
        false
    }

    /// Returns the timestamp (in ms) where vocal singing concludes in this zone
    pub fn last_vocal_end_ms(&self) -> Option<u64> {
        for (i, inst) in self.frames_20hz.instrument_activations.iter().enumerate().rev() {
            let clean = inst.get(4).copied().unwrap_or(0.0);
            let grit = inst.get(5).copied().unwrap_or(0.0);
            let f0 = self.frames_20hz.f0_midi_cents.get(i).copied().unwrap_or(0.0);
            if clean > 0.30 || grit > 0.35 || (f0 > 0.0 && clean > 0.15) {
                let ms = self.start_ms + (i as u64 * self.hop_ms as u64) + self.hop_ms as u64;
                return Some(ms.min(self.end_ms));
            }
        }
        None
    }

    /// Returns true if any vocal activity is present in the specified interval [start_ms, end_ms]
    pub fn has_vocal_between_ms(&self, start_ms: u64, end_ms: u64) -> bool {
        if self.hop_ms == 0 || start_ms > end_ms {
            return false;
        }
        let s = start_ms.max(self.start_ms);
        let e = end_ms.min(self.end_ms);
        if s > e {
            return false;
        }
        let start_idx = ((s - self.start_ms) / self.hop_ms as u64) as usize;
        let end_idx = (((e - self.start_ms) / self.hop_ms as u64) as usize)
            .min(self.frames_20hz.instrument_activations.len().saturating_sub(1));

        for i in start_idx..=end_idx {
            if let Some(inst) = self.frames_20hz.instrument_activations.get(i) {
                let clean = inst.get(4).copied().unwrap_or(0.0);
                let grit = inst.get(5).copied().unwrap_or(0.0);
                let f0 = self.frames_20hz.f0_midi_cents.get(i).copied().unwrap_or(0.0);
                if clean > 0.30 || grit > 0.35 || (f0 > 0.0 && clean > 0.15) {
                    return true;
                }
            }
        }
        false
    }

    /// Detect whether this zone concludes in a gradual fade-out or an abrupt cold-end / reverb ring-out
    pub fn detect_ending_type(&self) -> EndingType {
        let loudness = self.compute_short_term_loudness_db();
        if loudness.len() < 10 {
            return EndingType::ColdEnd;
        }
        let peak_loudness = loudness.iter().copied().fold(-120.0f32, f32::max);
        let threshold = peak_loudness - 18.0;

        // Check if there is a long gradual decay spanning >= 2.5 seconds
        if let Some(first_cross) = loudness.iter().position(|&l| l <= threshold) {
            let decay_frames = loudness.len() - first_cross;
            let decay_ms = decay_frames as u64 * self.hop_ms.max(1) as u64;
            if decay_ms >= 1500 {
                return EndingType::FadeOut;
            }
        }
        EndingType::ColdEnd
    }

    /// Computes short-term loudness contour (RMS dBFS per frame)
    pub fn compute_short_term_loudness_db(&self) -> Vec<f32> {
        self.frames_20hz
            .energy_4band_db
            .iter()
            .map(|band_db| {
                let p_sum: f32 = band_db
                    .iter()
                    .map(|&db| 10.0f32.powf(db / 10.0))
                    .sum();
                if p_sum > 1e-9 {
                    10.0 * p_sum.log10()
                } else {
                    -90.0
                }
            })
            .collect()
    }

    /// Dynamic Cue-Point Trigger: Finds the exact frame in Track A's outro where energy drops
    /// permanently below `rel_drop_db` (default 18.0 dB) relative to average zone loudness,
    /// strictly clamped to >= last_vocal_end_ms to preserve vocal phrase integrity.
    pub fn find_outro_cue_trigger_ms(&self, rel_drop_db: f32) -> u64 {
        let loudness = self.compute_short_term_loudness_db();
        if loudness.is_empty() {
            return self.end_ms;
        }
        let avg_loudness: f32 = loudness.iter().sum::<f32>() / loudness.len() as f32;
        let threshold = avg_loudness - rel_drop_db.abs();

        let vocal_end = self.last_vocal_end_ms().unwrap_or(self.start_ms);

        let mut drop_idx = loudness.len().saturating_sub(1);
        for i in 0..loudness.len() {
            let ms = self.start_ms + (i as u64 * self.hop_ms.max(1) as u64);
            if ms < vocal_end {
                continue;
            }
            // Permanently drops below threshold for remaining frames
            if loudness[i..].iter().all(|&l| l <= threshold) {
                drop_idx = i;
                break;
            }
        }

        let trigger_ms = self.start_ms + (drop_idx as u64 * self.hop_ms.max(1) as u64);
        trigger_ms.max(vocal_end).min(self.end_ms)
    }

    /// Dynamic Cue-Point Trigger: Finds the point where Track B's intro rises above `threshold_db` (e.g. -24 dBFS)
    pub fn find_intro_energy_rise_ms(&self, threshold_db: f32) -> u64 {
        let loudness = self.compute_short_term_loudness_db();
        for (i, &l) in loudness.iter().enumerate() {
            if l >= threshold_db {
                return self.start_ms + (i as u64 * self.hop_ms.max(1) as u64);
            }
        }
        self.downbeats_ms.first().copied().unwrap_or(self.start_ms)
    }
}

/// Outro termination behavior for dynamic cue detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndingType {
    FadeOut,
    ColdEnd,
}

/// Track Analysis Profile (.tap)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackAnalysisProfile {
    pub schema_version: u32,
    pub track_id: TrackId,
    pub analysis_tier: AnalysisTier,
    pub global_summary: GlobalSummary,
    pub cue_zones: Vec<CueZone>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransitionArchetype {
    #[serde(rename = "ARCHETYPE_1_SUSTAIN_BRIDGE")]
    SustainBridge,
    #[serde(rename = "ARCHETYPE_2_RIFF_HANDOFF")]
    RiffHandoff,
    #[serde(rename = "ARCHETYPE_3_EQ_SWAP")]
    EqSwap,
    #[serde(rename = "ARCHETYPE_4_TAIL_FREEZE")]
    TailFreeze,
    #[serde(rename = "ARCHETYPE_5_COLD_CUT")]
    ColdCut,
    #[serde(rename = "ARCHETYPE_6_GENERATIVE_MELODIC_FILL")]
    GenerativeMelodicFill,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransitionTiming {
    pub track_a_transition_start_ms: u64,
    pub track_a_exit_end_ms: u64,
    pub track_b_entry_start_ms: u64,
    pub track_b_native_lock_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeckAutomation {
    pub gain_db: f32,
    pub eq_sub_db: f32,
    pub eq_mid_db: f32,
    pub eq_high_db: f32,
    pub stretch_ratio: f32,
    pub pitch_cents: i32,
    pub reverb_freeze_send: f32,
}

impl Default for DeckAutomation {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            eq_sub_db: 0.0,
            eq_mid_db: 0.0,
            eq_high_db: 0.0,
            stretch_ratio: 1.0,
            pitch_cents: 0,
            reverb_freeze_send: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DspAutomationKeyframe {
    pub offset_ms: u64,
    pub deck_a: DeckAutomation,
    pub deck_b: DeckAutomation,
}

/// Pairwise Transition Recipe (.ptr)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransitionRecipe {
    pub schema_version: u32,
    pub track_a_hash: String,
    pub track_b_hash: String,
    pub archetype: TransitionArchetype,
    pub confidence_score: f32,
    pub timing: TransitionTiming,
    pub optional_stem_bridge_uri: Option<String>,
    pub dsp_automation_keyframes: Vec<DspAutomationKeyframe>,
}

pub type PairwiseTransitionRecipe = TransitionRecipe;

