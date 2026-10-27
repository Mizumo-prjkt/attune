//! Attune Studio Master FX DSP Rack
//!
//! Sequential high-performance f32 mastering chain executed directly inside
//! Attune's native process right after the dual-deck crossfader and before
//! OS audio hardware output (CPAL/Oboe):
//!
//! [Decoded PCM f32]
//!   ➔ 1. Pre-Amp Headroom Pad (-12 dB to +6 dB, preventing filter clipping)
//!   ➔ 2. 10-Band Graphic / Parametric EQ (Cascaded IIR DirectForm2Transposed Biquads)
//!   ➔ 3. Attune Pure Bass+ (Psychoacoustic Sub-Harmonic Synthesizer)
//!   ➔ 4. Attune Clarity & Vocal Air (High-Shelf Harmonic Exciter)
//!   ➔ 5. Stereo Field Expander / Differential Surround (Mid-Side matrix with Mono-Bass protection)
//!   ➔ 6. Master Limiter & Soft-Knee Saturator (Brickwall -0.1 dBFS ceiling)
//!   ➔ [OS Audio Output Sink]

use std::f32::consts::PI;

pub const EQ_BANDS_COUNT: usize = 10;
pub const EQ_CENTER_FREQS: [f32; EQ_BANDS_COUNT] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadCoeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoeffs {
    /// Identity bypass filter
    pub fn bypass() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Peaking EQ filter with center frequency f0, Q, and gain in dB
    pub fn peaking(sample_rate: f32, f0: f32, q: f32, gain_db: f32) -> Self {
        if gain_db.abs() < 0.001 {
            return Self::bypass();
        }
        let w0 = 2.0 * PI * (f0 / sample_rate).clamp(0.0001, 0.499);
        let alpha = w0.sin() / (2.0 * q.max(0.1));
        let a = 10.0f32.powf(gain_db / 40.0);
        let cos_w0 = w0.cos();

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Low-shelf filter for sub-bass
    pub fn low_shelf(sample_rate: f32, f0: f32, gain_db: f32) -> Self {
        if gain_db.abs() < 0.001 {
            return Self::bypass();
        }
        let w0 = 2.0 * PI * (f0 / sample_rate).clamp(0.0001, 0.499);
        let a = 10.0f32.powf(gain_db / 40.0);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * (2.0f32).sqrt();

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// High-shelf filter for treble air
    pub fn high_shelf(sample_rate: f32, f0: f32, gain_db: f32) -> Self {
        if gain_db.abs() < 0.001 {
            return Self::bypass();
        }
        let w0 = 2.0 * PI * (f0 / sample_rate).clamp(0.0001, 0.499);
        let a = 10.0f32.powf(gain_db / 40.0);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * (2.0f32).sqrt();

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// 2nd order Butterworth lowpass filter
    pub fn lowpass(sample_rate: f32, cutoff_hz: f32) -> Self {
        let w0 = 2.0 * PI * (cutoff_hz / sample_rate).clamp(0.0001, 0.499);
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * (0.7071));

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// 2nd order Butterworth highpass filter
    pub fn highpass(sample_rate: f32, cutoff_hz: f32) -> Self {
        let w0 = 2.0 * PI * (cutoff_hz / sample_rate).clamp(0.0001, 0.499);
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * (0.7071));

        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadState {
    pub s1: f32,
    pub s2: f32,
}

impl BiquadState {
    #[inline(always)]
    pub fn process(&mut self, x: f32, c: &BiquadCoeffs) -> f32 {
        let y = c.b0 * x + self.s1;
        self.s1 = c.b1 * x - c.a1 * y + self.s2;
        self.s2 = c.b2 * x - c.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}

/// Parameters for Master FX Rack
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MasterFxParams {
    pub enabled: bool,
    pub preamp_db: f32,
    pub band_gains_db: [f32; EQ_BANDS_COUNT],
    pub bass_enabled: bool,
    pub bass_gain_db: f32,
    pub bass_freq_hz: f32,
    pub clarity_enabled: bool,
    pub clarity_wet: f32,
    pub stereo_width: f32,
    pub limiter_ceiling_db: f32,
}

impl Default for MasterFxParams {
    fn default() -> Self {
        Self {
            enabled: true,
            preamp_db: -3.0, // Default 3dB headroom pad preventing filter clipping
            band_gains_db: [0.0; EQ_BANDS_COUNT],
            bass_enabled: false,
            bass_gain_db: 4.0,
            bass_freq_hz: 80.0,
            clarity_enabled: false,
            clarity_wet: 0.12,
            stereo_width: 1.0,
            limiter_ceiling_db: -0.1,
        }
    }
}

/// Attune Studio Master FX Rack executing the full 6-stage mastering chain
#[derive(Debug, Clone)]
pub struct MasterFxRack {
    sample_rate: f32,
    target_params: MasterFxParams,

    // 1. Preamp smoothing
    current_preamp_linear: f32,
    target_preamp_linear: f32,

    // 2. 10-Band EQ filters (Left, Right)
    eq_coeffs: [BiquadCoeffs; EQ_BANDS_COUNT],
    eq_state_l: [BiquadState; EQ_BANDS_COUNT],
    eq_state_r: [BiquadState; EQ_BANDS_COUNT],

    // 3. Attune Pure Bass+ (Psychoacoustic Sub-Harmonic Synthesizer)
    bass_isolate_coeffs: BiquadCoeffs,
    bass_smooth_coeffs: BiquadCoeffs,
    bass_isolate_l: BiquadState,
    bass_isolate_r: BiquadState,
    bass_smooth_l: BiquadState,
    bass_smooth_r: BiquadState,
    current_bass_linear: f32,
    target_bass_linear: f32,

    // 4. Attune Clarity & Vocal Air (High-Shelf Harmonic Exciter)
    clarity_hp_coeffs: BiquadCoeffs,
    clarity_hp_l: BiquadState,
    clarity_hp_r: BiquadState,
    current_clarity_wet: f32,
    target_clarity_wet: f32,

    // 5. Stereo Field Expander (Mid-Side matrix with mono-bass highpass)
    side_mono_hp_coeffs: BiquadCoeffs,
    side_hp_state: BiquadState,
    current_stereo_width: f32,
    target_stereo_width: f32,

    // 6. Master Limiter & Peak Envelope
    envelope: f32,
    attack_coef: f32,
    release_coef: f32,
    limiter_ceiling_linear: f32,
}

impl MasterFxRack {
    pub fn new(sample_rate: f32) -> Self {
        let sr = if sample_rate > 0.0 { sample_rate } else { 44100.0 };
        let mut rack = Self {
            sample_rate: sr,
            target_params: MasterFxParams::default(),
            current_preamp_linear: 1.0,
            target_preamp_linear: 1.0,
            eq_coeffs: [BiquadCoeffs::bypass(); EQ_BANDS_COUNT],
            eq_state_l: [BiquadState::default(); EQ_BANDS_COUNT],
            eq_state_r: [BiquadState::default(); EQ_BANDS_COUNT],
            bass_isolate_coeffs: BiquadCoeffs::bypass(),
            bass_smooth_coeffs: BiquadCoeffs::bypass(),
            bass_isolate_l: BiquadState::default(),
            bass_isolate_r: BiquadState::default(),
            bass_smooth_l: BiquadState::default(),
            bass_smooth_r: BiquadState::default(),
            current_bass_linear: 0.0,
            target_bass_linear: 0.0,
            clarity_hp_coeffs: BiquadCoeffs::bypass(),
            clarity_hp_l: BiquadState::default(),
            clarity_hp_r: BiquadState::default(),
            current_clarity_wet: 0.0,
            target_clarity_wet: 0.0,
            side_mono_hp_coeffs: BiquadCoeffs::bypass(),
            side_hp_state: BiquadState::default(),
            current_stereo_width: 1.0,
            target_stereo_width: 1.0,
            envelope: 0.0,
            attack_coef: (-1.0 / (0.001 * sr)).exp(),   // 1ms attack
            release_coef: (-1.0 / (0.050 * sr)).exp(),  // 50ms release
            limiter_ceiling_linear: 10.0f32.powf(-0.1 / 20.0), // -0.1 dBFS ceiling
        };
        rack.recompute_all_coeffs();
        rack
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if sample_rate > 0.0 && (sample_rate - self.sample_rate).abs() > 1.0 {
            self.sample_rate = sample_rate;
            self.attack_coef = (-1.0 / (0.001 * sample_rate)).exp();
            self.release_coef = (-1.0 / (0.050 * sample_rate)).exp();
            self.recompute_all_coeffs();
            self.reset_states();
        }
    }

    pub fn reset_states(&mut self) {
        for s in &mut self.eq_state_l { s.reset(); }
        for s in &mut self.eq_state_r { s.reset(); }
        self.bass_isolate_l.reset();
        self.bass_isolate_r.reset();
        self.bass_smooth_l.reset();
        self.bass_smooth_r.reset();
        self.clarity_hp_l.reset();
        self.clarity_hp_r.reset();
        self.side_hp_state.reset();
        self.envelope = 0.0;
    }

    pub fn params(&self) -> &MasterFxParams {
        &self.target_params
    }

    pub fn set_params(&mut self, params: MasterFxParams) {
        self.target_params = params.clone();
        self.target_preamp_linear = 10.0f32.powf(params.preamp_db / 20.0);
        self.target_bass_linear = if params.bass_enabled {
            10.0f32.powf(params.bass_gain_db / 20.0)
        } else {
            0.0
        };
        self.target_clarity_wet = if params.clarity_enabled {
            params.clarity_wet.clamp(0.0, 0.40)
        } else {
            0.0
        };
        self.target_stereo_width = params.stereo_width.clamp(0.0, 2.5);
        self.limiter_ceiling_linear = 10.0f32.powf(params.limiter_ceiling_db.min(0.0) / 20.0);

        self.recompute_all_coeffs();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.target_params.enabled = enabled;
    }

    pub fn set_preamp_db(&mut self, db: f32) {
        self.target_params.preamp_db = db;
        self.target_preamp_linear = 10.0f32.powf(db / 20.0);
    }

    pub fn set_band_gain(&mut self, band_idx: usize, db: f32) {
        if band_idx < EQ_BANDS_COUNT {
            self.target_params.band_gains_db[band_idx] = db.clamp(-12.0, 12.0);
            self.recompute_band(band_idx);
        }
    }

    pub fn set_all_bands(&mut self, gains_db: &[f32]) {
        for (i, &db) in gains_db.iter().take(EQ_BANDS_COUNT).enumerate() {
            self.target_params.band_gains_db[i] = db.clamp(-12.0, 12.0);
        }
        self.recompute_all_coeffs();
    }

    pub fn set_bass(&mut self, enabled: bool, gain_db: f32, freq_hz: f32) {
        self.target_params.bass_enabled = enabled;
        self.target_params.bass_gain_db = gain_db.clamp(0.0, 14.0);
        self.target_params.bass_freq_hz = freq_hz.clamp(40.0, 150.0);
        self.target_bass_linear = if enabled {
            10.0f32.powf(gain_db / 20.0)
        } else {
            0.0
        };
        self.recompute_bass_coeffs();
    }

    pub fn set_clarity(&mut self, enabled: bool, wet: f32) {
        self.target_params.clarity_enabled = enabled;
        self.target_params.clarity_wet = wet.clamp(0.0, 0.40);
        self.target_clarity_wet = if enabled { wet.clamp(0.0, 0.40) } else { 0.0 };
    }

    pub fn set_stereo_width(&mut self, width: f32) {
        self.target_params.stereo_width = width.clamp(0.0, 2.5);
        self.target_stereo_width = width.clamp(0.0, 2.5);
    }

    fn recompute_all_coeffs(&mut self) {
        for i in 0..EQ_BANDS_COUNT {
            self.recompute_band(i);
        }
        self.recompute_bass_coeffs();
        self.clarity_hp_coeffs = BiquadCoeffs::highpass(self.sample_rate, 3500.0);
        self.side_mono_hp_coeffs = BiquadCoeffs::highpass(self.sample_rate, 150.0);
    }

    fn recompute_band(&mut self, band_idx: usize) {
        let f0 = EQ_CENTER_FREQS[band_idx];
        let db = self.target_params.band_gains_db[band_idx];

        self.eq_coeffs[band_idx] = if band_idx == 0 {
            BiquadCoeffs::low_shelf(self.sample_rate, f0, db)
        } else if band_idx == EQ_BANDS_COUNT - 1 {
            BiquadCoeffs::high_shelf(self.sample_rate, f0, db)
        } else {
            BiquadCoeffs::peaking(self.sample_rate, f0, 1.414, db)
        };
    }

    fn recompute_bass_coeffs(&mut self) {
        self.bass_isolate_coeffs = BiquadCoeffs::lowpass(self.sample_rate, self.target_params.bass_freq_hz);
        self.bass_smooth_coeffs = BiquadCoeffs::lowpass(self.sample_rate, 180.0);
    }

    /// Process a single stereo frame (L, R) through the full 6-stage chain
    #[inline]
    pub fn process_stereo_frame(&mut self, frame: (f32, f32)) -> (f32, f32) {
        if !self.target_params.enabled {
            return frame;
        }

        // 1-Pole exponential smoothing per sample frame (smooth ramps, zero zipper noise)
        const SMOOTH_ALPHA: f32 = 0.005;
        self.current_preamp_linear += SMOOTH_ALPHA * (self.target_preamp_linear - self.current_preamp_linear);
        self.current_bass_linear += SMOOTH_ALPHA * (self.target_bass_linear - self.current_bass_linear);
        self.current_clarity_wet += SMOOTH_ALPHA * (self.target_clarity_wet - self.current_clarity_wet);
        self.current_stereo_width += SMOOTH_ALPHA * (self.target_stereo_width - self.current_stereo_width);

        // Stage 1: Pre-Amp Gain / Headroom Pad
        let mut l = frame.0 * self.current_preamp_linear;
        let mut r = frame.1 * self.current_preamp_linear;

        // Stage 2: 10-Band Graphic / Parametric EQ (Cascaded Biquads)
        for i in 0..EQ_BANDS_COUNT {
            let c = &self.eq_coeffs[i];
            l = self.eq_state_l[i].process(l, c);
            r = self.eq_state_r[i].process(r, c);
        }

        // Stage 3: Attune Pure Bass+ (Psychoacoustic Sub-Harmonic Synthesizer)
        if self.current_bass_linear > 0.001 {
            // Isolate low frequencies
            let sub_l = self.bass_isolate_l.process(l, &self.bass_isolate_coeffs);
            let sub_r = self.bass_isolate_r.process(r, &self.bass_isolate_coeffs);

            // Asymmetric waveshaping to generate 2nd/3rd harmonics for earbuds/headphones
            let harm_l = (sub_l * 2.2).tanh() + 0.15 * sub_l * sub_l.abs();
            let harm_r = (sub_r * 2.2).tanh() + 0.15 * sub_r * sub_r.abs();

            // Low-pass at 180 Hz to prevent vocal bleed
            let clean_bass_l = self.bass_smooth_l.process(harm_l, &self.bass_smooth_coeffs);
            let clean_bass_r = self.bass_smooth_r.process(harm_r, &self.bass_smooth_coeffs);

            l += clean_bass_l * self.current_bass_linear;
            r += clean_bass_r * self.current_bass_linear;
        }

        // Stage 4: Attune Clarity & Vocal Air (High-Shelf Harmonic Exciter)
        if self.current_clarity_wet > 0.001 {
            let high_l = self.clarity_hp_l.process(l, &self.clarity_hp_coeffs);
            let high_r = self.clarity_hp_r.process(r, &self.clarity_hp_coeffs);

            // Gentle polynomial saturation (warmth + air)
            let sat_l = high_l.clamp(-1.5, 1.5);
            let exc_l = sat_l - (sat_l * sat_l * sat_l) / 3.0 + 0.08 * (sat_l * sat_l);
            let sat_r = high_r.clamp(-1.5, 1.5);
            let exc_r = sat_r - (sat_r * sat_r * sat_r) / 3.0 + 0.08 * (sat_r * sat_r);

            l += (exc_l - high_l) * self.current_clarity_wet;
            r += (exc_r - high_r) * self.current_clarity_wet;
        }

        // Stage 5: Stereo Field Expander / Differential Surround (Mid-Side Matrix)
        if (self.current_stereo_width - 1.0).abs() > 0.01 {
            let mid = 0.5 * (l + r);
            let side = 0.5 * (l - r);

            // Filter side channel above 150 Hz to keep sub-bass perfectly mono (prevents phase cancellation)
            let side_high = self.side_hp_state.process(side, &self.side_mono_hp_coeffs);
            let side_low = side - side_high;

            let side_expanded = side_low + side_high * self.current_stereo_width;

            l = mid + side_expanded;
            r = mid - side_expanded;
        }

        // Stage 6: Master Limiter & Soft Clipper (Brickwall -0.1 dBFS ceiling)
        let peak = l.abs().max(r.abs());
        if peak > self.envelope {
            self.envelope = peak + self.attack_coef * (self.envelope - peak);
        } else {
            self.envelope = peak + self.release_coef * (self.envelope - peak);
        }

        let gain = if self.envelope > self.limiter_ceiling_linear {
            self.limiter_ceiling_linear / self.envelope
        } else {
            1.0
        };

        // Smooth tanh soft-knee saturation
        let out_l = (l * gain).tanh().clamp(-0.999, 0.999);
        let out_r = (r * gain).tanh().clamp(-0.999, 0.999);

        (out_l, out_r)
    }

    /// Process interleaved stereo audio buffer `[L, R, L, R, ...]` in-place
    pub fn process_interleaved_block(&mut self, data: &mut [f32]) {
        if !self.target_params.enabled {
            return;
        }
        for frame in data.chunks_exact_mut(2) {
            let (l, r) = self.process_stereo_frame((frame[0], frame[1]));
            frame[0] = l;
            frame[1] = r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_fx_bypass() {
        let mut rack = MasterFxRack::new(44100.0);
        rack.set_enabled(false);

        let input = (0.42, -0.37);
        let output = rack.process_stereo_frame(input);
        assert!((output.0 - input.0).abs() < 1e-6);
        assert!((output.1 - input.1).abs() < 1e-6);
    }

    #[test]
    fn test_preamp_attenuation() {
        let mut rack = MasterFxRack::new(44100.0);
        rack.set_preamp_db(-6.0); // approx 0.501 linear

        // Process several samples to let 1-pole smoothing converge
        let mut last_frame = (0.5, 0.5);
        for _ in 0..1000 {
            last_frame = rack.process_stereo_frame((0.5, 0.5));
        }

        // Expected ~0.2505 with flat EQ and limiter ceiling untouched
        assert!((last_frame.0 - 0.25).abs() < 0.05);
        assert!((last_frame.1 - 0.25).abs() < 0.05);
    }

    #[test]
    fn test_10_band_peaking_boost() {
        let mut rack = MasterFxRack::new(44100.0);
        rack.set_preamp_db(0.0);
        rack.set_band_gain(5, 6.0); // Boost 1000 Hz band by +6 dB

        // Generate a 1000 Hz sine wave and measure RMS
        let freq = 1000.0;
        let mut rms_boosted = 0.0f32;
        let count = 4410; // 100ms
        for i in 0..count {
            let t = i as f32 / 44100.0;
            let sample = (2.0 * PI * freq * t).sin() * 0.1;
            let (l, _) = rack.process_stereo_frame((sample, sample));
            if i > 2000 { // steady-state
                rms_boosted += l * l;
            }
        }
        rms_boosted = (rms_boosted / 2410.0).sqrt();

        // Compare with flat EQ
        let mut flat_rack = MasterFxRack::new(44100.0);
        flat_rack.set_preamp_db(0.0);
        let mut rms_flat = 0.0f32;
        for i in 0..count {
            let t = i as f32 / 44100.0;
            let sample = (2.0 * PI * freq * t).sin() * 0.1;
            let (l, _) = flat_rack.process_stereo_frame((sample, sample));
            if i > 2000 {
                rms_flat += l * l;
            }
        }
        rms_flat = (rms_flat / 2410.0).sqrt();

        assert!(rms_boosted > rms_flat * 1.5, "1000 Hz boost should increase energy significantly");
    }

    #[test]
    fn test_attune_bass_synthesis() {
        let mut rack = MasterFxRack::new(44100.0);
        rack.set_preamp_db(0.0);
        rack.set_bass(true, 6.0, 80.0);

        // 60 Hz sine wave
        let mut bass_energy = 0.0f32;
        for i in 0..2000 {
            let t = i as f32 / 44100.0;
            let sample = (2.0 * PI * 60.0 * t).sin() * 0.1;
            let (l, _) = rack.process_stereo_frame((sample, sample));
            if i > 1000 {
                bass_energy += l * l;
            }
        }

        let mut no_bass_rack = MasterFxRack::new(44100.0);
        no_bass_rack.set_preamp_db(0.0);
        no_bass_rack.set_bass(false, 0.0, 80.0);
        let mut no_bass_energy = 0.0f32;
        for i in 0..2000 {
            let t = i as f32 / 44100.0;
            let sample = (2.0 * PI * 60.0 * t).sin() * 0.1;
            let (l, _) = no_bass_rack.process_stereo_frame((sample, sample));
            if i > 1000 {
                no_bass_energy += l * l;
            }
        }

        assert!(bass_energy > no_bass_energy, "Attune Pure Bass+ must synthesize sub-harmonics");
    }

    #[test]
    fn test_stereo_width_expansion() {
        let mut rack = MasterFxRack::new(44100.0);
        rack.set_preamp_db(0.0);
        rack.set_stereo_width(1.8);

        // Test with differential signal (pure Side energy at 1000 Hz)
        let mut diff_expanded = 0.0f32;
        for i in 0..1000 {
            let t = i as f32 / 44100.0;
            let sample = (2.0 * PI * 1000.0 * t).sin() * 0.2;
            let (l, r) = rack.process_stereo_frame((sample, -sample));
            if i > 500 {
                diff_expanded += (l - r).abs();
            }
        }

        let mut rack_normal = MasterFxRack::new(44100.0);
        rack_normal.set_preamp_db(0.0);
        rack_normal.set_stereo_width(1.0);
        let mut diff_normal = 0.0f32;
        for i in 0..1000 {
            let t = i as f32 / 44100.0;
            let sample = (2.0 * PI * 1000.0 * t).sin() * 0.2;
            let (l, r) = rack_normal.process_stereo_frame((sample, -sample));
            if i > 500 {
                diff_normal += (l - r).abs();
            }
        }

        assert!(diff_expanded > diff_normal, "Expanded stereo width should boost high side signal");
    }

    #[test]
    fn test_master_limiter_brickwall() {
        let mut rack = MasterFxRack::new(44100.0);
        rack.set_preamp_db(6.0); // Boost input heavily

        // Feed extremely loud frames (peak 5.0)
        let mut max_output = 0.0f32;
        for _ in 0..5000 {
            let (l, r) = rack.process_stereo_frame((4.5, -4.5));
            max_output = max_output.max(l.abs()).max(r.abs());
        }

        // Must strictly not exceed 1.0 (-0.1 dBFS ceiling with tanh saturation)
        assert!(max_output <= 1.0, "Limiter ceiling must prevent hard clipping above 1.0: got {}", max_output);
    }

    #[test]
    fn test_params_json_roundtrip() {
        let mut params = MasterFxParams::default();
        params.preamp_db = -4.5;
        params.band_gains_db[0] = 3.0;
        params.bass_enabled = true;
        params.stereo_width = 1.35;

        let json = serde_json::to_string(&params).expect("Failed to serialize MasterFxParams");
        let decoded: MasterFxParams = serde_json::from_str(&json).expect("Failed to deserialize MasterFxParams");

        assert_eq!(params, decoded);
    }
}

