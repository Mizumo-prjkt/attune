//! Reverb & Tail Freeze effect for Archetype 4 transitions
//!
//! Uses a multi-tap recirculating diffusion network with freeze hold and gentle decay.

#[derive(Debug, Clone)]
pub struct ReverbFreeze {
    sample_rate: f32,
    freeze_active: bool,
    freeze_send: f32,
    // Delay lines with prime lengths
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    idx_l: usize,
    idx_r: usize,
    // Damping state (one-pole lowpass)
    damp_l: f32,
    damp_r: f32,
    // Freeze decay accumulator
    decay: f32,
}

impl ReverbFreeze {
    pub fn new(sample_rate: f32) -> Self {
        // ~250ms buffer for freeze circulation
        let len_l = ((sample_rate * 0.231) as usize).max(1024);
        let len_r = ((sample_rate * 0.267) as usize).max(1024);

        Self {
            sample_rate,
            freeze_active: false,
            freeze_send: 0.0,
            delay_l: vec![0.0; len_l],
            delay_r: vec![0.0; len_r],
            idx_l: 0,
            idx_r: 0,
            damp_l: 0.0,
            damp_r: 0.0,
            decay: 1.0,
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn set_freeze(&mut self, active: bool, send: f32) {
        self.freeze_active = active;
        self.freeze_send = send.clamp(0.0, 1.0);
        if !active {
            self.decay = 1.0;
        }
    }

    pub fn reset(&mut self) {
        self.delay_l.fill(0.0);
        self.delay_r.fill(0.0);
        self.idx_l = 0;
        self.idx_r = 0;
        self.damp_l = 0.0;
        self.damp_r = 0.0;
        self.decay = 1.0;
    }

    /// Process a stereo frame (left, right), returning the wet reverb/freeze output
    #[inline]
    pub fn process_stereo(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let len_l = self.delay_l.len();
        let len_r = self.delay_r.len();

        let out_l = self.delay_l[self.idx_l];
        let out_r = self.delay_r[self.idx_r];

        // One-pole damping (absorption of high frequencies in freeze tail)
        self.damp_l = self.damp_l * 0.35 + out_l * 0.65;
        self.damp_r = self.damp_r * 0.35 + out_r * 0.65;

        let feedback = if self.freeze_active {
            // Decay over ~3.5 seconds
            self.decay *= 0.99998;
            0.995 * self.decay
        } else {
            0.85
        };

        let write_l = if self.freeze_active {
            // Recirculate with cross-channel diffusion
            (self.damp_r * 0.4 + self.damp_l * 0.6) * feedback
        } else {
            in_l * self.freeze_send + self.damp_r * 0.3 * feedback
        };

        let write_r = if self.freeze_active {
            (self.damp_l * 0.4 + self.damp_r * 0.6) * feedback
        } else {
            in_r * self.freeze_send + self.damp_l * 0.3 * feedback
        };

        self.delay_l[self.idx_l] = write_l;
        self.delay_r[self.idx_r] = write_r;

        self.idx_l = (self.idx_l + 1) % len_l;
        self.idx_r = (self.idx_r + 1) % len_r;

        (out_l * self.freeze_send, out_r * self.freeze_send)
    }
}
