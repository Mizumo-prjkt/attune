//! 3-Band DJ Isolator EQ (-24 dB / mute to +6 dB)

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
