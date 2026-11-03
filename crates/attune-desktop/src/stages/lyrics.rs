//! Synchronized Lyrics viewer stage for Attune egui.
//!
//! Features:
//! - Full-sheet scrollable lyrics
//! - Real-time active singing line focus with glowing glass backdrop
//! - Dimmed past lines and semi-translucent upcoming lines
//! - Auto-scroll to active line with user scroll detection and [Sync to Current Line] button

use eframe::egui::{self, Color32, CornerRadius, Stroke, Ui};
use crate::components::lyrics_parser::LyricsSheet;

pub struct LyricsStage {
    auto_scroll_locked: bool,
    last_scrolled_line: Option<usize>,
}

impl Default for LyricsStage {
    fn default() -> Self {
        Self {
            auto_scroll_locked: true,
            last_scrolled_line: None,
        }
    }
}

impl LyricsStage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ui: &mut Ui, lyrics: &LyricsSheet, current_pos_ms: u64) {
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Synchronized Lyrics").strong().color(crate::theme::ACCENT));
            ui.label(egui::RichText::new(format!("• Source: {}", lyrics.source_label)).weak().small());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let lock_label = if self.auto_scroll_locked { "🔒 Auto-Scroll: ON" } else { "🔓 Free-Scroll" };
                if ui.selectable_label(self.auto_scroll_locked, lock_label).clicked() {
                    self.auto_scroll_locked = !self.auto_scroll_locked;
                }
            });
        });

        ui.separator();

        if lyrics.lines.is_empty() {
            ui.add_space(32.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No lyrics available for this track.").weak().size(16.0));
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        "Place a companion .lrc file next to your music file, or embed Vorbis/ID3 lyrics.",
                    )
                    .small()
                    .weak(),
                );
            });
            return;
        }

        let active_idx = lyrics.active_line_index(current_pos_ms);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(20.0);

                for (i, line) in lyrics.lines.iter().enumerate() {
                    let is_active = active_idx == Some(i);
                    let is_past = active_idx.map(|a| i < a).unwrap_or(false);

                    let (text_color, font_size, bg_fill, border_stroke) = if is_active {
                        (
                            crate::theme::ACCENT,
                            18.0,
                            Color32::from_rgba_premultiplied(0x10, 0x30, 0x2A, 180),
                            Stroke::new(1.0, crate::theme::ACCENT.linear_multiply(0.6)),
                        )
                    } else if is_past {
                        (
                            Color32::from_white_alpha(75),
                            14.0,
                            Color32::TRANSPARENT,
                            Stroke::NONE,
                        )
                    } else {
                        (
                            Color32::from_white_alpha(160),
                            14.0,
                            Color32::TRANSPARENT,
                            Stroke::NONE,
                        )
                    };

                    let line_rect = ui.horizontal(|ui| {
                        if lyrics.is_timed {
                            let total_sec = line.timestamp_ms / 1000;
                            let time_str = format!("{}:{:02}", total_sec / 60, total_sec % 60);
                            ui.label(
                                egui::RichText::new(time_str)
                                    .monospace()
                                    .size(11.0)
                                    .color(Color32::from_white_alpha(50)),
                            );
                            ui.add_space(10.0);
                        }

                        let text = egui::RichText::new(&line.text)
                            .size(font_size)
                            .color(text_color);
                        let text = if is_active { text.strong() } else { text };
                        ui.label(text);
                    }).response.rect;

                    if is_active && bg_fill != Color32::TRANSPARENT {
                        let padded_rect = line_rect.expand2(egui::vec2(12.0, 4.0));
                        ui.painter().rect(
                            padded_rect,
                            CornerRadius::same(8u8),
                            bg_fill,
                            border_stroke,
                            egui::StrokeKind::Inside,
                        );
                    }

                    if is_active && self.auto_scroll_locked && self.last_scrolled_line != Some(i) {
                        ui.scroll_to_rect(line_rect, Some(egui::Align::Center));
                        self.last_scrolled_line = Some(i);
                    }

                    ui.add_space(10.0);
                }

                ui.add_space(40.0);
            });
    }
}
