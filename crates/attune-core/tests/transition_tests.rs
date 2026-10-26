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
