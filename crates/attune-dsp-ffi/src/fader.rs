//! Fader and crossfading math for DJ mixing

use std::f32::consts::PI;

#[inline]
pub fn db_to_linear(db: f32) -> f32 {
    if db <= -59.0 {
        0.0
    } else {
        10.0f32.powf(db / 20.0)
    }
}

#[inline]
pub fn linear_to_db(linear: f32) -> f32 {
    if linear <= 1e-4 {
        -80.0
    } else {
        20.0 * linear.log10()
    }
}

/// Equal-power 3dB crossfade curve: cos(t * pi/2), sin(t * pi/2)
#[inline]
pub fn equal_power_crossfade(t: f32) -> (f32, f32) {
    let t_clamped = t.clamp(0.0, 1.0);
    let angle = t_clamped * (PI / 2.0);
    (angle.cos(), angle.sin())
}

/// Linear crossfade curve: 1 - t, t
#[inline]
pub fn linear_crossfade(t: f32) -> (f32, f32) {
    let t_clamped = t.clamp(0.0, 1.0);
    (1.0 - t_clamped, t_clamped)
}

/// S-Curve crossfade: smooth cubic hermite curve
#[inline]
pub fn s_curve_crossfade(t: f32) -> (f32, f32) {
    let t_clamped = t.clamp(0.0, 1.0);
    let s = t_clamped * t_clamped * (3.0 - 2.0 * t_clamped);
    (1.0 - s, s)
}
