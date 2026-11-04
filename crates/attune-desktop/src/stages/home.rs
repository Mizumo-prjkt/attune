//! Home Stage — Pure Desktop Experience (ADR-067).
//!
//! Features:
//! - Personalized greeting ("Good morning / afternoon / evening, <user>")
//! - Hero Now Playing Liquid Glass Card (Cover art, title, artist, audio quality badge, playback controls, and live transition preview)
//! - "HARMONIC MATCHES & RECENTLY PLAYED" compact grid tiles with cover art, harmonic keys, and instant play action
//! - Quick action shortcuts (Start Smart Mix, View Library, Equalizer)

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Stroke, Ui};

use crate::components::glass::{liquid_glass_card, render_audio_quality_badge};
use crate::library::{fmt_ms, TrackEntry};
use crate::player::Player;
use crate::theme::ACCENT;

pub struct HomeStage {
    pub greeting_user: String,
}

impl Default for HomeStage {
    fn default() -> Self {
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "Listener".to_string());
        Self { greeting_user: user }
    }
}

impl HomeStage {
    pub fn new() -> Self {
        Self::default()
    }

    fn greeting_text(&self) -> String {
        // Estimate hour of day from system time
        let now = std::time::SystemTime::now();
        let secs = now.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        // Approximation of UTC hour; local hour can be estimated
        let hour = ((secs / 3600) % 24) as u32;
        let prefix = if (5..12).contains(&hour) {
            "Good morning"
        } else if (12..18).contains(&hour) {
            "Good afternoon"
        } else {
            "Good evening"
        };
        format!("{prefix}, {}", self.greeting_user)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tracks: &[TrackEntry],
        current_track: Option<&TrackEntry>,
        is_playing: bool,
        player: &Player,
        on_play_track: &mut impl FnMut(usize),
        on_toggle_play: &mut impl FnMut(),
        on_skip_next: &mut impl FnMut(),
        on_open_now_playing: &mut impl FnMut(),
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);

                // 1. Personalized Desktop Greeting Banner
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(
                            egui::RichText::new(self.greeting_text())
                                .strong()
                                .size(26.0)
                                .color(Color32::from_white_alpha(240)),
                        );
                        let subtitle = if tracks.is_empty() {
                            "No music files loaded. Choose a folder in the sidebar to populate your library.".to_string()
                        } else {
                            format!("{} songs available • DJ-grade harmonic transition engine ready", tracks.len())
                        };
                        ui.label(egui::RichText::new(subtitle).weak().size(13.0));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⚡ Start Smart Flow").clicked() && !tracks.is_empty() {
                            on_play_track(0);
                        }
                    });
                });

                ui.add_space(14.0);

                // 2. Hero Now Playing Card (Liquid Glass with ambient lighting)
                self.show_hero_now_playing(
                    ui,
                    current_track,
                    is_playing,
                    player,
                    on_toggle_play,
                    on_skip_next,
                    on_open_now_playing,
                );

                ui.add_space(20.0);

                // 3. Section: HARMONIC MATCHES & RECENTLY PLAYED
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("HARMONIC MATCHES & RECENTLY PLAYED")
                            .strong()
                            .size(13.0)
                            .color(Color32::from_white_alpha(180)),
                    );
                });
                ui.add_space(6.0);

                if tracks.is_empty() {
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("Add music tracks to explore harmonic pairings and recommendations.").weak());
                } else {
                    self.show_harmonic_matches_grid(ui, tracks, current_track, on_play_track);
                }

                ui.add_space(40.0);
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn show_hero_now_playing(
        &mut self,
        ui: &mut Ui,
        current_track: Option<&TrackEntry>,
        is_playing: bool,
        player: &Player,
        on_toggle_play: &mut impl FnMut(),
        on_skip_next: &mut impl FnMut(),
        on_open_now_playing: &mut impl FnMut(),
    ) {
        liquid_glass_card(ui, 18.0, ACCENT, |ui| {
            match current_track {
                Some(t) => {
                    ui.horizontal(|ui| {
                        // Hero Cover Art (140x140)
                        let art_size = egui::vec2(130.0, 130.0);
                        if let Some(p) = &t.artwork_path {
                            let resp = ui.add(
                                egui::Image::new(format!("file://{}", p.display()))
                                    .fit_to_exact_size(art_size)
                                    .corner_radius(14.0)
                                    .sense(egui::Sense::click()),
                            );
                            if resp.clicked() {
                                on_open_now_playing();
                            }
                        } else {
                            let (rect, resp) = ui.allocate_exact_size(art_size, egui::Sense::click());
                            ui.painter().rect_filled(rect, 14.0, Color32::from_rgb(0x14, 0x1A, 0x24));
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "♪",
                                egui::FontId::proportional(48.0),
                                Color32::from_white_alpha(50),
                            );
                            if resp.clicked() {
                                on_open_now_playing();
                            }
                        }

                        ui.add_space(16.0);

                        // Track metadata and hero controls
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("NOW PLAYING").strong().color(ACCENT).size(11.0));
                                if let Some(key) = &t.key {
                                    ui.label(
                                        egui::RichText::new(format!("• Key {key}"))
                                            .color(Color32::from_rgb(0x38, 0xBD, 0xF8))
                                            .small()
                                            .strong(),
                                    );
                                }
                                if let Some(bpm) = t.bpm {
                                    ui.label(
                                        egui::RichText::new(format!("• {bpm:.0} BPM"))
                                            .weak()
                                            .small(),
                                    );
                                }
                            });

                            ui.add_space(2.0);
                            ui.heading(egui::RichText::new(&t.title).strong().size(22.0));
                            ui.label(egui::RichText::new(&t.artist).size(15.0).weak());
                            ui.label(egui::RichText::new(&t.album).size(13.0).weak());

                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                render_audio_quality_badge(ui, &t.format, t.sample_rate);

                                ui.add_space(10.0);

                                // Quick Play/Pause and Skip buttons
                                let play_icon = if is_playing { "⏸ Pause" } else { "▶ Play" };
                                if ui.button(egui::RichText::new(play_icon).strong()).clicked() {
                                    on_toggle_play();
                                }
                                if ui.button("⏭ Next").clicked() {
                                    on_skip_next();
                                }
                                if ui.button("⛶ Expand").clicked() {
                                    on_open_now_playing();
                                }
                            });

                            // Transition Pill if transition is pending/active
                            if let Some(info) = player.pending_transition() {
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "⚡ Primed DJ Handoff: {} ({:.1}s, {}% match) → Upcoming: {}",
                                        info.archetype_name, info.duration_sec, info.confidence_pct, info.incoming_title
                                    ))
                                    .small()
                                    .color(ACCENT),
                                );
                            }
                        });
                    });
                }
                None => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.heading(egui::RichText::new("Ready to Mix").size(20.0).strong().color(ACCENT));
                        ui.label(egui::RichText::new("Select a track from the grid below or pick a folder from the sidebar.").weak());
                        ui.add_space(8.0);
                    });
                }
            }
        });
    }

    fn show_harmonic_matches_grid(
        &mut self,
        ui: &mut Ui,
        tracks: &[TrackEntry],
        current_track: Option<&TrackEntry>,
        on_play_track: &mut impl FnMut(usize),
    ) {
        // Collect candidate tracks: prioritize tracks with matching/adjacent Camelot keys or recently played
        let cur_key = current_track.and_then(|t| t.key.as_deref()).unwrap_or("");
        let mut candidates: Vec<(usize, &TrackEntry, u32)> = tracks
            .iter()
            .enumerate()
            .map(|(idx, t)| {
                let mut score = 50;
                if let Some(k) = &t.key {
                    if k == cur_key {
                        score += 40;
                    } else if !cur_key.is_empty() && (k.ends_with('A') == cur_key.ends_with('A')) {
                        score += 20;
                    }
                }
                (idx, t, score)
            })
            .collect();

        // Sort by score descending, up to 12 tiles
        candidates.sort_by_key(|b| std::cmp::Reverse(b.2));
        let display_candidates: Vec<_> = candidates.into_iter().take(9).collect();

        let card_w = ((ui.available_width() - 32.0) / 3.0).max(180.0);
        let card_h = 72.0;

        egui::Grid::new("harmonic_matches_grid")
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                for (col_idx, (track_idx, track, _score)) in display_candidates.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());

                    // Liquid Glass tile background
                    let bg_color = if resp.hovered() {
                        Color32::from_rgba_premultiplied(0x18, 0x24, 0x30, 220)
                    } else {
                        Color32::from_rgba_premultiplied(0x10, 0x16, 0x22, 180)
                    };
                    let stroke = if resp.hovered() {
                        Stroke::new(1.0, ACCENT)
                    } else {
                        Stroke::new(1.0, Color32::from_white_alpha(25))
                    };

                    ui.painter().rect(rect, CornerRadius::same(12u8), bg_color, stroke, egui::StrokeKind::Inside);

                    // Tile content: cover art on left, text on right
                    let thumb_rect = Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), egui::vec2(56.0, 56.0));
                    if let Some(p) = &track.artwork_path {
                        let img = egui::Image::new(format!("file://{}", p.display()))
                            .fit_to_exact_size(thumb_rect.size())
                            .corner_radius(8.0);
                        ui.put(thumb_rect, img);
                    } else {
                        ui.painter().rect_filled(thumb_rect, 8.0, Color32::from_rgb(0x18, 0x1E, 0x28));
                        ui.painter().text(
                            thumb_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "♪",
                            egui::FontId::proportional(22.0),
                            Color32::from_white_alpha(50),
                        );
                    }

                    // Text labels
                    let text_left = thumb_rect.right() + 10.0;
                    let text_rect = Rect::from_min_max(
                        Pos2::new(text_left, rect.top() + 10.0),
                        Pos2::new(rect.right() - 8.0, rect.bottom() - 10.0),
                    );

                    ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect), |ui| {
                        ui.vertical(|ui| {
                            ui.add(egui::Label::new(egui::RichText::new(&track.title).strong().size(13.0)).truncate());
                            ui.add(egui::Label::new(egui::RichText::new(&track.artist).weak().size(11.0)).truncate());
                            ui.horizontal(|ui| {
                                if let Some(k) = &track.key {
                                    ui.label(egui::RichText::new(k).small().color(Color32::from_rgb(0x38, 0xBD, 0xF8)));
                                }
                                if let Some(bpm) = track.bpm {
                                    ui.label(egui::RichText::new(format!("{bpm:.0}")).small().weak());
                                }
                                ui.label(egui::RichText::new(fmt_ms(track.duration_ms)).monospace().small().weak());
                            });
                        });
                    });

                    if resp.clicked() {
                        on_play_track(*track_idx);
                    }

                    if (col_idx + 1) % 3 == 0 {
                        ui.end_row();
                    }
                }
            });
    }
}
