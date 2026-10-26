//! Reverb & Tail Freeze effect for Archetype 4 transitions

#[derive(Debug, Clone)]
pub struct ReverbFreeze {
    pub sample_rate: f32,
    pub freeze_active: bool,
}

impl ReverbFreeze {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            freeze_active: false,
        }
    }
}
