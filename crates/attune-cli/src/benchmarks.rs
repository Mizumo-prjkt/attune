use hound::{WavSpec, WavWriter};
use std::f32::consts::PI;
use std::path::{Path, PathBuf};

/// Generates synthetic audio files modeling the 3 Gold Standard benchmarks
pub fn generate_benchmarks<P: AsRef<Path>>(dir: P) -> std::io::Result<Vec<(String, PathBuf, PathBuf)>> {
    let dir = dir.as_ref();
    std::fs::create_dir_all(dir)?;

    let sr = 44100u32;
    let mut cases = Vec::new();

    // -----------------------------------------------------------------------
    // Benchmark 1: Whitney (Track A) -> Mariah (Track B) Vocal Sustain
    // -----------------------------------------------------------------------
    let case1_a = dir.join("bm1_whitney_sustain.wav");
    let case1_b = dir.join("bm1_mariah_sustain.wav");

    // Track A: 4s audio ending with 2.5s sustained note at 440 Hz (A4)
    write_sine_tone(&case1_a, sr, 4.0, 440.0, true, 0.0)?;
    // Track B: 4s audio starting with 2.0s sustained note at 445 Hz (A4 + ~20 cents)
    write_sine_tone(&case1_b, sr, 4.0, 445.0, false, 0.0)?;

    cases.push(("bm1_sustain_morph".to_string(), case1_a, case1_b));

    // -----------------------------------------------------------------------
    // Benchmark 2: Ado (Track A) -> Punk/Rock (Track B) High Aggression
    // -----------------------------------------------------------------------
    let case2_a = dir.join("bm2_ado_aggressive.wav");
    let case2_b = dir.join("bm2_punk_aggressive.wav");

    // Both tracks with distorted high-roughness audio (odd harmonics + noise + steep transients)
    write_aggressive_riff(&case2_a, sr, 5.0, 160.0)?;
    write_aggressive_riff(&case2_b, sr, 5.0, 165.0)?;

    cases.push(("bm2_aggressive_riff_handoff".to_string(), case2_a, case2_b));

    // -----------------------------------------------------------------------
    // Benchmark 3: Slow Rock (Track A) -> Upbeat Pop (Track B)
    // -----------------------------------------------------------------------
    let case3_a = dir.join("bm3_slow_rock.wav");
    let case3_b = dir.join("bm3_pop_groove.wav");

    // Slow Rock: gentle clean chords, low aggression (120 BPM)
    write_clean_groove(&case3_a, sr, 6.0, 120.0, 0.2)?;
    // Pop: 4-on-the-floor beat, moderate aggression (120 BPM)
    write_clean_groove(&case3_b, sr, 6.0, 120.0, 0.5)?;

    cases.push(("bm3_slow_rock_to_pop_eq_swap".to_string(), case3_a, case3_b));

    // -----------------------------------------------------------------------
    // Benchmark 3b: Aggressive Rock (Track A) -> Slow Ballad (Track B)
    // -----------------------------------------------------------------------
    let case3b_a = dir.join("bm3b_aggressive_rock.wav");
    let case3b_b = dir.join("bm3b_slow_ballad.wav");

    write_aggressive_riff(&case3b_a, sr, 5.0, 140.0)?;
    write_clean_groove(&case3b_b, sr, 5.0, 75.0, 0.15)?;

    cases.push(("bm3b_high_delta_agg_tail_freeze".to_string(), case3b_a, case3b_b));

    Ok(cases)
}

fn write_sine_tone(path: &Path, sr: u32, dur_sec: f32, freq: f32, is_outro: bool, grit: f32) -> std::io::Result<()> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).map_err(|e| std::io::Error::other(e))?;
    let total_samples = (dur_sec * sr as f32) as usize;

    for i in 0..total_samples {
        let t = i as f32 / sr as f32;
        let mut sample = (2.0 * PI * freq * t).sin() * 0.7;

        // Add 3rd harmonic for warmth
        sample += (2.0 * PI * (freq * 3.0) * t).sin() * 0.15;

        if grit > 0.0 {
            sample += (2.0 * PI * 1800.0 * t).sin() * grit;
        }

        let envelope = if is_outro {
            // Sustained note in the second half
            if t > 1.5 { 1.0 } else { (t / 1.5).min(1.0) * 0.8 }
        } else {
            // Sustained note in the first half
            if t < 2.5 { 1.0 } else { ((dur_sec - t) / 1.5).max(0.0) }
        };

        let val = (sample * envelope * 0.8 * 32767.0) as i16;
        writer.write_sample(val).unwrap();
        writer.write_sample(val).unwrap();
    }
    writer.finalize().unwrap();
    Ok(())
}

fn write_aggressive_riff(path: &Path, sr: u32, dur_sec: f32, bpm: f32) -> std::io::Result<()> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).map_err(|e| std::io::Error::other(e))?;
    let total_samples = (dur_sec * sr as f32) as usize;
    let beat_samples = (60.0 / bpm * sr as f32) as usize;

    for i in 0..total_samples {
        let t = i as f32 / sr as f32;
        let beat_pos = i % beat_samples;
        let beat_frac = beat_pos as f32 / beat_samples as f32;

        // Snare / transient punch at start of each beat
        let transient = (-15.0 * beat_frac).exp() * ((i as f32 * 0.3).sin() * 0.4);

        // Heavy distorted guitar power chord (E2 ~82.4 Hz + B2 ~123 Hz + E3 ~164 Hz + clipping)
        let mut chord = (2.0 * PI * 82.4 * t).sin()
            + (2.0 * PI * 123.4 * t).sin()
            + (2.0 * PI * 164.8 * t).sin()
            + (2.0 * PI * 1800.0 * t).sin() * 0.4;

        // Distortion hard clipping
        chord = (chord * 2.5).clamp(-0.85, 0.85);

        let mix = (chord * 0.6 + transient * 0.4).clamp(-1.0, 1.0);
        let val = (mix * 32767.0) as i16;
        writer.write_sample(val).unwrap();
        writer.write_sample(val).unwrap();
    }
    writer.finalize().unwrap();
    Ok(())
}

fn write_clean_groove(path: &Path, sr: u32, dur_sec: f32, bpm: f32, aggression_target: f32) -> std::io::Result<()> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).map_err(|e| std::io::Error::other(e))?;
    let total_samples = (dur_sec * sr as f32) as usize;
    let beat_samples = (60.0 / bpm * sr as f32) as usize;

    for i in 0..total_samples {
        let t = i as f32 / sr as f32;
        let beat_pos = i % beat_samples;
        let beat_frac = beat_pos as f32 / beat_samples as f32;

        // Clean low-end kick transient (decaying envelope on 55 Hz sine)
        let kick = (-12.0 * beat_frac).exp() * (2.0 * PI * 55.0 * t).sin() * 0.6;

        // Clean acoustic / Rhodes piano chord (C Major: 261.6 Hz, 329.6 Hz, 392.0 Hz)
        let chord = (2.0 * PI * 261.6 * t).sin() * 0.25
            + (2.0 * PI * 329.6 * t).sin() * 0.20
            + (2.0 * PI * 392.0 * t).sin() * 0.15;

        // Upbeat high-hat tick for pop
        let hat = if aggression_target > 0.35 {
            (-20.0 * ((beat_frac * 2.0) % 1.0)).exp() * (2.0 * PI * 6000.0 * t).sin() * 0.08
        } else {
            0.0
        };

        let mix = (kick + chord + hat).clamp(-1.0, 1.0);
        let val = (mix * 0.7 * 32767.0) as i16;
        writer.write_sample(val).unwrap();
        writer.write_sample(val).unwrap();
    }
    writer.finalize().unwrap();
    Ok(())
}
