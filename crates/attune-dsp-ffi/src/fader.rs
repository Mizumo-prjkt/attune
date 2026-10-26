//! Equal-power crossfade curves

pub fn equal_power_fade(progress: f32) -> (f32, f32) {
    let p = progress.clamp(0.0, 1.0);
    let angle = p * std::f32::consts::FRAC_PI_2;
    (angle.cos(), angle.sin())
}
