//! Generative Melodic Fill & Musical Bridge Engine (ARCHETYPE_6_GENERATIVE_MELODIC_FILL)
//!
//! Procedurally generates a beat-synchronized, harmonically bridging melodic fill
//! across outgoing Track A and incoming Track B for upbeat, Eurobeat, electronic,
//! and rhythm-driven tracks.

use crate::audio::AudioBuffer;
use crate::schema::{CueZone, TrackAnalysisProfile};
use std::f32::consts::PI;

/// Style of the generative fill
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MelodicFillStyle {
    /// Quintessential Eurobeat / Trance dual-detuned PolyBLEP saw with resonant filter riser
    EurobeatSuperSaw,
    /// Punchy FM brass lead with snappy attack and 16th-note arpeggiator
    FmPluckRiser,
    /// Hybrid blend: synthesized lead arp + outgoing melodic motif stutter extension
    HybridMotifAndArp,
}

/// Configuration parameters for the generative fill
#[derive(Debug, Clone)]
pub struct GenerativeFillConfig {
    pub duration_bars: u32,
    pub tempo_a_bpm: f32,
    pub tempo_b_bpm: f32,
    pub root_a_pitch_class: usize, // 0 = C, 1 = C#, ..., 11 = B
    pub root_b_pitch_class: usize,
    pub is_minor_a: bool,
    pub is_minor_b: bool,
    pub style: MelodicFillStyle,
    pub sample_rate: u32,
    pub master_gain: f32,
    pub include_noise_riser: bool,
    pub include_snare_roll: bool,
}

impl Default for GenerativeFillConfig {
    fn default() -> Self {
        Self {
            duration_bars: 1,
            tempo_a_bpm: 160.0,
            tempo_b_bpm: 171.4,
            root_a_pitch_class: 5, // F
            root_b_pitch_class: 0, // C
            is_minor_a: false,
            is_minor_b: false,
            style: MelodicFillStyle::EurobeatSuperSaw,
            sample_rate: 44100,
            master_gain: 0.85,
            include_noise_riser: true,
            include_snare_roll: true,
        }
    }
}

/// Single musical note event in the procedural fill sequence
#[derive(Debug, Clone, Copy)]
pub struct NoteEvent {
    pub start_sample: usize,
    pub duration_samples: usize,
    pub midi_pitch: f32,
    pub velocity: f32,
    pub is_resolution: bool,
}

/// Unconditionally stable 2-pole TPT (Topology-Preserving Transform) State Variable Filter
#[derive(Debug, Clone, Copy, Default)]
pub struct TptSvf {
    pub s1: f32,
    pub s2: f32,
}

impl TptSvf {
    pub fn new() -> Self {
        Self { s1: 0.0, s2: 0.0 }
    }

    /// Process one sample through the filter. Returns (low_pass, band_pass, high_pass)
    #[inline]
    pub fn process(&mut self, x: f32, cutoff_hz: f32, q: f32, sr: f32) -> (f32, f32, f32) {
        let clamped_cutoff = cutoff_hz.clamp(20.0, sr * 0.45);
        let g = (PI * clamped_cutoff / sr).tan();
        let r = 1.0 / (2.0 * q.max(0.1));
        let denom = 1.0 + 2.0 * r * g + g * g;

        let hp = (x - (2.0 * r + g) * self.s1 - self.s2) / denom;
        let bp = g * hp + self.s1;
        let lp = g * bp + self.s2;

        self.s1 = bp + g * hp;
        self.s2 = lp + g * bp;

        (lp, bp, hp)
    }
}

/// The Generative Melodic Fill Engine
pub struct GenerativeFillEngine;


impl GenerativeFillEngine {
    /// Analyze outgoing Track A and incoming Track B to create an optimal fill configuration
    pub fn create_config(
        track_a: &TrackAnalysisProfile,
        track_b: &TrackAnalysisProfile,
        bars: u32,
        style: MelodicFillStyle,
        sample_rate: u32,
    ) -> GenerativeFillConfig {
        let mut bpm_a = track_a.global_summary.nominal_bpm.max(40.0);
        let mut bpm_b = track_b.global_summary.nominal_bpm.max(40.0);

        // Fold octave tempos into upbeat range [130, 200]
        while bpm_a < 125.0 {
            bpm_a *= 2.0;
        }
        while bpm_a > 220.0 {
            bpm_a /= 2.0;
        }
        while bpm_b < 125.0 {
            bpm_b *= 2.0;
        }
        while bpm_b > 220.0 {
            bpm_b /= 2.0;
        }

        // Extract exit chroma from Track A
        let exit_a = track_a
            .cue_zones
            .iter()
            .find(|z| z.zone_type == crate::schema::ZoneType::EXIT)
            .or_else(|| track_a.cue_zones.last());

        // Extract entry chroma from Track B
        let entry_b = track_b
            .cue_zones
            .iter()
            .find(|z| z.zone_type == crate::schema::ZoneType::ENTRY)
            .or_else(|| track_b.cue_zones.first());

        let (root_a, is_minor_a) = if let Some(zone) = exit_a {
            Self::detect_root_and_mode(zone, true)
        } else {
            (5, false) // Default F Major
        };

        let (root_b, is_minor_b) = if let Some(zone) = entry_b {
            Self::detect_root_and_mode(zone, false)
        } else {
            (0, false) // Default C Major
        };

        GenerativeFillConfig {
            duration_bars: bars.clamp(1, 2),
            tempo_a_bpm: bpm_a,
            tempo_b_bpm: bpm_b,
            root_a_pitch_class: root_a,
            root_b_pitch_class: root_b,
            is_minor_a,
            is_minor_b,
            style,
            sample_rate,
            master_gain: 0.85,
            include_noise_riser: true,
            include_snare_roll: true,
        }
    }

    /// Create fill configuration directly from audio slices (e.g. during headless rendering or real-time playback)
    pub fn create_config_from_slices(
        slice_a: &AudioBuffer,
        slice_b: &AudioBuffer,
        transition_duration_ms: u64,
        sample_rate: u32,
    ) -> GenerativeFillConfig {
        let (root_a, is_minor_a) = Self::detect_root_and_mode_from_audio(slice_a);
        let (root_b, is_minor_b) = Self::detect_root_and_mode_from_audio(slice_b);

        let dur_sec = (transition_duration_ms as f32 / 1000.0).max(0.5);
        let tempo_a = (60.0 / (dur_sec / 4.0)).clamp(120.0, 200.0);
        let tempo_b = tempo_a;

        GenerativeFillConfig {
            duration_bars: 1,
            tempo_a_bpm: tempo_a,
            tempo_b_bpm: tempo_b,
            root_a_pitch_class: root_a,
            root_b_pitch_class: root_b,
            is_minor_a,
            is_minor_b,
            style: MelodicFillStyle::EurobeatSuperSaw,
            sample_rate,
            master_gain: 0.85,
            include_noise_riser: true,
            include_snare_roll: true,
        }
    }

    /// Detect root pitch class and mode directly from raw audio samples via CQT chroma
    pub fn detect_root_and_mode_from_audio(audio: &AudioBuffer) -> (usize, bool) {
        let mono = audio.to_mono();
        if mono.is_empty() {
            return (0, false);
        }

        let sr = audio.sample_rate;
        let fft_len = 2048.min(mono.len()).next_power_of_two();
        if fft_len < 256 {
            return (0, false);
        }

        let mut planner = realfft::RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_len);

        let mut input = r2c.make_input_vec();
        let mut output = r2c.make_output_vec();

        let mut chroma = [0.0f32; 12];
        let num_windows = (mono.len() / fft_len).clamp(1, 10);
        let step = (mono.len().saturating_sub(fft_len)) / num_windows.max(1);

        for w in 0..num_windows {
            let offset = w * step;
            if offset + fft_len > mono.len() {
                break;
            }
            for i in 0..fft_len {
                let hann = 0.5 * (1.0 - (2.0 * PI * i as f32 / (fft_len as f32 - 1.0)).cos());
                input[i] = mono[offset + i] * hann;
            }

            if r2c.process(&mut input, &mut output).is_ok() {
                for (bin, comp) in output.iter().enumerate().skip(2) {
                    let freq = (bin as f32 / fft_len as f32) * sr as f32;
                    if (35.0..=2200.0).contains(&freq) {
                        let mag = comp.norm();
                        let midi = 69.0 + 12.0 * (freq / 440.0).log2();
                        let semitone = (midi.round() as i32).rem_euclid(12) as usize;
                        chroma[semitone] += mag;
                    }
                }
            }
        }

        let mut root = 0;
        let mut max_val = -1.0f32;
        for (i, &v) in chroma.iter().enumerate() {
            if v > max_val {
                max_val = v;
                root = i;
            }
        }

        let minor_third = chroma[(root + 3) % 12];
        let major_third = chroma[(root + 4) % 12];
        let is_minor = minor_third > major_third * 1.05;

        (root, is_minor)
    }

    /// Detect root pitch class and mode (major vs minor) from cue zone chroma
    fn detect_root_and_mode(zone: &CueZone, is_exit: bool) -> (usize, bool) {
        let chroma_frames = &zone.frames_20hz.chroma_12;
        if chroma_frames.is_empty() {
            return (0, false);
        }

        // Average chroma across the relevant boundary frames (last 10 for exit, first 10 for entry)
        let mut avg_chroma = [0.0f32; 12];
        let window = if is_exit {
            let start = chroma_frames.len().saturating_sub(15);
            &chroma_frames[start..]
        } else {
            let end = 15.min(chroma_frames.len());
            &chroma_frames[..end]
        };

        for f in window {
            for (i, &v) in f.iter().enumerate() {
                avg_chroma[i] += v;
            }
        }

        let mut root = 0;
        let mut max_val = -1.0f32;
        for (i, &v) in avg_chroma.iter().enumerate() {
            if v > max_val {
                max_val = v;
                root = i;
            }
        }

        let minor_third = avg_chroma[(root + 3) % 12];
        let major_third = avg_chroma[(root + 4) % 12];
        let is_minor = minor_third > major_third * 1.05;

        (root, is_minor)
    }

    /// Generate the note events for the bridge fill based on musical rules
    pub fn build_note_sequence(config: &GenerativeFillConfig) -> (Vec<NoteEvent>, usize) {
        let sr = config.sample_rate as f64;
        let total_beats = config.duration_bars * 4;
        let total_sixteenths = total_beats * 4;

        // Compute beat timing with smooth tempo acceleration from tempo_a to tempo_b
        let mut sixteenth_starts = Vec::with_capacity(total_sixteenths as usize);
        let mut current_sample = 0.0f64;

        for s in 0..total_sixteenths {
            sixteenth_starts.push(current_sample.round() as usize);
            let progress = s as f64 / total_sixteenths as f64;
            let current_bpm = config.tempo_a_bpm as f64 + progress * (config.tempo_b_bpm as f64 - config.tempo_a_bpm as f64);
            let beat_duration_sec = 60.0 / current_bpm;
            let sixteenth_duration_sec = beat_duration_sec / 4.0;
            current_sample += sixteenth_duration_sec * sr;
        }

        let total_fill_samples = current_sample.round() as usize;

        let root_a = config.root_a_pitch_class as i32;
        let root_b = config.root_b_pitch_class as i32;

        let third_a = if config.is_minor_a { 3 } else { 4 };
        let fifth_a = 7;

        let third_b = if config.is_minor_b { 3 } else { 4 };
        let fifth_b = 7;
        let leading_b = 11; // 1 semitone below root_b

        let mut notes = Vec::new();

        // Base octaves: MIDI 60 is C4
        let base_a = 60 + root_a;
        let base_b = 60 + root_b;

        for s in 0..total_sixteenths {
            let start = sixteenth_starts[s as usize];
            let next_start = if (s as usize + 1) < sixteenth_starts.len() {
                sixteenth_starts[s as usize + 1]
            } else {
                total_fill_samples
            };
            let step_len = next_start.saturating_sub(start);

            let note_pitch;
            let mut vel = 0.70;

            if config.duration_bars == 1 {
                // 1-Bar (16 sixteenths) Fill Grammar:
                match s {
                    // Beat 1: Track A Tonic statement (octave arpeggio)
                    0 => { note_pitch = base_a; vel = 0.85; },
                    1 => { note_pitch = base_a + fifth_a; },
                    2 => { note_pitch = base_a + third_a; },
                    3 => { note_pitch = base_a + 12; vel = 0.80; },

                    // Beat 2: Eurobeat octave syncopation
                    4 => { note_pitch = base_a + 12; vel = 0.90; },
                    5 => { note_pitch = base_a + fifth_a; },
                    6 => { note_pitch = base_a + third_a + 12; },
                    7 => { note_pitch = base_a + 12; },

                    // Beat 3: Pivot Modulation towards Track B key
                    8 => {
                        // Dominant fifth or passing approach tone
                        note_pitch = base_b + fifth_b;
                        vel = 0.88;
                    },
                    9 => {
                        note_pitch = base_b + third_b + 12;
                    },
                    10 => {
                        note_pitch = base_b + fifth_b + 12;
                    },
                    11 => {
                        // Tension note: leading tone
                        note_pitch = base_b + leading_b;
                        vel = 0.92;
                    },

                    // Beat 4: Climax Riser Arpeggio leading to Downbeat 1
                    12 => { note_pitch = base_b + leading_b; vel = 0.90; },
                    13 => { note_pitch = base_b + 12 + leading_b; vel = 0.92; },
                    14 => { note_pitch = base_b + 12 + leading_b; vel = 0.95; },
                    15 => { note_pitch = base_b + 12 + leading_b; vel = 1.00; },

                    _ => { note_pitch = base_a; }
                }
            } else {
                // 2-Bars (32 sixteenths) Fill Grammar:
                let bar = s / 16;
                let step = s % 16;
                if bar == 0 {
                    // Bar 1: Establish Track A energetic theme
                    match step {
                        0 | 4 | 8 | 12 => { note_pitch = base_a + 12; vel = 0.85; },
                        1 | 5 | 9 | 13 => { note_pitch = base_a + fifth_a; vel = 0.75; },
                        2 | 6 | 10 | 14 => { note_pitch = base_a + third_a; vel = 0.70; },
                        _ => { note_pitch = base_a; vel = 0.75; }
                    }
                } else {
                    // Bar 2: Ascending chromatic / fifth modulation into Track B
                    match step {
                        0..=3 => { note_pitch = base_a + fifth_a + 12; vel = 0.82; },
                        4..=7 => { note_pitch = base_b + fifth_b + 12; vel = 0.88; },
                        8..=11 => { note_pitch = base_b + third_b + 12; vel = 0.92; },
                        12..=15 => { note_pitch = base_b + leading_b + 12; vel = 0.98; },
                        _ => { note_pitch = base_b + 12; }
                    }
                }
            }

            notes.push(NoteEvent {
                start_sample: start,
                duration_samples: (step_len as f32 * 0.90) as usize,
                midi_pitch: note_pitch as f32,
                velocity: vel,
                is_resolution: false,
            });
        }

        // Add the triumphant resolution hit on Downbeat 1 of Track B (sample = total_fill_samples)
        let resolution_len = (sr * 1.2) as usize; // Long ring-out
        notes.push(NoteEvent {
            start_sample: total_fill_samples,
            duration_samples: resolution_len,
            midi_pitch: (base_b + 12) as f32,
            velocity: 1.0,
            is_resolution: true,
        });

        (notes, total_fill_samples)
    }

    /// Synthesize the complete stereo audio buffer for the generative fill
    pub fn render_fill(
        config: &GenerativeFillConfig,
        outgoing_slice: Option<&AudioBuffer>,
    ) -> AudioBuffer {
        let (notes, fill_duration_samples) = Self::build_note_sequence(config);
        let sr = config.sample_rate;
        // Output duration: fill duration + 1.2s resolution ring-out spillover
        let spillover_samples = (sr as f32 * 1.2) as usize;
        let total_output_samples = fill_duration_samples + spillover_samples;

        let mut left_buf = vec![0.0f32; total_output_samples];
        let mut right_buf = vec![0.0f32; total_output_samples];

        // 1. Synthesize the Melodic Lead / Arpeggiator Layer
        Self::synthesize_lead_arpeggio(
            &notes,
            config,
            fill_duration_samples,
            &mut left_buf,
            &mut right_buf,
        );

        // 2. Synthesize Riser & Snare Roll Build Layer (if enabled)
        if config.include_noise_riser || config.include_snare_roll {
            Self::synthesize_riser_and_snare(
                config,
                fill_duration_samples,
                &mut left_buf,
                &mut right_buf,
            );
        }

        // 3. Synthesize Hybrid Motif Grain Stutter Layer (if enabled and slice provided)
        if config.style == MelodicFillStyle::HybridMotifAndArp {
            if let Some(slice) = outgoing_slice {
                Self::blend_hybrid_motif_stutter(
                    slice,
                    config,
                    fill_duration_samples,
                    &mut left_buf,
                    &mut right_buf,
                );
            }
        }

        // 4. Apply Spatial Ping-Pong Delay & Reverb Diffusion
        Self::apply_delay_and_reverb(&mut left_buf, &mut right_buf, sr, fill_duration_samples);

        // Normalize / master gain
        let master = config.master_gain;
        for i in 0..total_output_samples {
            left_buf[i] = (left_buf[i] * master).clamp(-1.0, 1.0);
            right_buf[i] = (right_buf[i] * master).clamp(-1.0, 1.0);
        }

        AudioBuffer {
            channels: vec![left_buf, right_buf],
            sample_rate: sr,
        }
    }

    /// Synthesize dual-detuned PolyBLEP saw / FM lead with resonant filter sweep
    fn synthesize_lead_arpeggio(
        notes: &[NoteEvent],
        config: &GenerativeFillConfig,
        fill_duration_samples: usize,
        out_l: &mut [f32],
        out_r: &mut [f32],
    ) {
        let sr = config.sample_rate as f32;

        for note in notes {
            let freq = 440.0 * 2.0f32.powf((note.midi_pitch - 69.0) / 12.0);
            let duration = note.duration_samples.min(out_l.len().saturating_sub(note.start_sample));

            // Detuning for stereo width: Left -7 cents, Right +7 cents
            let freq_l = freq * 2.0f32.powf(-7.0 / 1200.0);
            let freq_r = freq * 2.0f32.powf(7.0 / 1200.0);

            let mut phase_l = 0.0f32;
            let mut phase_r = 0.0f32;

            // Unconditionally stable TPT state-variable filter per voice
            let mut filter_l = TptSvf::new();
            let mut filter_r = TptSvf::new();

            let attack_samples = (sr * 0.003).max(1.0);
            let decay_samples = if note.is_resolution { sr * 0.45 } else { sr * 0.07 };
            let sustain = if note.is_resolution { 0.55 } else { 0.35 };
            let release_samples = if note.is_resolution { sr * 0.55 } else { sr * 0.035 };

            for i in 0..duration {
                let sample_idx = note.start_sample + i;
                if sample_idx >= out_l.len() {
                    break;
                }

                let t_note = i as f32;
                // Envelope ADSR
                let env = if t_note < attack_samples {
                    t_note / attack_samples
                } else if t_note < attack_samples + decay_samples {
                    1.0 - (1.0 - sustain) * ((t_note - attack_samples) / decay_samples)
                } else if i < duration.saturating_sub(release_samples as usize) {
                    sustain
                } else {
                    let rel_progress = (i - duration.saturating_sub(release_samples as usize)) as f32 / release_samples;
                    sustain * (1.0 - rel_progress).max(0.0)
                };

                // Filter cutoff frequency: sweeps from 800 Hz up to 16,000 Hz across fill duration!
                let fill_progress = (sample_idx as f32 / fill_duration_samples as f32).clamp(0.0, 1.0);
                let base_cutoff = 800.0 * (16000.0 / 800.0f32).powf(fill_progress);
                // Note pluck envelope added to cutoff
                let note_env_pluck = (-t_note / (sr * 0.04)).exp() * 3000.0;
                let cutoff = (base_cutoff + note_env_pluck).clamp(200.0, sr * 0.45);

                // PolyBLEP anti-aliased Sawtooth oscillators
                let saw_l = polyblep_saw(phase_l, freq_l / sr);
                let saw_r = polyblep_saw(phase_r, freq_r / sr);

                // FM transient bite: 2:1 ratio
                let fm_mod = (2.0 * PI * 2.0 * freq * (i as f32 / sr)).sin() * (-t_note / (sr * 0.035)).exp() * 0.35;

                let osc_l = (saw_l + fm_mod) * env * note.velocity;
                let osc_r = (saw_r + fm_mod) * env * note.velocity;

                // Step TPT SVF (low-pass)
                let (lp_l, _, _) = filter_l.process(osc_l, cutoff, 2.2, sr);
                let (lp_r, _, _) = filter_r.process(osc_r, cutoff, 2.2, sr);

                out_l[sample_idx] += lp_l * 0.65;
                out_r[sample_idx] += lp_r * 0.65;

                // Advance phases
                phase_l = (phase_l + freq_l / sr).fract();
                phase_r = (phase_r + freq_r / sr).fract();
            }
        }
    }

    /// Synthesize building noise riser and rhythmic 16th/32nd snare roll
    fn synthesize_riser_and_snare(
        config: &GenerativeFillConfig,
        fill_duration_samples: usize,
        out_l: &mut [f32],
        out_r: &mut [f32],
    ) {
        let sr = config.sample_rate as f32;
        let mut rng_state: u32 = 0x12345678;

        // 1. Noise Riser
        if config.include_noise_riser {
            let mut noise_filter = TptSvf::new();

            for i in 0..fill_duration_samples {
                if i >= out_l.len() {
                    break;
                }
                let t_progress = i as f32 / fill_duration_samples as f32;
                // Exponential volume rise
                let riser_vol = t_progress.powi(2) * 0.22;

                // Sweeping bandpass cutoff 1200 Hz -> 9000 Hz
                let cutoff = 1200.0 * (9000.0 / 1200.0f32).powf(t_progress);

                // Fast LCG noise
                rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
                let white_noise = ((rng_state >> 16) as f32 / 32768.0) - 1.0;

                let (_, bp, _) = noise_filter.process(white_noise, cutoff, 3.0, sr);

                let noise_sample = bp * riser_vol;
                out_l[i] += noise_sample;
                out_r[i] += noise_sample;
            }
        }

        // 2. Snare Roll Crescendo in the second half of the fill
        if config.include_snare_roll {
            let start_sample = fill_duration_samples / 2;
            let roll_duration = fill_duration_samples - start_sample;
            let total_snare_hits = 8; // 8 16th-note hits in the final 2 beats
            let hit_interval = roll_duration / total_snare_hits;

            for h in 0..total_snare_hits {
                let hit_start = start_sample + h * hit_interval;
                let vel = 0.30 + 0.70 * (h as f32 / total_snare_hits as f32);
                let hit_len = (sr * 0.075) as usize; // 75ms snare

                for i in 0..hit_len {
                    let idx = hit_start + i;
                    if idx >= out_l.len() {
                        break;
                    }
                    let t = i as f32 / sr;
                    // Snare body: 180 Hz dropping to 85 Hz
                    let pitch_env = 85.0 + 95.0 * (-t / 0.02).exp();
                    let body = (2.0 * PI * pitch_env * t).sin() * (-t / 0.035).exp() * 0.40;

                    // Snare noise crackle
                    rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
                    let noise = (((rng_state >> 16) as f32 / 32768.0) - 1.0) * (-t / 0.05).exp() * 0.30;

                    let snare = (body + noise) * vel * 0.50;
                    out_l[idx] += snare;
                    out_r[idx] += snare;
                }
            }
        }
    }

    /// Hybrid Mode: Extract the outgoing motif, high-pass, and rhythmic stutter loop it
    fn blend_hybrid_motif_stutter(
        outgoing: &AudioBuffer,
        config: &GenerativeFillConfig,
        fill_duration_samples: usize,
        out_l: &mut [f32],
        out_r: &mut [f32],
    ) {
        if outgoing.channels.is_empty() {
            return;
        }
        let in_len = outgoing.channels[0].len();
        if in_len < 44100 {
            return;
        }

        let motif_len = (config.sample_rate as f32 * 0.375) as usize; // 1 beat
        let motif_start = in_len.saturating_sub(motif_len);

        let slice_l = &outgoing.channels[0][motif_start..];
        let slice_r = outgoing.channels.get(1).map(|c| &c[motif_start..]).unwrap_or(slice_l);

        // Stutter accelerate: 1/2 beat -> 1/4 beat -> 1/8 beat
        let mut loop_len = motif_len;

        for (loop_pos, i) in (0..fill_duration_samples).enumerate() {
            if i >= out_l.len() {
                break;
            }
            if i > (fill_duration_samples * 3 / 4) {
                loop_len = motif_len / 4;
            } else if i > (fill_duration_samples / 2) {
                loop_len = motif_len / 2;
            }

            let src_idx = loop_pos % loop_len.max(1);
            let s_l = slice_l.get(src_idx).copied().unwrap_or(0.0);
            let s_r = slice_r.get(src_idx).copied().unwrap_or(s_l);

            // Duck low-end and blend at -8 dB
            let duck = (1.0 - (i as f32 / fill_duration_samples as f32)) * 0.38;
            out_l[i] += s_l * duck;
            out_r[i] += s_r * duck;
        }
    }

    /// Stereo ping-pong delay and diffuse spatial wash
    fn apply_delay_and_reverb(
        out_l: &mut [f32],
        out_r: &mut [f32],
        sr: u32,
        fill_duration_samples: usize,
    ) {
        let delay_samples = (sr as f32 * 0.1875) as usize; // ~187ms dotted 16th at 160 BPM
        let mut delay_l = vec![0.0f32; delay_samples + 1];
        let mut delay_r = vec![0.0f32; delay_samples + 1];
        let mut d_idx = 0;

        let len = out_l.len();
        for i in 0..len {
            let in_l = out_l[i];
            let in_r = out_r[i];

            let delayed_l = delay_l[d_idx];
            let delayed_r = delay_r[d_idx];

            // Cross ping-pong feedback (25%) with 6kHz low-pass filter
            delay_l[d_idx] = in_l + delayed_r * 0.25;
            delay_r[d_idx] = in_r + delayed_l * 0.25;

            d_idx = (d_idx + 1) % delay_samples;

            out_l[i] += delayed_l * 0.30;
            out_r[i] += delayed_r * 0.30;

            // At resolution point, diffuse into stereo reverb wash
            if i >= fill_duration_samples {
                let diff_progress = (i - fill_duration_samples) as f32 / (len - fill_duration_samples) as f32;
                let decay = (-3.0 * diff_progress).exp();
                let diffuse = (delayed_l - delayed_r) * 0.20 * decay;
                out_l[i] += diffuse;
                out_r[i] -= diffuse;
            }
        }
    }
}

/// Band-limited PolyBLEP Anti-Aliased Sawtooth
#[inline]
fn polyblep_saw(phase: f32, dt: f32) -> f32 {
    let mut saw = 2.0 * phase - 1.0;
    if phase < dt {
        let t = phase / dt;
        saw -= 2.0 * t - t * t - 1.0;
    } else if phase > 1.0 - dt {
        let t = (phase - 1.0) / dt;
        saw -= t * t + 2.0 * t + 1.0;
    }
    saw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generative_fill_config_and_note_sequence() {
        let config = GenerativeFillConfig {
            duration_bars: 1,
            tempo_a_bpm: 160.0,
            tempo_b_bpm: 171.4,
            root_a_pitch_class: 5, // F
            root_b_pitch_class: 0, // C
            is_minor_a: false,
            is_minor_b: false,
            style: MelodicFillStyle::EurobeatSuperSaw,
            sample_rate: 44100,
            master_gain: 0.85,
            include_noise_riser: true,
            include_snare_roll: true,
        };

        let (notes, fill_duration) = GenerativeFillEngine::build_note_sequence(&config);
        // 1 bar = 16 sixteenths + 1 resolution hit = 17 note events
        assert_eq!(notes.len(), 17);
        assert!(fill_duration > 0);

        // Check that the last note event is the resolution hit on Downbeat 1
        let last_note = notes.last().unwrap();
        assert!(last_note.is_resolution);
        assert_eq!(last_note.start_sample, fill_duration);
        // Base C4 = 60, resolution = C5 = 72
        assert_eq!(last_note.midi_pitch, 72.0);
    }

    #[test]
    fn test_render_fill_audio_buffer() {
        let config = GenerativeFillConfig::default();
        let buf = GenerativeFillEngine::render_fill(&config, None);

        assert_eq!(buf.channels.len(), 2);
        assert!(buf.duration_ms() > 1000);

        // Verify audio contains non-zero signal and doesn't clip
        let mut max_abs = 0.0f32;
        for &s in &buf.channels[0] {
            max_abs = max_abs.max(s.abs());
        }
        assert!(max_abs > 0.1, "Rendered fill should have audible audio");
        assert!(max_abs <= 1.0, "Rendered fill should not clip above 1.0");
    }
}
