use attune_core::audio::AudioBuffer;
use attune_core::engine::{DeckId, DeckState, DualDeckMixer, JitPlanner};
use attune_core::schema::{
    DeckAutomation, DspAutomationKeyframe, TransitionArchetype, TransitionRecipe, TransitionTiming,
};
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
fn test_deck_playback_and_controls() {
    let sr = 44100;
    let buf = generate_test_tone(sr, 2.0, 440.0);
    let mut mixer = DualDeckMixer::new(sr);

    mixer.load_track_a(buf);
    assert_eq!(mixer.active_deck, DeckId::DeckA);
    assert_eq!(mixer.deck_a.state, DeckState::Stopped);

    mixer.play();
    assert_eq!(mixer.deck_a.state, DeckState::Playing);
    assert!(mixer.is_playing());

    // Process 1 second of stereo audio
    let mut output = vec![0.0f32; sr as usize * 2];
    mixer.process_interleaved_block(&mut output);

    let pos = mixer.deck_a.current_position_ms();
    assert!((pos as i64 - 1000).abs() <= 50, "Position was {}", pos);

    mixer.pause();
    assert_eq!(mixer.deck_a.state, DeckState::Paused);
}

#[test]
fn test_dual_deck_mixer_automated_transition() {
    let sr = 44100;
    let buf_a = generate_test_tone(sr, 4.0, 440.0);
    let buf_b = generate_test_tone(sr, 4.0, 554.37); // C#5
    let mut mixer = DualDeckMixer::new(sr);

    mixer.load_track_a(buf_a);
    mixer.load_track_b(buf_b);
    mixer.play();

    // 1-second transition recipe from A to B
    let recipe = TransitionRecipe {
        schema_version: 1,
        track_a_hash: "test_a".to_string(),
        track_b_hash: "test_b".to_string(),
        archetype: TransitionArchetype::RiffHandoff,
        confidence_score: 0.95,
        timing: TransitionTiming {
            track_a_transition_start_ms: 1000,
            track_a_exit_end_ms: 2000,
            track_b_entry_start_ms: 0,
            track_b_native_lock_ms: 1000,
        },
        optional_stem_bridge_uri: None,
        dsp_automation_keyframes: vec![
            DspAutomationKeyframe {
                offset_ms: 0,
                deck_a: DeckAutomation {
                    gain_db: 0.0,
                    ..DeckAutomation::default()
                },
                deck_b: DeckAutomation {
                    gain_db: -60.0,
                    ..DeckAutomation::default()
                },
            },
            DspAutomationKeyframe {
                offset_ms: 1000,
                deck_a: DeckAutomation {
                    gain_db: -60.0,
                    ..DeckAutomation::default()
                },
                deck_b: DeckAutomation {
                    gain_db: 0.0,
                    ..DeckAutomation::default()
                },
            },
        ],
    };

    mixer.trigger_transition(recipe);
    assert!(mixer.is_transitioning());
    assert_eq!(mixer.deck_a.state, DeckState::Transitioning);
    assert_eq!(mixer.deck_b.state, DeckState::Transitioning);

    // Process 1.5 seconds of audio (should complete the 1.0s transition)
    let frames = (1.5 * sr as f32) as usize;
    let mut output = vec![0.0f32; frames * 2];
    mixer.process_interleaved_block(&mut output);

    // Transition should have completed, handing off control seamlessly to the playing deck!
    assert!(!mixer.is_transitioning());
    assert_eq!(mixer.active_deck, DeckId::DeckA);
    assert_eq!(mixer.deck_a.state, DeckState::Playing);
    assert_eq!(mixer.deck_b.state, DeckState::Stopped);

    // Verify output has valid energy (no NaNs or extreme clipping)
    for &s in &output {
        assert!(!s.is_nan());
        assert!(!s.is_infinite());
        assert!(s.abs() <= 1.2);
    }
}

#[test]
fn test_jit_planner_execution_speed() {
    let sr = 44100;
    let buf_a = generate_test_tone(sr, 5.0, 440.0);
    let buf_b = generate_test_tone(sr, 5.0, 442.0);

    let jit = JitPlanner::new();
    let start = std::time::Instant::now();
    let recipe = jit.plan_jit(&buf_a, "track_a", &buf_b, "track_b");
    let elapsed = start.elapsed();

    println!("JIT plan executed in: {:?}", elapsed);
    assert!(recipe.confidence_score >= 0.0);
    let max_allowed_ms = if cfg!(debug_assertions) { 2000 } else { 200 };
    assert!(
        elapsed.as_millis() < max_allowed_ms,
        "JIT took {} ms (limit: {} ms)",
        elapsed.as_millis(),
        max_allowed_ms
    );
}

#[test]
fn test_sample_rate_resampling_accuracy() {
    let track_sr = 44100;
    let device_sr = 48000;

    // 440 Hz reference tone in a 44.1 kHz audio buffer
    let buf = generate_test_tone(track_sr, 3.0, 440.0);
    let mut mixer = DualDeckMixer::new(device_sr);

    mixer.load_track_a(buf);
    mixer.deck_a.seek_ms(500);
    assert_eq!(mixer.deck_a.current_position_ms(), 500);

    mixer.play();

    // Process exactly 1.0 second on 48 kHz hardware (48,000 frames)
    let mut output = vec![0.0f32; device_sr as usize * 2];
    mixer.process_interleaved_block(&mut output);

    // Speed check: position should have advanced by exactly 1000 ms (landing at 1500 ms)
    let pos = mixer.deck_a.current_position_ms();
    assert!(
        (pos as i64 - 1500).abs() <= 10,
        "Expected position ~1500ms, got {}ms (speed error!)",
        pos
    );

    // Pitch check: measure wavelength in the 48 kHz output stream.
    // 440 Hz tone in 48 kHz output has period = 48000 / 440 = 109.09 samples.
    // (If played without resampling, period would be 44100 / 440 = 100.2 samples -> 1.5 semitones sharp!)
    let mut zero_crossings = Vec::new();
    let left_samples: Vec<f32> = output.chunks_exact(2).map(|c| c[0]).collect();
    for i in 1..left_samples.len().min(4800) {
        if left_samples[i - 1] <= 0.0 && left_samples[i] > 0.0 {
            zero_crossings.push(i);
        }
    }
    assert!(zero_crossings.len() >= 10);
    let periods: Vec<f64> = zero_crossings.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
    let avg_period: f64 = periods.iter().sum::<f64>() / periods.len() as f64;

    println!("Detected 48 kHz output period: {:.2} samples (expected 109.09)", avg_period);
    assert!(
        (avg_period - 109.09).abs() <= 1.0,
        "Expected period ~109.09 samples (440 Hz), got {:.2} samples (pitch error!)",
        avg_period
    );
}
