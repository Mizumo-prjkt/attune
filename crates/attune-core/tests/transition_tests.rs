use attune_core::audio::AudioBuffer;
use attune_core::planner::TransitionPlanner;
use attune_core::profiler::AudioProfiler;
use attune_core::renderer::TransitionRenderer;
use attune_core::schema::{AnalysisTier, TransitionArchetype};
use std::f32::consts::PI;

fn generate_test_tone(sr: u32, dur_sec: f32, freq: f32) -> AudioBuffer {
    let frames = (dur_sec * sr as f32) as usize;
    let mut ch0 = Vec::with_capacity(frames);
    let mut ch1 = Vec::with_capacity(frames);

    for i in 0..frames {
        let t = i as f32 / sr as f32;
        let s = (2.0 * PI * freq * t).sin() * 0.7;
        ch0.push(s);
        ch1.push(s);
    }

    AudioBuffer {
        channels: vec![ch0, ch1],
        sample_rate: sr,
    }
}

#[test]
fn test_audio_buffer_operations() {
    let sr = 44100;
    let buf = generate_test_tone(sr, 2.0, 440.0);

    assert_eq!(buf.num_channels(), 2);
    assert_eq!(buf.sample_rate, 44100);
    assert_eq!(buf.duration_ms(), 2000);

    let mono = buf.to_mono();
    assert_eq!(mono.len(), buf.num_frames());

    let slice = buf.slice_ms(500, 1500);
    assert_eq!(slice.duration_ms(), 1000);

    let hash = buf.compute_hash();
    assert!(!hash.is_empty());
}

#[test]
fn test_sustain_morph_transition_planning() {
    let sr = 44100;
    let profiler = AudioProfiler::new();
    let planner = TransitionPlanner::new();

    // 440 Hz (A4) and 445 Hz (within 20 cents)
    let buf_a = generate_test_tone(sr, 3.0, 440.0);
    let buf_b = generate_test_tone(sr, 3.0, 445.0);

    let prof_a = profiler.profile_track(&buf_a, "track_a", AnalysisTier::FastHeadTail);
    let prof_b = profiler.profile_track(&buf_b, "track_b", AnalysisTier::FastHeadTail);

    let recipe = planner.plan_transition(&prof_a, &prof_b);

    assert_eq!(recipe.archetype, TransitionArchetype::SustainBridge);
    assert!(recipe.confidence_score >= 0.85);
    assert!(!recipe.dsp_automation_keyframes.is_empty());
}

#[test]
fn test_renderer_wav_output() {
    let sr = 44100;
    let profiler = AudioProfiler::new();
    let planner = TransitionPlanner::new();
    let renderer = TransitionRenderer::new();

    let buf_a = generate_test_tone(sr, 2.0, 440.0);
    let buf_b = generate_test_tone(sr, 2.0, 440.0);

    let prof_a = profiler.profile_track(&buf_a, "test_a", AnalysisTier::FastHeadTail);
    let prof_b = profiler.profile_track(&buf_b, "test_b", AnalysisTier::FastHeadTail);

    let recipe = planner.plan_transition(&prof_a, &prof_b);

    let temp_dir = std::env::temp_dir();
    let out_wav = temp_dir.join("attune_test_render.wav");

    let result = renderer.render_to_wav(&buf_a, &buf_b, &recipe, &out_wav);
    assert!(result.is_ok());
    assert!(out_wav.exists());
    let _ = std::fs::remove_file(out_wav);
}

#[test]
fn test_audible_silence_detection() {
    let sr = 44100;
    // 500ms silence + 1000ms tone + 500ms silence = 2000ms total
    let total_frames = (2.0 * sr as f32) as usize;
    let mut ch0 = vec![0.0f32; total_frames];
    let mut ch1 = vec![0.0f32; total_frames];

    let start_idx = (0.5 * sr as f32) as usize;
    let end_idx = (1.5 * sr as f32) as usize;
    for i in start_idx..end_idx {
        let t = i as f32 / sr as f32;
        let s = (2.0 * PI * 440.0 * t).sin() * 0.7;
        ch0[i] = s;
        ch1[i] = s;
    }

    let buf = AudioBuffer {
        channels: vec![ch0, ch1],
        sample_rate: sr,
    };

    let start_ms = buf.find_audible_start_ms(-45.0);
    let end_ms = buf.find_audible_end_ms(-45.0);

    // Should detect start around 500ms and end around 1500ms within 20ms chunk tolerance
    assert!((start_ms as i64 - 500).abs() <= 20, "start_ms was {}", start_ms);
    assert!((end_ms as i64 - 1500).abs() <= 20, "end_ms was {}", end_ms);
}

#[test]
fn test_vocal_protection_in_transition_planning() {
    use attune_core::schema::{
        AnalysisTier, CueZone, Frames20Hz, GlobalSummary, TrackAnalysisProfile, TrackId, ZoneType,
    };

    let hop_ms = 50u32;
    let frames_count = 100; // 5000ms
    let mut insts = vec![vec![0.0f32; 6]; frames_count];
    let mut f0 = vec![0.0f32; frames_count];

    // Simulate vocal singing across the last 20 frames (last 1000ms, from 157000ms to 158000ms)
    for i in 80..100 {
        insts[i][4] = 0.95; // clean vocal active
        f0[i] = 6900.0;     // A4 active pitch
    }

    let exit_zone = CueZone {
        zone_type: ZoneType::EXIT,
        start_ms: 153000,
        end_ms: 158000,
        hop_ms,
        beats_ms: vec![154000, 155000, 156000, 157000, 157500],
        downbeats_ms: vec![154000, 156000, 157500], // Downbeat at 157500 is mid-vocal!
        phrase_boundaries_ms: vec![154000],
        sustained_pitch_regions: vec![],
        frames_20hz: Frames20Hz {
            f0_midi_cents: f0,
            chroma_12: vec![[0.0; 12]; frames_count],
            energy_4band_db: vec![[0.0; 4]; frames_count],
            aggression_curve: vec![0.65; frames_count],
            instrument_activations: insts,
        },
    };

    let track_a = TrackAnalysisProfile {
        schema_version: 1,
        track_id: TrackId {
            audio_hash: "track_a".to_string(),
            duration_ms: 158000,
            canonical_slug: "track_a".to_string(),
        },
        analysis_tier: AnalysisTier::FastHeadTail,
        global_summary: GlobalSummary {
            nominal_bpm: 171.4,
            bpm_stability: 0.9,
            primary_key: "C Major".to_string(),
            mean_aggression_index: 0.67,
            macro_vibe_family: "AGGRESSIVE_ROCK_PUNK_JROCK".to_string(),
            dominant_instruments: vec!["inst_synth_electronic".to_string(), "vocal_clean_melodic".to_string(), "vocal_grit_fry_growl".to_string()],
        },
        cue_zones: vec![exit_zone],
    };

    let track_b = TrackAnalysisProfile {
        schema_version: 1,
        track_id: TrackId {
            audio_hash: "track_b".to_string(),
            duration_ms: 160000,
            canonical_slug: "track_b".to_string(),
        },
        analysis_tier: AnalysisTier::FastHeadTail,
        global_summary: GlobalSummary {
            nominal_bpm: 171.4,
            bpm_stability: 0.9,
            primary_key: "C Major".to_string(),
            mean_aggression_index: 0.65,
            macro_vibe_family: "AGGRESSIVE_ROCK_PUNK_JROCK".to_string(),
            dominant_instruments: vec!["inst_synth_electronic".to_string(), "vocal_clean_melodic".to_string()],
        },
        cue_zones: vec![CueZone {
            zone_type: ZoneType::ENTRY,
            start_ms: 1000,
            end_ms: 45000,
            hop_ms: 50,
            beats_ms: vec![1000, 1350, 1700, 2050],
            downbeats_ms: vec![1000, 2400],
            phrase_boundaries_ms: vec![1000],
            sustained_pitch_regions: vec![],
            frames_20hz: Frames20Hz {
                f0_midi_cents: vec![0.0; 100],
                chroma_12: vec![[0.0; 12]; 100],
                energy_4band_db: vec![[0.0; 4]; 100],
                aggression_curve: vec![0.6; 100],
                instrument_activations: vec![vec![0.0; 6]; 100],
            },
        }],
    };

    let planner = TransitionPlanner::new();
    let recipe = planner.plan_transition(&track_a, &track_b);

    // Track A exit end must NOT cut off at 157500ms; it must extend to at least 158000ms!
    assert!(
        recipe.timing.track_a_exit_end_ms >= 158000,
        "Expected exit_end_ms >= 158000 (after vocal resolution), but was {}",
        recipe.timing.track_a_exit_end_ms
    );

    // Verify keyframes preserve vocal clarity (mid EQ is 0 dB, not heavily ducked)
    for kf in &recipe.dsp_automation_keyframes {
        if kf.offset_ms < recipe.timing.track_a_exit_end_ms.saturating_sub(recipe.timing.track_a_transition_start_ms) {
            assert!(
                kf.deck_a.eq_mid_db >= -0.5,
                "Vocal mid EQ was ducked to {} dB while singing!",
                kf.deck_a.eq_mid_db
            );
        }
    }
}

#[test]
fn test_automix_equal_power_and_bass_swap_planning_and_render() {
    use attune_core::schema::{
        AnalysisTier, CueZone, Frames20Hz, GlobalSummary, TrackAnalysisProfile, TrackId, ZoneType,
    };

    let frames_count = 100;
    let mut exit_energy = vec![[-12.0; 4]; frames_count];
    for i in 40..frames_count {
        let decay = (i - 40) as f32 * 0.8;
        exit_energy[i] = [(-12.0 - decay).max(-80.0); 4];
    }

    let exit_zone = CueZone {
        zone_type: ZoneType::EXIT,
        start_ms: 10000,
        end_ms: 15000,
        hop_ms: 50,
        beats_ms: vec![10000, 10375, 10750, 11125, 11500, 11875, 12250, 12625, 13000, 13375, 13750, 14125, 14500, 14875],
        downbeats_ms: vec![10000, 11500, 13000, 14500],
        phrase_boundaries_ms: vec![10000, 13000],
        sustained_pitch_regions: vec![],
        frames_20hz: Frames20Hz {
            f0_midi_cents: vec![0.0; frames_count],
            chroma_12: vec![[0.0; 12]; frames_count],
            energy_4band_db: exit_energy,
            aggression_curve: vec![0.55; frames_count],
            instrument_activations: vec![vec![0.0; 6]; frames_count],
        },
    };

    let mut entry_energy = vec![[-10.0; 4]; frames_count];
    for i in 0..10 {
        entry_energy[i] = [-60.0; 4];
    }

    let entry_zone = CueZone {
        zone_type: ZoneType::ENTRY,
        start_ms: 0,
        end_ms: 10000,
        hop_ms: 50,
        beats_ms: vec![0, 350, 700, 1050, 1400, 1750, 2100, 2450, 2800],
        downbeats_ms: vec![0, 1400, 2800],
        phrase_boundaries_ms: vec![0, 2800],
        sustained_pitch_regions: vec![],
        frames_20hz: Frames20Hz {
            f0_midi_cents: vec![0.0; frames_count],
            chroma_12: vec![[0.0; 12]; frames_count],
            energy_4band_db: entry_energy,
            aggression_curve: vec![0.58; frames_count],
            instrument_activations: vec![vec![0.0; 6]; frames_count],
        },
    };

    let track_a = TrackAnalysisProfile {
        schema_version: 1,
        track_id: TrackId {
            audio_hash: "synth_track_a".to_string(),
            duration_ms: 15000,
            canonical_slug: "synth_track_a".to_string(),
        },
        analysis_tier: AnalysisTier::FastHeadTail,
        global_summary: GlobalSummary {
            nominal_bpm: 80.0, // Folded to 160.0 BPM
            bpm_stability: 0.95,
            primary_key: "F Major".to_string(),
            mean_aggression_index: 0.55,
            macro_vibe_family: "POP_DANCE_GROOVE".to_string(),
            dominant_instruments: vec!["inst_synth_electronic".to_string(), "vocal_clean_melodic".to_string()],
        },
        cue_zones: vec![exit_zone],
    };

    let track_b = TrackAnalysisProfile {
        schema_version: 1,
        track_id: TrackId {
            audio_hash: "synth_track_b".to_string(),
            duration_ms: 15000,
            canonical_slug: "synth_track_b".to_string(),
        },
        analysis_tier: AnalysisTier::FastHeadTail,
        global_summary: GlobalSummary {
            nominal_bpm: 165.0, // Effective 165.0 BPM (diff 3.1% <= 6% threshold)
            bpm_stability: 0.95,
            primary_key: "C Major".to_string(),
            mean_aggression_index: 0.58,
            macro_vibe_family: "POP_DANCE_GROOVE".to_string(),
            dominant_instruments: vec!["inst_synth_electronic".to_string()],
        },
        cue_zones: vec![entry_zone],
    };

    let planner = TransitionPlanner::new();
    let recipe = planner.plan_transition(&track_a, &track_b);

    assert_eq!(recipe.archetype, TransitionArchetype::EqSwap);
    assert!(recipe.confidence_score >= 0.85);

    // Verify keyframes cover transition window
    assert!(!recipe.dsp_automation_keyframes.is_empty());

    // Check Equal-Power Property: At midpoint, G_a^2 + G_b^2 ~= 1.0 (within epsilon)
    let total_duration = recipe.timing.track_a_exit_end_ms.saturating_sub(recipe.timing.track_a_transition_start_ms);
    let mid_kf = recipe.dsp_automation_keyframes.iter().find(|kf| {
        (kf.offset_ms as f32 - (total_duration as f32 * 0.5)).abs() < 100.0
    });
    if let Some(kf) = mid_kf {
        let gain_a = 10.0f32.powf(kf.deck_a.gain_db / 20.0);
        let gain_b = 10.0f32.powf(kf.deck_b.gain_db / 20.0);
        let power = gain_a * gain_a + gain_b * gain_b;
        assert!(
            (power - 1.0).abs() < 0.1,
            "Midpoint power sum {} deviated from 1.0 (gain_a: {}, gain_b: {})",
            power,
            gain_a,
            gain_b
        );
    }

    // Check DJ Bass Swap: Deck B sub should be ducked at start, Deck A sub ducked at end
    let first_kf = recipe.dsp_automation_keyframes.first().unwrap();
    let last_kf = recipe.dsp_automation_keyframes.last().unwrap();
    assert!(first_kf.deck_b.eq_sub_db <= -18.0, "Deck B sub EQ should be ducked early");
    assert!(last_kf.deck_a.eq_sub_db <= -18.0, "Deck A sub EQ should be ducked late");

    // Test audio rendering with dummy buffers
    let sr = 44100;
    let buf_a = generate_test_tone(sr, 15.0, 349.23); // F4
    let buf_b = generate_test_tone(sr, 15.0, 261.63); // C4

    let renderer = TransitionRenderer::new();
    let temp_dir = std::env::temp_dir();
    let out_wav = temp_dir.join("attune_test_automix_eq_swap.wav");

    let res = renderer.render_to_wav(&buf_a, &buf_b, &recipe, &out_wav);
    assert!(res.is_ok(), "Rendering failed: {:?}", res.err());
    assert!(out_wav.exists());
    let _ = std::fs::remove_file(out_wav);
}



