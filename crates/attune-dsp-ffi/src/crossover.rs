//! 4-Band Crossover Filter (Linkwitz-Riley / Biquad)

#[derive(Debug, Clone)]
pub struct FourBandCrossover {
    pub sample_rate: f32,
    pub sub_cutoff: f32,
    pub lowmid_cutoff: f32,
    pub vocal_cutoff: f32,
}

impl FourBandCrossover {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            sub_cutoff: 150.0,
            lowmid_cutoff: 600.0,
            vocal_cutoff: 4000.0,
        }
    }
}
