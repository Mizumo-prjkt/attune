use crate::audio::AudioBuffer;
use crate::schema::{
    AnalysisTier, CueZone, Frames20Hz, GlobalSummary, SustainedPitchRegion, TrackAnalysisProfile,
    TrackId, ZoneType,
};
use realfft::RealFftPlanner;
use std::f32::consts::PI;

pub struct AudioProfiler {
    pub hop_size_ms: u32,
}

impl Default for AudioProfiler {
    fn default() -> Self {
        Self { hop_size_ms: 50 } // 20 Hz
    }
}

impl AudioProfiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Profile an entire track (or head/tail cue zones) into a complete .tap profile
    pub fn profile_track(
        &self,
        audio: &AudioBuffer,
        canonical_slug: &str,
        tier: AnalysisTier,
    ) -> TrackAnalysisProfile {
        let sr = audio.sample_rate;
        let dur_ms = audio.duration_ms();
        let mono = audio.to_mono();
        let audio_hash = audio.compute_hash();

        let track_id = TrackId {
            audio_hash,
            duration_ms: dur_ms,
            canonical_slug: canonical_slug.to_string(),
        };

        // Detect audible boundaries (-45 dBFS threshold) to completely eliminate dead air / pre-gap / post-gap
        let audible_start = audio.find_audible_start_ms(-45.0);
        let audible_end = audio.find_audible_end_ms(-45.0).max(audible_start + 1000);
        let audible_dur = audible_end - audible_start;

        // Define Cue Zones strictly within audible boundaries:
        // Entry: [audible_start, min(audible_start + 45s, audible_end)]
        // Exit: [max(audible_start, audible_end - 45s), audible_end]
        let mut cue_zones = Vec::new();

        let entry_end_ms = (audible_start + 45_000).min(audible_end);
        let exit_start_ms = if audible_dur > 45_000 {
            audible_end.saturating_sub(45_000).max(audible_start)
        } else {
            audible_start + audible_dur / 2
        };

        // Multi-core parallel extraction of Entry and Exit cue zones via Rayon
        let (entry_zone, exit_zone) = rayon::join(
            || self.extract_cue_zone(&mono, sr, audible_start, entry_end_ms, ZoneType::ENTRY),
            || self.extract_cue_zone(&mono, sr, exit_start_ms, audible_end, ZoneType::EXIT),
        );

        cue_zones.push(entry_zone);
        cue_zones.push(exit_zone);

        // Global summary
        let global_summary = self.compute_global_summary(&cue_zones);

        TrackAnalysisProfile {
            schema_version: 1,
            track_id,
            analysis_tier: tier,
            global_summary,
            cue_zones,
        }
    }

    /// Extract all 5 layers within a specific time window [start_ms, end_ms]
    pub fn extract_cue_zone(
        &self,
        mono: &[f32],
        sr: u32,
        start_ms: u64,
        end_ms: u64,
        zone_type: ZoneType,
    ) -> CueZone {
        let start_sample = ((start_ms as f64 / 1000.0) * sr as f64) as usize;
        let end_sample = ((end_ms as f64 / 1000.0) * sr as f64) as usize;
        let clamped_end = end_sample.min(mono.len());
        let zone_samples = if start_sample < clamped_end {
            &mono[start_sample..clamped_end]
        } else {
            &[]
        };

        let hop_size = ((self.hop_size_ms as f64 / 1000.0) * sr as f64) as usize;
        let fft_len = 2048.min(zone_samples.len().max(256)).next_power_of_two();

        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_len);
        let hann_window: Vec<f32> = (0..fft_len)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (fft_len as f32 - 1.0)).cos()))
            .collect();

        let num_frames = if zone_samples.len() >= fft_len {
            (zone_samples.len() - fft_len) / hop_size + 1
        } else {
            0
        };

        let mut f0_midi_cents = Vec::with_capacity(num_frames);
        let mut voicing_confs = Vec::with_capacity(num_frames);
        let mut chroma_12 = Vec::with_capacity(num_frames);
        let mut energy_4band_db = Vec::with_capacity(num_frames);
        let mut aggression_curve = Vec::with_capacity(num_frames);
        let mut instrument_activations = Vec::with_capacity(num_frames);

        let mut input_buf = r2c.make_input_vec();
        let mut output_spectrum = r2c.make_output_vec();

        let mut prev_spec = vec![0.0f32; fft_len / 2 + 1];
        let mut mag_spec = vec![0.0f32; fft_len / 2 + 1];
        let mut onset_strengths = Vec::with_capacity(num_frames);
        let mut prev_midi = 0.0f32;

        for frame_idx in 0..num_frames {
            let offset = frame_idx * hop_size;
            let frame_slice = &zone_samples[offset..offset + fft_len];

            for i in 0..fft_len {
                input_buf[i] = frame_slice[i] * hann_window[i];
            }

            // FFT
            r2c.process(&mut input_buf, &mut output_spectrum).ok();

            for i in 0..mag_spec.len() {
                mag_spec[i] = output_spectrum[i].norm();
            }

            // Layer 1: Pitch & Voicing via Autocorrelation / Peak Picking
            let (pitch_hz, voicing) = estimate_pitch_autocorr(frame_slice, sr);
            let midi_cents = if pitch_hz > 30.0 && voicing > 0.4 {
                6900.0 + 1200.0 * (pitch_hz / 440.0).log2()
            } else {
                0.0
            };
            f0_midi_cents.push(midi_cents);
            voicing_confs.push(voicing);

            let pitch_jitter = if midi_cents > 1000.0 && prev_midi > 1000.0 {
                (midi_cents - prev_midi).abs()
            } else {
                0.0
            };
            if midi_cents > 1000.0 {
                prev_midi = midi_cents;
            }

            // Layer 2: 12-Bin Chroma
            let chroma = compute_chroma(&mag_spec, sr, fft_len);
            chroma_12.push(chroma);

            // Layer 3: Onset strength (spectral flux)
            let mut flux = 0.0f32;
            for i in 0..mag_spec.len() {
                let diff = mag_spec[i] - prev_spec[i];
                if diff > 0.0 {
                    flux += diff;
                }
            }
            onset_strengths.push(flux);
            prev_spec.copy_from_slice(&mag_spec);

            // Layer 4: 4-Band Energy (Sub: 20-150, Low-Mid: 150-600, Vocal: 600-4000, Air: 4000-20000)
            let (e4_power, e4_db) = compute_4band_energy(&mag_spec, sr, fft_len);
            energy_4band_db.push(e4_db);

            // Layer 5: Aggression & Timbre
            let attack_slope = if flux > 0.1 { (flux * 2.0).min(1.0) } else { 0.0 };
            let roughness = compute_spectral_roughness(&mag_spec, sr, fft_len);
            let total_power = e4_power[0] + e4_power[1] + e4_power[2] + e4_power[3];
            let mid_high_ratio = if total_power > 1e-6 {
                ((e4_power[2] + e4_power[3]) / total_power).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let aggression = (0.25 * attack_slope + 0.50 * roughness + 0.25 * mid_high_ratio).clamp(0.0, 1.0);
            aggression_curve.push(aggression);

            // Instrument activations:
            // [distorted_guitar, clean_acoustic_piano, brass_horns, synth, clean_vocal, grit_vocal]
            let inst = detect_instrument_signatures(
                &mag_spec,
                sr,
                fft_len,
                voicing,
                midi_cents,
                pitch_jitter,
                e4_power,
                roughness,
            );
            instrument_activations.push(inst);
        }

        // Layer 1 post-processing: Sustained Pitch / Fermata Regions
        let sustained_pitch_regions = detect_sustained_pitch_regions(
            &f0_midi_cents,
            &voicing_confs,
            start_ms,
            self.hop_size_ms,
        );

        // Layer 3: Dynamic Rhythm, Beats & Phrase Boundaries
        let (beats_ms, downbeats_ms, phrase_boundaries_ms) = estimate_beats_and_phrases(
            &onset_strengths,
            start_ms,
            self.hop_size_ms,
        );

        let frames_20hz = Frames20Hz {
            f0_midi_cents,
            chroma_12,
            energy_4band_db,
            aggression_curve,
            instrument_activations,
        };

        CueZone {
            zone_type,
            start_ms,
            end_ms,
            hop_ms: self.hop_size_ms,
            beats_ms,
            downbeats_ms,
            phrase_boundaries_ms,
            sustained_pitch_regions,
            frames_20hz,
        }
    }

    fn compute_global_summary(&self, zones: &[CueZone]) -> GlobalSummary {
        let mut total_aggression = 0.0f32;
        let mut count = 0;
        let mut chroma_acc = [0.0f32; 12];
        let mut inst_acc = [0.0f32; 6];

        for zone in zones {
            for &agg in &zone.frames_20hz.aggression_curve {
                total_aggression += agg;
                count += 1;
            }
            for ch in &zone.frames_20hz.chroma_12 {
                for i in 0..12 {
                    chroma_acc[i] += ch[i];
                }
            }
            for inst in &zone.frames_20hz.instrument_activations {
                for i in 0..6.min(inst.len()) {
                    inst_acc[i] += inst[i];
                }
            }
        }

        let mean_agg = if count > 0 { total_aggression / count as f32 } else { 0.5 };

        // Best key from chroma
        let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        let mut max_idx = 0;
        let mut max_val = 0.0f32;
        for (i, &val) in chroma_acc.iter().enumerate() {
            if val > max_val {
                max_val = val;
                max_idx = i;
            }
        }
        let primary_key = format!("{} Major", note_names[max_idx]);

        // BPM from beat intervals
        let mut bpm = 120.0f32;
        for zone in zones {
            if zone.beats_ms.len() >= 4 {
                let intervals: Vec<f32> = zone
                    .beats_ms
                    .windows(2)
                    .map(|w| (w[1] - w[0]) as f32)
                    .filter(|&dt| (250.0..=1500.0).contains(&dt))
                    .collect();
                if !intervals.is_empty() {
                    let avg_dt = intervals.iter().sum::<f32>() / intervals.len() as f32;
                    let mut raw_bpm = 60_000.0 / avg_dt;
                    while raw_bpm > 175.0 {
                        raw_bpm /= 2.0;
                    }
                    while raw_bpm < 75.0 && raw_bpm > 30.0 {
                        raw_bpm *= 2.0;
                    }
                    bpm = raw_bpm;
                    break;
                }
            }
        }

        let macro_vibe_family = if mean_agg >= 0.70 {
            "AGGRESSIVE_ROCK_PUNK_JROCK".to_string()
        } else if mean_agg >= 0.40 {
            "POP_DANCE_GROOVE".to_string()
        } else {
            "SLOW_BALLAD_SOUL".to_string()
        };

        let inst_tags = [
            "inst_distorted_guitar",
            "inst_clean_acoustic_guitar_piano",
            "inst_brass_horns",
            "inst_synth_electronic",
            "vocal_clean_melodic",
            "vocal_grit_fry_growl",
        ];
        let mut dominant_instruments = Vec::new();
        for (i, &val) in inst_acc.iter().enumerate() {
            if count > 0 && (val / count as f32) > 0.25 {
                dominant_instruments.push(inst_tags[i].to_string());
            }
        }
        if dominant_instruments.is_empty() {
            dominant_instruments.push("vocal_clean_melodic".to_string());
        }

        GlobalSummary {
            nominal_bpm: (bpm * 10.0).round() / 10.0,
            bpm_stability: 0.85,
            primary_key,
            mean_aggression_index: (mean_agg * 100.0).round() / 100.0,
            macro_vibe_family,
            dominant_instruments,
        }
    }
}

// ---------------------------------------------------------------------------
// DSP Helper Implementations
// ---------------------------------------------------------------------------

fn estimate_pitch_autocorr(frame: &[f32], sr: u32) -> (f32, f32) {
    let min_pitch = 65.0f32;  // C2
    let max_pitch = 1000.0f32; // ~B5

    let min_lag = (sr as f32 / max_pitch) as usize;
    let max_lag = (sr as f32 / min_pitch) as usize;

    if frame.len() < max_lag * 2 {
        return (0.0, 0.0);
    }

    let mut energy = 0.0f32;
    for &s in frame {
        energy += s * s;
    }
    if energy < 1e-4 {
        return (0.0, 0.0);
    }

    let mut best_lag = 0;
    let mut best_corr = -1.0f32;

    // Fast coarse search (step 2)
    for lag in (min_lag..=max_lag).step_by(2) {
        let mut sum = 0.0f32;
        let mut n = 0;
        for i in (0..(frame.len() - lag)).step_by(2) {
            sum += frame[i] * frame[i + lag];
            n += 1;
        }
        let norm_corr = if n > 0 { sum / (energy * 0.5 + 1e-6) } else { 0.0 };
        if norm_corr > best_corr {
            best_corr = norm_corr;
            best_lag = lag;
        }
    }

    // Local fine search (+/- 1 lag)
    if best_lag > min_lag && best_lag < max_lag {
        for lag in [best_lag.saturating_sub(1), best_lag + 1] {
            let mut sum = 0.0f32;
            let mut n = 0;
            for i in (0..(frame.len() - lag)).step_by(2) {
                sum += frame[i] * frame[i + lag];
                n += 1;
            }
            let norm_corr = if n > 0 { sum / (energy * 0.5 + 1e-6) } else { 0.0 };
            if norm_corr > best_corr {
                best_corr = norm_corr;
                best_lag = lag;
            }
        }
    }

    if best_lag > 0 && best_corr > 0.35 {
        let pitch_hz = sr as f32 / best_lag as f32;
        let confidence = (best_corr * 1.5).clamp(0.0, 1.0);
        (pitch_hz, confidence)
    } else {
        (0.0, 0.0)
    }
}

fn compute_chroma(mag_spec: &[f32], sr: u32, fft_len: usize) -> [f32; 12] {
    let mut chroma = [0.0f32; 12];
    let num_bins = mag_spec.len();

    // Map bins to 12 chroma semitones across 6 octaves (C1: ~32.7 Hz to B6: ~1975 Hz)
    for bin in 2..num_bins {
        let freq = (bin as f32 / fft_len as f32) * sr as f32;
        if !(30.0..=2500.0).contains(&freq) {
            continue;
        }
        let midi = 69.0 + 12.0 * (freq / 440.0).log2();
        let semitone = (midi.round() as i32).rem_euclid(12) as usize;
        chroma[semitone] += mag_spec[bin];
    }

    // L2 Normalize
    let mut norm = 0.0f32;
    for &val in &chroma {
        norm += val * val;
    }
    norm = norm.sqrt();
    if norm > 1e-6 {
        for val in &mut chroma {
            *val /= norm;
        }
    }

    chroma
}

fn compute_4band_energy(mag_spec: &[f32], sr: u32, fft_len: usize) -> ([f32; 4], [f32; 4]) {
    let mut power = [0.0f32; 4];
    let num_bins = mag_spec.len();

    // Sub: 20-150, Low-Mid: 150-600, Vocal: 600-4000, Air: 4000-20000
    for bin in 1..num_bins {
        let freq = (bin as f32 / fft_len as f32) * sr as f32;
        let p = mag_spec[bin] * mag_spec[bin];

        if (20.0..150.0).contains(&freq) {
            power[0] += p;
        } else if (150.0..600.0).contains(&freq) {
            power[1] += p;
        } else if (600.0..4000.0).contains(&freq) {
            power[2] += p;
        } else if (4000.0..=20000.0).contains(&freq) {
            power[3] += p;
        }
    }

    let mut db = [0.0f32; 4];
    for (i, &p) in power.iter().enumerate() {
        db[i] = if p > 1e-9 { 10.0 * p.log10() } else { -90.0 };
    }

    (power, db)
}

fn compute_spectral_roughness(mag_spec: &[f32], sr: u32, fft_len: usize) -> f32 {
    let mut count = 0;
    let mut sum = 0.0f32;
    let mut geom_log = 0.0f32;

    for bin in 1..mag_spec.len() {
        let freq = (bin as f32 / fft_len as f32) * sr as f32;
        if (800.0..=3500.0).contains(&freq) {
            let val = mag_spec[bin] + 1e-6;
            sum += val;
            geom_log += val.ln();
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let arith = sum / count as f32;
    // Low energy in mid-range band has zero distortion roughness
    if arith < 0.012 {
        return 0.0;
    }

    let geom = (geom_log / count as f32).exp();
    let flatness = (geom / arith).clamp(0.0, 1.0);
    let presence_weight = ((arith - 0.012) * 20.0).clamp(0.0, 1.0);
    flatness * presence_weight
}

fn detect_instrument_signatures(
    _mag_spec: &[f32],
    _sr: u32,
    _fft_len: usize,
    voicing: f32,
    midi_cents: f32,
    pitch_jitter: f32,
    e4_power: [f32; 4],
    roughness: f32,
) -> Vec<f32> {
    // 0: distorted_guitar
    // 1: clean_acoustic_guitar_piano
    // 2: brass_horns
    // 3: synth_electronic
    // 4: vocal_clean_melodic
    // 5: vocal_grit_fry_growl

    let total_power = e4_power[0] + e4_power[1] + e4_power[2] + e4_power[3] + 1e-9;
    let air_to_vocal = e4_power[3] / (e4_power[2] + 1e-9);
    let lowmid_presence = e4_power[1] / total_power;

    // Distorted guitar: heavy distortion roughness, heavy low-mid, but cabinet roll-off in air band (< 0.22)
    let dist_guitar = if roughness > 0.45 && air_to_vocal < 0.22 && lowmid_presence > 0.25 {
        roughness
    } else {
        0.05
    };

    // Synth electronic: high air presence (sparkle), periodic saw/pulse combs, low cabinet roll-off
    let synth = if air_to_vocal >= 0.20 || (roughness < 0.35 && e4_power[0] / total_power > 0.20) {
        0.75
    } else {
        0.10
    };

    // Clean acoustic / piano: low roughness, decaying harmonics, sharp attacks
    let clean_piano = if roughness < 0.20 && voicing < 0.55 && lowmid_presence > 0.30 {
        0.70
    } else {
        0.10
    };

    // Brass: strong formant peak around 1-2.5 kHz
    let brass = if roughness > 0.25 && roughness < 0.45 && e4_power[2] / total_power > 0.45 {
        0.45
    } else {
        0.05
    };

    // Clean vocal: strong voicing, audible pitch, smooth pitch contour (jitter <= 45 cents)
    let clean_vocal = if voicing >= 0.50 && midi_cents > 1000.0 && pitch_jitter <= 45.0 {
        voicing
    } else {
        0.10
    };

    // Vocal grit / growl / fry: high pitch jitter (> 60 cents), raspy roughness, vocal frequency range
    let grit_vocal = if voicing >= 0.45 && midi_cents > 1000.0 && pitch_jitter > 60.0 && roughness >= 0.40 {
        0.85
    } else {
        0.05
    };

    vec![dist_guitar, clean_piano, brass, synth, clean_vocal, grit_vocal]
}

fn detect_sustained_pitch_regions(
    f0_midi_cents: &[f32],
    voicing_confs: &[f32],
    start_ms: u64,
    hop_ms: u32,
) -> Vec<SustainedPitchRegion> {
    let mut regions = Vec::new();
    let min_frames = (800 / hop_ms) as usize; // >= 800 ms

    let mut current_start = None;
    let mut current_cents = Vec::new();
    let mut current_confs = Vec::new();

    let mut finalize_region = |start_idx: usize, end_idx: usize, cents_slice: &[f32], confs_slice: &[f32]| {
        if cents_slice.len() >= min_frames {
            let mean = cents_slice.iter().sum::<f32>() / cents_slice.len() as f32;
            let var = cents_slice.iter().map(|&c| (c - mean).powi(2)).sum::<f32>()
                / cents_slice.len() as f32;
            let std_cents = var.sqrt();

            if std_cents <= 50.0 {
                let avg_conf = confs_slice.iter().sum::<f32>() / confs_slice.len() as f32;
                let r_start_ms = start_ms + (start_idx as u64 * hop_ms as u64);
                let r_end_ms = start_ms + (end_idx as u64 * hop_ms as u64);

                regions.push(SustainedPitchRegion {
                    start_ms: r_start_ms,
                    end_ms: r_end_ms,
                    mean_midi_cents: mean,
                    pitch_std_cents: std_cents,
                    voicing_confidence: avg_conf,
                    formant_f1_hz: 480.0,
                    formant_f2_hz: 1150.0,
                    percussive_leakage_db: -30.0,
                });
            }
        }
    };

    for (i, (&cents, &conf)) in f0_midi_cents.iter().zip(voicing_confs.iter()).enumerate() {
        let is_voiced = conf >= 0.50 && cents > 1000.0;

        if is_voiced {
            if current_start.is_none() {
                current_start = Some(i);
            }
            current_cents.push(cents);
            current_confs.push(conf);
        } else if let Some(start_idx) = current_start {
            finalize_region(start_idx, i, &current_cents, &current_confs);
            current_start = None;
            current_cents.clear();
            current_confs.clear();
        }
    }

    if let Some(start_idx) = current_start {
        finalize_region(start_idx, f0_midi_cents.len(), &current_cents, &current_confs);
    }

    regions
}

fn estimate_beats_and_phrases(
    onset_strengths: &[f32],
    start_ms: u64,
    hop_ms: u32,
) -> (Vec<u64>, Vec<u64>, Vec<u64>) {
    let mut beats = Vec::new();
    let mut downbeats = Vec::new();
    let mut phrases = Vec::new();

    if onset_strengths.len() < 20 {
        return (beats, downbeats, phrases);
    }

    let hop_sec = hop_ms as f64 / 1000.0;
    let min_bpm = 70.0f64;
    let max_bpm = 185.0f64;

    let min_lag = (60.0 / (max_bpm * hop_sec)).floor() as usize;
    let max_lag = (60.0 / (min_bpm * hop_sec)).ceil() as usize;

    // Mean-center onset envelope
    let mean = onset_strengths.iter().sum::<f32>() / onset_strengths.len() as f32;
    let env: Vec<f64> = onset_strengths
        .iter()
        .map(|&x| (x - mean).max(0.0) as f64)
        .collect();

    let mut corr_vals = vec![0.0f64; max_lag + 2];

    for lag in min_lag..=max_lag {
        let mut sum = 0.0f64;
        let mut count = 0;
        for i in 0..(env.len() - lag) {
            sum += env[i] * env[i + lag];
            count += 1;
        }
        let r = if count > 0 { sum / count as f64 } else { 0.0 };
        let bpm = 60.0 / (lag as f64 * hop_sec);
        let prior = (-0.5 * ((bpm - 130.0) / 45.0).powi(2)).exp();
        corr_vals[lag] = r * (1.0 + 0.25 * prior);
    }

    let mut best_lag_f = 10.0f64;
    let mut max_corr = -1.0f64;

    for lag in (min_lag + 1)..max_lag {
        let r = corr_vals[lag];
        if r > max_corr && r > corr_vals[lag - 1] && r > corr_vals[lag + 1] {
            max_corr = r;
            let y0 = corr_vals[lag - 1];
            let y1 = corr_vals[lag];
            let y2 = corr_vals[lag + 1];
            let denom = 2.0 * (2.0 * y1 - y0 - y2);
            let delta = if denom.abs() > 1e-6 {
                (y0 - y2) / denom
            } else {
                0.0
            };
            best_lag_f = lag as f64 + delta;
        }
    }

    if max_corr <= 0.0 {
        let mut best_idx = min_lag;
        let mut best_val = -1.0;
        for (i, &val) in corr_vals.iter().enumerate().skip(min_lag).take(max_lag - min_lag + 1) {
            if val > best_val {
                best_val = val;
                best_idx = i;
            }
        }
        best_lag_f = best_idx as f64;
    }

    let beat_period_ms = (best_lag_f * hop_ms as f64).clamp(300.0, 1000.0);

    // Phase locking: find best phase alignment offset
    let num_phase_bins = 16;
    let mut best_phase_ms = 0.0f64;
    let mut best_phase_score = -1.0f64;

    for p in 0..num_phase_bins {
        let phase_offset_ms = (p as f64 / num_phase_bins as f64) * beat_period_ms;
        let mut score = 0.0f64;
        let mut t = phase_offset_ms;
        while t < (onset_strengths.len() as f64 * hop_ms as f64) {
            let frame = (t / hop_ms as f64).round() as usize;
            if frame < onset_strengths.len() {
                score += onset_strengths[frame] as f64;
            }
            t += beat_period_ms;
        }
        if score > best_phase_score {
            best_phase_score = score;
            best_phase_ms = phase_offset_ms;
        }
    }

    let total_ms = onset_strengths.len() as f64 * hop_ms as f64;
    let mut t = best_phase_ms;
    while t < total_ms {
        beats.push(start_ms + t.round() as u64);
        t += beat_period_ms;
    }

    for (idx, &beat) in beats.iter().enumerate() {
        if idx % 4 == 0 {
            downbeats.push(beat);
        }
        if idx % 16 == 0 {
            phrases.push(beat);
        }
    }

    (beats, downbeats, phrases)
}
