//! Liquid Material Glass theme & dynamic color engine for Attune egui.
//!
//! Complies with `ledger/05_UI_DESIGN_SPEC.md`:
//! - Primary Accent: `#2DD4BF` (Prismatic Mint / Teal)
//! - Secondary Accent: `#A855F7` (Electric Violet)
//! - Tertiary Accent: `#FB7185` (Coral Rose)
//! - Deep Canvas Base: `#090A10`
//! - Dynamic album cover art color extraction

use std::path::Path;
use eframe::egui::{self, Color32, Pos2, Rect, Ui};

pub const ACCENT: Color32 = Color32::from_rgb(0x2E, 0xE6, 0xC5);
pub const PRIMARY: Color32 = Color32::from_rgb(0x2D, 0xD4, 0xBF);
pub const SECONDARY: Color32 = Color32::from_rgb(0xA8, 0x55, 0xF7);
pub const TERTIARY: Color32 = Color32::from_rgb(0xFB, 0x71, 0x85);
pub const CANVAS_BASE: Color32 = Color32::from_rgb(0x09, 0x0A, 0x10);

#[derive(Debug, Clone, Copy)]
pub struct DynamicPalette {
    pub primary: Color32,
    pub secondary: Color32,
    pub tertiary: Color32,
}

impl Default for DynamicPalette {
    fn default() -> Self {
        Self {
            primary: PRIMARY,
            secondary: SECONDARY,
            tertiary: TERTIARY,
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
                best_color = Some(Color32::from_rgb(p[0], p[1], p[2]));
            }
        }

        let prim = best_color.unwrap_or(PRIMARY);
        // Generate complementary secondary and tertiary accents
        let sec = Color32::from_rgb(prim.b(), prim.r(), prim.g());
        let tert = Color32::from_rgb(prim.g(), prim.b(), prim.r());

        Some(Self {
            primary: prim,
            secondary: sec,
            tertiary: tert,
        })
    }
}

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgba_premultiplied(0x0C, 0x10, 0x18, 235);
    visuals.window_fill = Color32::from_rgba_premultiplied(0x10, 0x15, 0x22, 240);
    visuals.faint_bg_color = Color32::from_rgb(0x14, 0x1A, 0x26);
    visuals.extreme_bg_color = CANVAS_BASE;
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.35);
    visuals.selection.stroke.color = ACCENT;
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.hovered.bg_stroke.color = ACCENT.linear_multiply(0.7);
    visuals.widgets.active.bg_stroke.color = ACCENT;
    ctx.set_visuals(visuals);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(10.0, 7.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
    });
}

/// Paints the living ambient glass background mesh behind the desktop scaffold.
pub fn paint_living_ambient_mesh(ui: &Ui, rect: Rect, palette: &DynamicPalette, time: f32) {
    let painter = ui.painter_at(rect);

    // Deep canvas backdrop
    painter.rect_filled(rect, 0.0, CANVAS_BASE);

    // Living breathing orbs sampled from the dynamic palette
    let t = time * 0.4;
    let orb1_center = Pos2::new(
        rect.left() + rect.width() * (0.25 + 0.10 * (t * 1.1).sin()),
        rect.top() + rect.height() * (0.30 + 0.12 * (t * 0.9).cos()),
    );
    let orb2_center = Pos2::new(
        rect.right() - rect.width() * (0.28 + 0.12 * (t * 0.8).cos()),
        rect.bottom() - rect.height() * (0.32 + 0.10 * (t * 1.2).sin()),
    );

    let orb1_col = Color32::from_rgba_premultiplied(
        (palette.primary.r() as f32 * 0.12) as u8,
        (palette.primary.g() as f32 * 0.12) as u8,
        (palette.primary.b() as f32 * 0.12) as u8,
        35,
    );
    let orb2_col = Color32::from_rgba_premultiplied(
        (palette.secondary.r() as f32 * 0.10) as u8,
        (palette.secondary.g() as f32 * 0.10) as u8,
        (palette.secondary.b() as f32 * 0.10) as u8,
        30,
    );
    let orb3_center = Pos2::new(
        rect.left() + rect.width() * (0.55 + 0.14 * (t * 0.7).sin()),
        rect.top() + rect.height() * (0.60 + 0.12 * (t * 1.3).cos()),
    );
    let orb3_col = Color32::from_rgba_premultiplied(
        (palette.tertiary.r() as f32 * 0.08) as u8,
        (palette.tertiary.g() as f32 * 0.08) as u8,
        (palette.tertiary.b() as f32 * 0.08) as u8,
        25,
    );

    painter.circle_filled(orb1_center, rect.width() * 0.35, orb1_col);
    painter.circle_filled(orb2_center, rect.width() * 0.40, orb2_col);
    painter.circle_filled(orb3_center, rect.width() * 0.30, orb3_col);
}
