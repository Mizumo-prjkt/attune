//! Minimal dark theme tuned toward the Attune teal accent.
//! The full Liquid Material Glass skin (ledger/05_UI_DESIGN_SPEC.md) comes later.

use eframe::egui::{self, Color32};

pub const ACCENT: Color32 = Color32::from_rgb(0x2E, 0xE6, 0xC5);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(0x0E, 0x12, 0x18);
    visuals.window_fill = Color32::from_rgb(0x12, 0x17, 0x1F);
    visuals.faint_bg_color = Color32::from_rgb(0x18, 0x1E, 0x28);
    visuals.extreme_bg_color = Color32::from_rgb(0x0A, 0x0D, 0x12);
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.35);
    visuals.selection.stroke.color = ACCENT;
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.hovered.bg_stroke.color = ACCENT.linear_multiply(0.6);
    ctx.set_visuals(visuals);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
    });
}
