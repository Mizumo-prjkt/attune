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
