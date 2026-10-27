//! 3-Band DJ Isolator EQ (-60 dB / mute to +6 dB)
//!
//! Uses cascaded Direct Form II Transposed biquad filters:
//! - Low Shelf: Sub/Bass cutoff at 200 Hz
//! - Peaking: Mid/Vocal center at 1000 Hz (Q = 0.707)
//! - High Shelf: Air/Treble cutoff at 4000 Hz

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, Default)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    s1: f32,
    s2: f32,
}

impl BiquadState {
    #[inline(always)]
    fn process(&mut self, x: f32, c: &BiquadCoeffs) -> f32 {
        let y = c.b0 * x + self.s1;
        self.s1 = c.b1 * x - c.a1 * y + self.s2;
        self.s2 = c.b2 * x - c.a2 * y;
        y
    }

    fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThreeBandEqParams {
    pub low_db: f32,
    pub mid_db: f32,
    pub high_db: f32,
}

impl Default for ThreeBandEqParams {
    fn default() -> Self {
        Self {
            low_db: 0.0,
            mid_db: 0.0,
            high_db: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThreeBandEq {
    sample_rate: f32,
    params: ThreeBandEqParams,
    target_params: ThreeBandEqParams,
    // Filter coefficients
    low_coeffs: BiquadCoeffs,
    mid_coeffs: BiquadCoeffs,
    high_coeffs: BiquadCoeffs,
    // Per-channel state: (Left, Right)
    low_state: [BiquadState; 2],
    mid_state: [BiquadState; 2],
    high_state: [BiquadState; 2],
}

impl ThreeBandEq {
    pub fn new(sample_rate: f32) -> Self {
        let mut eq = Self {
            sample_rate,
            params: ThreeBandEqParams::default(),
            target_params: ThreeBandEqParams::default(),
            low_coeffs: BiquadCoeffs::default(),
            mid_coeffs: BiquadCoeffs::default(),
            high_coeffs: BiquadCoeffs::default(),
            low_state: [BiquadState::default(); 2],
            mid_state: [BiquadState::default(); 2],
            high_state: [BiquadState::default(); 2],
        };
        eq.recompute_coeffs();
        eq
    }

    pub fn params(&self) -> ThreeBandEqParams {
        self.params
    }

    pub fn set_params(&mut self, params: ThreeBandEqParams) {
        self.target_params = params;
        self.params = params;
        self.recompute_coeffs();
    }

    pub fn set_low_db(&mut self, db: f32) {
        self.target_params.low_db = db;
        self.params.low_db = db;
        self.recompute_coeffs();
    }

    pub fn set_mid_db(&mut self, db: f32) {
        self.target_params.mid_db = db;
        self.params.mid_db = db;
        self.recompute_coeffs();
    }

    pub fn set_high_db(&mut self, db: f32) {
        self.target_params.high_db = db;
        self.params.high_db = db;
        self.recompute_coeffs();
    }

    pub fn reset_state(&mut self) {
        for ch in 0..2 {
            self.low_state[ch].reset();
            self.mid_state[ch].reset();
            self.high_state[ch].reset();
        }
    }

    /// Process a stereo frame (left, right) in place
    #[inline]
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        let l1 = self.low_state[0].process(left, &self.low_coeffs);
        let l2 = self.mid_state[0].process(l1, &self.mid_coeffs);
        let out_l = self.high_state[0].process(l2, &self.high_coeffs);

        let r1 = self.low_state[1].process(right, &self.low_coeffs);
        let r2 = self.mid_state[1].process(r1, &self.mid_coeffs);
        let out_r = self.high_state[1].process(r2, &self.high_coeffs);

        (out_l, out_r)
    }

    fn recompute_coeffs(&mut self) {
        self.low_coeffs = compute_low_shelf(self.sample_rate, 200.0, self.params.low_db);
        self.mid_coeffs = compute_peaking(self.sample_rate, 1000.0, 0.707, self.params.mid_db);
        self.high_coeffs = compute_high_shelf(self.sample_rate, 4000.0, self.params.high_db);
    }
}

fn compute_low_shelf(sr: f32, f0: f32, gain_db: f32) -> BiquadCoeffs {
    if gain_db <= -59.0 {
        // Deep notch / isolator cut
        return compute_high_pass(sr, f0);
    }

    let a = 10.0f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * (f0 / sr);
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / 2.0 * 2.0f32.sqrt(); // Q = 0.707

    let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
    let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
    let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

    let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
    let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
    let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

fn compute_high_shelf(sr: f32, f0: f32, gain_db: f32) -> BiquadCoeffs {
    if gain_db <= -59.0 {
        // Isolator cut
        return compute_low_pass(sr, f0);
    }

    let a = 10.0f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * (f0 / sr);
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / 2.0 * 2.0f32.sqrt(); // Q = 0.707

    let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
    let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
    let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

    let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
    let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
    let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

fn compute_peaking(sr: f32, f0: f32, q: f32, gain_db: f32) -> BiquadCoeffs {
    let a = 10.0f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * (f0 / sr);
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);

    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha / a;

    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * cos_w0;
    let b2 = 1.0 - alpha * a;

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

fn compute_high_pass(sr: f32, f0: f32) -> BiquadCoeffs {
    let w0 = 2.0 * PI * (f0 / sr);
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * 0.707);

    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;

    let b0 = (1.0 + cos_w0) / 2.0;
    let b1 = -(1.0 + cos_w0);
    let b2 = (1.0 + cos_w0) / 2.0;

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

fn compute_low_pass(sr: f32, f0: f32) -> BiquadCoeffs {
    let w0 = 2.0 * PI * (f0 / sr);
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * 0.707);

    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;

    let b0 = (1.0 - cos_w0) / 2.0;
    let b1 = 1.0 - cos_w0;
    let b2 = (1.0 - cos_w0) / 2.0;

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}
