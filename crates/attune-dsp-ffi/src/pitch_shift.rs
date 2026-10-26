//! Resampling and Pitch Shifting primitives

#[inline]
pub fn cents_to_ratio(cents: i32) -> f32 {
    2.0f32.powf(cents as f32 / 1200.0)
}

#[inline]
pub fn ratio_to_cents(ratio: f32) -> i32 {
    if ratio <= 0.0 {
        return 0;
    }
    (1200.0 * ratio.log2()).round() as i32
}

/// Cubic Hermite interpolation between 4 consecutive audio samples
#[inline]
pub fn interpolate_hermite(y0: f32, y1: f32, y2: f32, y3: f32, frac: f32) -> f32 {
    let c0 = y1;
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

    ((c3 * frac + c2) * frac + c1) * frac + c0
}

/// Variable-speed fractional resampler
#[derive(Debug, Clone)]
pub struct Resampler {
    speed_ratio: f64,
    fractional_pos: f64,
}

impl Default for Resampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Resampler {
    pub fn new() -> Self {
        Self {
            speed_ratio: 1.0,
            fractional_pos: 0.0,
        }
    }

    pub fn set_speed_ratio(&mut self, ratio: f64) {
        self.speed_ratio = ratio.clamp(0.5, 2.0);
    }

    pub fn speed_ratio(&self) -> f64 {
        self.speed_ratio
    }

    pub fn reset(&mut self) {
        self.fractional_pos = 0.0;
    }
}
