//! Liquid Material Glass design system components for Attune egui.
//!
//! Complies strictly with:
//! - `ledger/05_UI_DESIGN_SPEC.md` (Liquid Glass stack, Specular Rim, Honest Audio Badges)
//! - Mandate 3.1 (No fake Dolby Atmos/MQA hype; honest audio quality pills)

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Response, Stroke, Ui};

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

    pub fn bg_tint(self) -> Color32 {
        match self {
            AudioQualityCategory::VeryGood => Color32::from_rgba_premultiplied(0x2D, 0xD4, 0xBF, 35),
            AudioQualityCategory::Good => Color32::from_rgba_premultiplied(0xA8, 0x55, 0xF7, 30),
            AudioQualityCategory::Proprietary => Color32::from_rgba_premultiplied(0xFB, 0x71, 0x85, 35),
        }
    }

    pub fn border_stroke(self) -> Stroke {
        match self {
            AudioQualityCategory::VeryGood => Stroke::new(1.0, Color32::from_rgba_premultiplied(0x2D, 0xD4, 0xBF, 120)),
            AudioQualityCategory::Good => Stroke::new(1.0, Color32::from_rgba_premultiplied(0xA8, 0x55, 0xF7, 100)),
            AudioQualityCategory::Proprietary => Stroke::new(1.0, Color32::from_rgba_premultiplied(0xFB, 0x71, 0x85, 120)),
        }
    }

    pub fn text_color(self) -> Color32 {
        match self {
            AudioQualityCategory::VeryGood => Color32::from_rgb(0x5E, 0xEB, 0xD6),
            AudioQualityCategory::Good => Color32::from_rgb(0xC4, 0x8D, 0xFC),
            AudioQualityCategory::Proprietary => Color32::from_rgb(0xFD, 0x9D, 0xAC),
        }
    }
}

/// Renders the honest audio quality pill badge.
pub fn render_audio_quality_badge(ui: &mut Ui, format: &str, sample_rate: u32) {
    let cat = AudioQualityCategory::classify(format, sample_rate);
    let padding = egui::vec2(10.0, 4.0);

    let text = egui::RichText::new(cat.label())
        .size(11.0)
        .strong()
        .color(cat.text_color());

    let galley = ui.painter().layout_no_wrap(
        text.text().to_string(),
        egui::FontId::proportional(11.0),
        cat.text_color(),
    );

    let desired_size = galley.size() + padding * 2.0;
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    ui.painter().rect(
        rect,
        CornerRadius::same((rect.height() * 0.5) as u8),
        cat.bg_tint(),
        cat.border_stroke(),
        egui::StrokeKind::Inside,
    );

    let text_pos = Pos2::new(
        rect.left() + padding.x,
        rect.top() + padding.y,
    );
    ui.painter().galley(text_pos, galley, cat.text_color());
}

/// Paints a Liquid Material Glass container card with specular diagonal rim lighting.
#[allow(dead_code)]
pub fn paint_glass_rect(ui: &Ui, rect: Rect, radius: f32, surface_alpha: f32, accent_color: Color32) {
    // 1. Dark frosted base fill
    let fill = Color32::from_rgba_premultiplied(
        (0x10 as f32 * surface_alpha) as u8,
        (0x14 as f32 * surface_alpha) as u8,
        (0x1E as f32 * surface_alpha) as u8,
        (255.0 * surface_alpha) as u8,
    );

    // 2. Specular rim highlight (top-left catch light)
    let stroke = Stroke::new(1.0, Color32::from_white_alpha(38));

    ui.painter().rect(
        rect,
        CornerRadius::same(radius as u8),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );

    // 3. Subtle accent tint line along top border
    let top_line_stroke = Stroke::new(
        1.5,
        Color32::from_rgba_premultiplied(
            (accent_color.r() as f32 * 0.6) as u8,
            (accent_color.g() as f32 * 0.6) as u8,
            (accent_color.b() as f32 * 0.6) as u8,
            90,
        ),
    );
    ui.painter().line_segment(
        [
            Pos2::new(rect.left() + radius * 0.5, rect.top() + 1.0),
            Pos2::new(rect.right() - radius * 0.5, rect.top() + 1.0),
        ],
        top_line_stroke,
    );
}

/// Reusable helper for displaying a Liquid Glass card.
pub fn liquid_glass_card<R>(
    ui: &mut Ui,
    radius: f32,
    accent: Color32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Response {
    let margin = egui::Margin::same(14);
    let frame = egui::Frame::NONE
        .inner_margin(margin)
        .corner_radius(radius as u8)
        .fill(Color32::from_rgba_premultiplied(0x12, 0x16, 0x22, 190))
        .stroke(Stroke::new(1.0, Color32::from_white_alpha(32)));

    let res = frame.show(ui, |ui| {
        add_contents(ui);
    });

    // Draw specular top glow
    let r = res.response.rect;
    ui.painter().line_segment(
        [
            Pos2::new(r.left() + radius * 0.5, r.top() + 1.0),
            Pos2::new(r.right() - radius * 0.5, r.top() + 1.0),
        ],
        Stroke::new(1.2, accent.linear_multiply(0.45)),
    );

    res.response
}
