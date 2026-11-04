//! Liquid Material Glass theme & dynamic color engine for Attune.
//!
//! Complies with `ledger/05_UI_DESIGN_SPEC.md`:
//! - Primary Accent: `#2DD4BF` (Prismatic Mint / Teal)
//! - Secondary Accent: `#A855F7` (Electric Violet)
//! - Tertiary Accent: `#FB7185` (Coral Rose)
//! - Deep Canvas Base: `#090A10`
//! - Dynamic album cover art color extraction

use std::path::Path;

pub const PRIMARY_HEX: &str = "#2DD4BF";
pub const SECONDARY_HEX: &str = "#A855F7";
pub const TERTIARY_HEX: &str = "#FB7185";
pub const CANVAS_BASE_HEX: &str = "#090A10";

#[derive(Debug, Clone)]
pub struct DynamicPalette {
    pub primary: String,
    pub secondary: String,
    pub tertiary: String,
}

impl Default for DynamicPalette {
    fn default() -> Self {
        Self {
            primary: PRIMARY_HEX.to_string(),
            secondary: SECONDARY_HEX.to_string(),
            tertiary: TERTIARY_HEX.to_string(),
        }
    }
}

impl DynamicPalette {
    /// Extracts dominant vibrant colors from an image file on disk.
    pub fn from_image(path: &Path) -> Option<Self> {
        let reader = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?;
        let img = reader.decode().ok()?.thumbnail(32, 32);
        let rgb = img.to_rgb8();

        let mut best_color = None;
        let mut max_saturation = 0.0f32;

        for p in rgb.pixels() {
            let r = p[0] as f32 / 255.0;
            let g = p[1] as f32 / 255.0;
            let b = p[2] as f32 / 255.0;

            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let delta = max - min;

            // Skip dark blacks, washed out whites, and pure grays
            let brightness = (r + g + b) / 3.0;
            if !(0.15..=0.88).contains(&brightness) || delta < 0.15 {
                continue;
            }

            let sat = if max > 0.0 { delta / max } else { 0.0 };
            if sat > max_saturation {
                max_saturation = sat;
                best_color = Some((p[0], p[1], p[2]));
            }
        }

        let prim = best_color.unwrap_or((0x2D, 0xD4, 0xBF));
        let sec = (prim.2, prim.0, prim.1);
        let tert = (prim.1, prim.2, prim.0);

        Some(Self {
            primary: format!("rgb({}, {}, {})", prim.0, prim.1, prim.2),
            secondary: format!("rgb({}, {}, {})", sec.0, sec.1, sec.2),
            tertiary: format!("rgb({}, {}, {})", tert.0, tert.1, tert.2),
        })
    }
}
