//! Micro-pitch shifting & time stretching utilities

#[derive(Debug, Clone)]
pub struct PitchShifter {
    pub sample_rate: f32,
}

impl PitchShifter {
    pub fn new(sample_rate: f32) -> Self {
        Self { sample_rate }
    }
}
