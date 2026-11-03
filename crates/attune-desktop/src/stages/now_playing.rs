//! Now Playing stage with 3 adaptive skins:
//! 1. Default Liquid Glass
//! 2. Material You Expressive Freeshapes
//! 3. Normal Square Big (edge-to-edge square art, left-aligned text, inline timed lyrics snippet)

use eframe::egui::{self, Color32, CornerRadius, Rect, Stroke, Ui};

use crate::components::freeshape::{paint_freeshape_backdrop, FreeshapeRecencyTracker};
use crate::components::glass::render_audio_quality_badge;
use crate::components::lyrics_parser::LyricsSheet;
use crate::library::{fmt_ms, TrackEntry};
use crate::player::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NowPlayingSkin {
    LiquidGlass,
    ExpressiveFreeshapes,
    NormalSquareBig,
}

impl NowPlayingSkin {
    pub const ALL: [NowPlayingSkin; 3] = [
        NowPlayingSkin::LiquidGlass,
        NowPlayingSkin::ExpressiveFreeshapes,
        NowPlayingSkin::NormalSquareBig,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NowPlayingSkin::LiquidGlass => "Default Liquid Glass",
            NowPlayingSkin::ExpressiveFreeshapes => "Material You Freeshapes",
            NowPlayingSkin::NormalSquareBig => "Normal Square Big",
        }
    }
}

pub struct NowPlayingStage {
    pub skin: NowPlayingSkin,
    pub freeshape_tracker: FreeshapeRecencyTracker,
}

impl Default for NowPlayingStage {
    fn default() -> Self {
        Self {
            skin: NowPlayingSkin::LiquidGlass,
            freeshape_tracker: FreeshapeRecencyTracker::new(),
        }
    }
}

impl NowPlayingStage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_track_changed(&mut self, track: &TrackEntry) {
        let seed = track.title.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64));
        self.freeshape_tracker.trigger_next_shape(seed);
    }

    pub fn update_animations(&mut self, dt: f32) {
        self.freeshape_tracker.step_morph(dt);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        current_track: Option<&TrackEntry>,
        player: &Player,
        lyrics: &LyricsSheet,
        position_ms: u64,
        duration_ms: u64,
        is_playing: bool,
    ) {
        // Skin selector header
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Now Playing Skin:").weak().small());
            for skin in NowPlayingSkin::ALL {
                if ui.selectable_label(self.skin == skin, skin.label()).clicked() {
                    self.skin = skin;
                }
            }
        });

        ui.add_space(8.0);

        match self.skin {
            NowPlayingSkin::LiquidGlass => {
                self.show_liquid_glass_skin(ui, current_track, player, position_ms, duration_ms, is_playing);
            }
            NowPlayingSkin::ExpressiveFreeshapes => {
                self.show_freeshapes_skin(ui, current_track, player, position_ms, duration_ms, is_playing);
            }
            NowPlayingSkin::NormalSquareBig => {
                self.show_normal_square_big_skin(ui, current_track, player, lyrics, position_ms, duration_ms, is_playing);
            }
        }
    }

    // ------------------------------------------------------------------ Skin 1: Default Liquid Glass
    fn show_liquid_glass_skin(
        &mut self,
        ui: &mut Ui,
        current_track: Option<&TrackEntry>,
        player: &Player,
        position_ms: u64,
        duration_ms: u64,
        is_playing: bool,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(16.0);

            // Centered hero cover art
            let art_dim = (ui.available_width() * 0.40).clamp(180.0, 320.0);
            let art_size = egui::vec2(art_dim, art_dim);

            match current_track.and_then(|t| t.artwork_path.as_ref()) {
                Some(p) => {
                    let (rect, _) = ui.allocate_exact_size(art_size, egui::Sense::hover());
                    // Specular border backdrop
                    ui.painter().rect(
                        rect,
                        CornerRadius::same(18u8),
                        Color32::from_rgb(0x15, 0x1A, 0x24),
                        Stroke::new(1.5, Color32::from_white_alpha(45)),
                        egui::StrokeKind::Inside,
                    );
                    let img_rect = rect.shrink(3.0);
                    ui.put(
                        img_rect,
                        egui::Image::new(format!("file://{}", p.display()))
                            .fit_to_exact_size(img_rect.size())
                            .corner_radius(16.0),
                    );
                }
                None => {
                    let (rect, _) = ui.allocate_exact_size(art_size, egui::Sense::hover());
                    ui.painter().rect_filled(rect, 18.0, Color32::from_rgb(0x18, 0x1E, 0x2A));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "♪",
                        egui::FontId::proportional(64.0),
                        Color32::from_white_alpha(60),
                    );
                }
            }

            ui.add_space(20.0);

            // Title & Artist
            if let Some(t) = current_track {
                ui.label(egui::RichText::new(&t.title).strong().size(22.0));
                ui.label(egui::RichText::new(&t.artist).size(15.0).weak());
                ui.add_space(8.0);

                // Honest Audio Quality Pill
                ui.horizontal(|ui| {
                    ui.set_width(ui.available_width());
                    ui.centered_and_justified(|ui| {
                        render_audio_quality_badge(ui, &t.format, t.sample_rate);
                    });
                });
            } else {
                ui.label(egui::RichText::new("No Track Playing").size(20.0).weak());
                ui.label(egui::RichText::new("Choose a song from your library or queue").weak());
            }

            ui.add_space(24.0);

            // Transport Scrubber & Controls
            self.render_large_controls(ui, player, position_ms, duration_ms, is_playing);
        });
    }

    // ------------------------------------------------------------------ Skin 2: Material You Freeshapes
    fn show_freeshapes_skin(
        &mut self,
        ui: &mut Ui,
        current_track: Option<&TrackEntry>,
        player: &Player,
        position_ms: u64,
        duration_ms: u64,
        is_playing: bool,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(16.0);

            let art_dim = (ui.available_width() * 0.45).clamp(200.0, 340.0);
            let art_size = egui::vec2(art_dim, art_dim);
            let (rect, _) = ui.allocate_exact_size(art_size, egui::Sense::hover());

            // 1. Paint morphing freeshape backdrop with organic color
            let fill = Color32::from_rgba_premultiplied(0x16, 0x26, 0x32, 220);
            let stroke_col = crate::theme::ACCENT;
            paint_freeshape_backdrop(ui, rect, &self.freeshape_tracker, fill, stroke_col);

            // 2. Center circular cutout for cover art
            let inner_size = art_size * 0.65;
            let inner_rect = Rect::from_center_size(rect.center(), inner_size);
            if let Some(p) = current_track.and_then(|t| t.artwork_path.as_ref()) {
                ui.put(
                    inner_rect,
                    egui::Image::new(format!("file://{}", p.display()))
                        .fit_to_exact_size(inner_size)
                        .corner_radius(inner_size.x * 0.5),
                );
            } else {
                ui.painter().circle_filled(rect.center(), inner_size.x * 0.5, Color32::from_rgb(0x1E, 0x26, 0x36));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "♪",
                    egui::FontId::proportional(48.0),
                    Color32::from_white_alpha(70),
                );
            }

            ui.add_space(20.0);

            if let Some(t) = current_track {
                ui.label(egui::RichText::new(&t.title).strong().size(22.0));
                ui.label(egui::RichText::new(&t.artist).size(15.0).weak());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.centered_and_justified(|ui| {
                        render_audio_quality_badge(ui, &t.format, t.sample_rate);
                    });
                });
            }

            ui.add_space(20.0);
            self.render_large_controls(ui, player, position_ms, duration_ms, is_playing);
        });
    }

    // ------------------------------------------------------------------ Skin 3: Normal Square Big
    #[allow(clippy::too_many_arguments)]
    fn show_normal_square_big_skin(
        &mut self,
        ui: &mut Ui,
        current_track: Option<&TrackEntry>,
        player: &Player,
        lyrics: &LyricsSheet,
        position_ms: u64,
        duration_ms: u64,
        is_playing: bool,
    ) {
        ui.vertical(|ui| {
            ui.add_space(10.0);

            // Large edge-to-edge square album art
            let max_w = ui.available_width().min(ui.available_height() * 0.55);
            let art_size = egui::vec2(max_w, max_w);

            match current_track.and_then(|t| t.artwork_path.as_ref()) {
                Some(p) => {
                    ui.add(
                        egui::Image::new(format!("file://{}", p.display()))
                            .fit_to_exact_size(art_size)
                            .corner_radius(16.0),
                    );
                }
                None => {
                    let (rect, _) = ui.allocate_exact_size(art_size, egui::Sense::hover());
                    ui.painter().rect_filled(rect, 16.0, Color32::from_rgb(0x18, 0x1E, 0x28));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "♪",
                        egui::FontId::proportional(72.0),
                        Color32::from_white_alpha(60),
                    );
                }
            }

            ui.add_space(12.0);

            // Inline Timed Lyrics Snippet in cream-white styling (per user request!)
            if let Some(snippet) = lyrics.current_snippet(position_ms) {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("💬 \"{}\"", snippet))
                            .color(Color32::from_rgb(0xFF, 0xFD, 0xD0)) // Cream white #FFFDD0
                            .strong()
                            .size(15.0),
                    );
                });
                ui.add_space(6.0);
            }

            // Left-aligned title and artist
            if let Some(t) = current_track {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&t.title).strong().size(20.0));
                    render_audio_quality_badge(ui, &t.format, t.sample_rate);
                });
                ui.label(egui::RichText::new(&t.artist).size(14.0).weak());
            }

            ui.add_space(14.0);
            self.render_large_controls(ui, player, position_ms, duration_ms, is_playing);
        });
    }

    fn render_large_controls(
        &self,
        ui: &mut Ui,
        player: &Player,
        position_ms: u64,
        duration_ms: u64,
        is_playing: bool,
    ) {
        // Scrubber slider
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(fmt_ms(position_ms)).monospace());
            let max = duration_ms.max(1) as f64;
            let mut val = position_ms.min(duration_ms) as f64;
            let width = (ui.available_width() - 70.0).max(140.0);
            ui.spacing_mut().slider_width = width;
            let resp = ui.add_enabled(
                duration_ms > 0,
                egui::Slider::new(&mut val, 0.0..=max).show_value(false),
            );
            if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                player.seek_ms(val as u64);
            }
            ui.label(egui::RichText::new(fmt_ms(duration_ms)).monospace());
        });

        ui.add_space(8.0);

        // Big Playback Buttons
        ui.horizontal(|ui| {
            ui.set_width(ui.available_width());
            ui.centered_and_justified(|ui| {
                ui.horizontal(|ui| {
                    let play_label = if is_playing { "⏸ Pause" } else { "▶ Play" };
                    if ui.button(egui::RichText::new(play_label).size(16.0).strong()).clicked() {
                        if is_playing {
                            player.pause();
                        } else {
                            player.play();
                        }
                    }
                });
            });
        });
    }
}
