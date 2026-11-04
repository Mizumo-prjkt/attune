//! Liquid Material Glass design system and honest audio badges for Attune.
//!
//! Complies strictly with:
//! - `ledger/05_UI_DESIGN_SPEC.md` (Liquid Glass stack, Specular Rim, Honest Audio Badges)
//! - Mandate 3.1 (No fake Dolby Atmos/MQA hype; honest audio quality pills)

/// Honest Audio Quality category based on decoded container & sample rate metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioQualityCategory {
    /// High-Quality Lossless: FLAC, WAV, ALAC, AIFF with sample_rate >= 44100 Hz.
    VeryGood,
    /// Standard Clean Lossy or Low-Rate FLAC: MP3, AAC, OGG, Opus, or <= 32000 Hz.
    Good,
    /// Proprietary/Unnatural Codec: AC-3, E-AC-3, MQA, or DRM-wrapped formats.
    Proprietary,
}

impl AudioQualityCategory {
    pub fn classify(format: &str, sample_rate: u32) -> Self {
        let fmt = format.trim().to_uppercase();
        if fmt.contains("ATMOS") || fmt.contains("AC-3") || fmt.contains("MQA") || fmt.contains("EAC3") {
            AudioQualityCategory::Proprietary
        } else if (fmt == "FLAC" || fmt == "WAV" || fmt == "ALAC" || fmt == "AIFF") && sample_rate >= 44_100 {
            AudioQualityCategory::VeryGood
        } else {
            AudioQualityCategory::Good
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AudioQualityCategory::VeryGood => "Very Good Quality Audio",
            AudioQualityCategory::Good => "Good Quality Audio",
            AudioQualityCategory::Proprietary => "Proprietary Audio Codec",
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            AudioQualityCategory::VeryGood => "quality-badge quality-very-good",
            AudioQualityCategory::Good => "quality-badge quality-good",
            AudioQualityCategory::Proprietary => "quality-badge quality-proprietary",
        }
    }
}
