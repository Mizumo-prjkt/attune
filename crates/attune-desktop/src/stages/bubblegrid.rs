//! Bubblegrid / Bubble Navigation honeycomb stage for Attune egui (ADR-052 Parity).
//!
//! Features:
//! - Hexagonal concentric honeycomb lattice layout
//! - Smooth cosine fisheye magnification lens centered on cursor / center
//! - 2D momentum pan physics
//! - Floating Liquid Glass highlight card with cover art, track info, and 1-click play

use std::f32::consts::PI;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Ui, Vec2};

use crate::components::glass::liquid_glass_card;
use crate::library::TrackEntry;

pub struct BubblegridStage {
    pan_offset: Vec2,
    selected_track_idx: Option<usize>,
}

impl Default for BubblegridStage {
    fn default() -> Self {
        Self {
            pan_offset: Vec2::ZERO,
            selected_track_idx: None,
        }
    }
}

impl BubblegridStage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        tracks: &[TrackEntry],
        on_play_track: &mut impl FnMut(usize),
    ) {
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Bubblegrid Honeycomb Navigation").strong().color(crate::theme::ACCENT));
            ui.label(egui::RichText::new("Fisheye Lens Cosine Magnification").weak().small());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Reset Pan").clicked() {
                    self.pan_offset = Vec2::ZERO;
                }
            });
        });

        ui.separator();

        if tracks.is_empty() {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("No tracks loaded to populate Bubblegrid.").weak());
            return;
        }

        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

        // 1. Handle 2D pan dragging
        if response.dragged() {
            self.pan_offset += response.drag_delta();
        }

        // Clip drawing to canvas rect
        let painter = ui.painter_at(rect);
        let center = rect.center() + self.pan_offset;
        let cursor_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(rect.center());

        let base_radius = 42.0f32;
        let hex_spacing = base_radius * 2.15;

        // Generate hexagonal coordinates: ring 0 (center), ring 1 (6), ring 2 (12), ring 3 (18)...
        let mut hex_positions = Vec::with_capacity(tracks.len());
        hex_positions.push(Pos2::ZERO);

        let mut ring = 1;
        'gen: while hex_positions.len() < tracks.len() {
            for side in 0..6 {
                for step in 0..ring {
                    let angle1 = (side as f32) * (PI / 3.0);
                    let angle2 = ((side + 1) as f32) * (PI / 3.0);

                    let p1 = Pos2::new(angle1.cos(), angle1.sin()) * (ring as f32 * hex_spacing);
                    let p2 = Pos2::new(angle2.cos(), angle2.sin()) * (ring as f32 * hex_spacing);

                    let t = step as f32 / ring as f32;
                    let p = p1 + (p2 - p1) * t;
                    hex_positions.push(p);

                    if hex_positions.len() >= tracks.len() {
                        break 'gen;
                    }
                }
            }
            ring += 1;
        }

        let mut closest_dist = f32::MAX;
        let mut closest_idx = None;

        // 2. Render bubbles with cosine magnification
        for (idx, &hex_pos) in hex_positions.iter().take(tracks.len()).enumerate() {
            let bubble_center = center + hex_pos.to_vec2();

            // Skip bubbles far off screen
            if !rect.expand(100.0).contains(bubble_center) {
                continue;
            }

            // Fisheye magnification factor: 1.45x near cursor/center down to 0.65x
            let dist_to_cursor = bubble_center.distance(cursor_pos);
            let lens_radius = 280.0f32;
            let zoom = if dist_to_cursor < lens_radius {
                let norm = dist_to_cursor / lens_radius;
                0.70 + 0.75 * (norm * PI * 0.5).cos()
            } else {
                0.65
            };

            let r = base_radius * zoom;

            // Track closest bubble to center for highlight card
            let dist_to_screen_center = bubble_center.distance(rect.center());
            if dist_to_screen_center < closest_dist {
                closest_dist = dist_to_screen_center;
                closest_idx = Some(idx);
            }

            let is_hovered = bubble_center.distance(cursor_pos) < r;
            let bg_color = if is_hovered {
                crate::theme::ACCENT.linear_multiply(0.40)
            } else {
                Color32::from_rgba_premultiplied(0x18, 0x22, 0x30, 220)
            };

            let stroke = if is_hovered {
                Stroke::new(2.0, crate::theme::ACCENT)
            } else {
                Stroke::new(1.0, Color32::from_white_alpha(35))
            };

            painter.circle(bubble_center, r, bg_color, stroke);

            // Thumbnail text or image
            let t = &tracks[idx];
            let initial = t.title.chars().next().unwrap_or('♪').to_string();
            painter.text(
                bubble_center,
                egui::Align2::CENTER_CENTER,
                initial,
                egui::FontId::proportional((r * 0.75).clamp(10.0, 32.0)),
                Color32::from_white_alpha(200),
            );

            // Double click bubble to play
            if response.double_clicked() && is_hovered {
                on_play_track(idx);
            }
        }

        if let Some(idx) = closest_idx {
            self.selected_track_idx = Some(idx);
        }

        // 3. Floating Liquid Glass Highlight Card at bottom right of canvas
        if let Some(idx) = self.selected_track_idx {
            if let Some(t) = tracks.get(idx) {
                let card_w = 280.0;
                let card_h = 100.0;
                let card_rect = Rect::from_min_size(
                    Pos2::new(rect.right() - card_w - 20.0, rect.bottom() - card_h - 20.0),
                    egui::vec2(card_w, card_h),
                );

                ui.put(card_rect, |ui: &mut Ui| {
                    liquid_glass_card(ui, 16.0, crate::theme::ACCENT, |ui| {
                        ui.horizontal(|ui| {
                            let art_dim = 64.0;
                            if let Some(p) = &t.artwork_path {
                                ui.add(
                                    egui::Image::new(format!("file://{}", p.display()))
                                        .fit_to_exact_size(egui::vec2(art_dim, art_dim))
                                        .corner_radius(10.0),
                                );
                            } else {
                                let (r, _) = ui.allocate_exact_size(egui::vec2(art_dim, art_dim), egui::Sense::hover());
                                ui.painter().rect_filled(r, 10.0, Color32::from_rgb(0x18, 0x22, 0x30));
                                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "♪", egui::FontId::proportional(28.0), Color32::from_white_alpha(70));
                            }

                            ui.vertical(|ui| {
                                ui.add(egui::Label::new(egui::RichText::new(&t.title).strong().size(14.0)).truncate());
                                ui.add(egui::Label::new(egui::RichText::new(&t.artist).weak().size(12.0)).truncate());
                                if ui.small_button("▶ Play Track").clicked() {
                                    on_play_track(idx);
                                }
                            });
                        });
                    })
                });
            }
        }
    }
}
