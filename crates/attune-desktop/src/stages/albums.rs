//! Albums and Collections stage for Attune egui.
//!
//! Features:
//! - Dynamic library grouping by album and artist
//! - Responsive album grid with cover art thumbnails, track count, total duration
//! - Album Detail View with M3 Split Pill Button `[ ▶ Play Album | ▾ Add to Queue ]`
//! - Full tracklist with track durations and BPM/key tags

use std::collections::BTreeMap;
use std::path::PathBuf;
use eframe::egui::{self, Color32, Ui};

use crate::components::glass::liquid_glass_card;
use crate::library::{fmt_ms, TrackEntry};
use crate::stages::album_editor::AlbumEditorState;

#[derive(Debug, Clone)]
pub struct AlbumGroup {
    pub title: String,
    pub artist: String,
    pub track_indices: Vec<usize>,
    pub total_duration_ms: u64,
    pub artwork_path: Option<PathBuf>,
}

#[derive(Default)]
pub struct AlbumsStage {
    pub selected_album: Option<String>,
    pub editing_album: Option<AlbumEditorState>,
}

impl AlbumsStage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Groups tracks by `(album_title, artist)`.
    pub fn group_albums(tracks: &[TrackEntry]) -> Vec<AlbumGroup> {
        let mut map: BTreeMap<String, AlbumGroup> = BTreeMap::new();

        for (idx, t) in tracks.iter().enumerate() {
            let key = format!("{} — {}", t.album, t.artist);
            let entry = map.entry(key.clone()).or_insert_with(|| AlbumGroup {
                title: t.album.clone(),
                artist: t.artist.clone(),
                track_indices: Vec::new(),
                total_duration_ms: 0,
                artwork_path: t.artwork_path.clone(),
            });
            entry.track_indices.push(idx);
            entry.total_duration_ms += t.duration_ms;
            if entry.artwork_path.is_none() && t.artwork_path.is_some() {
                entry.artwork_path = t.artwork_path.clone();
            }
        }

        map.into_values().collect()
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        tracks: &mut [TrackEntry],
        on_play_track: &mut impl FnMut(usize),
        on_play_album: &mut impl FnMut(&[usize]),
        on_enqueue_album: &mut impl FnMut(&[usize]),
    ) {
        if let Some(mut editor) = self.editing_album.take() {
            let action = editor.show(ui, tracks);
            match action {
                crate::stages::album_editor::AlbumEditorAction::Back => {
                    self.editing_album = None;
                }
                crate::stages::album_editor::AlbumEditorAction::Saved(new_title) => {
                    self.selected_album = Some(new_title);
                    self.editing_album = None;
                }
                crate::stages::album_editor::AlbumEditorAction::None => {
                    self.editing_album = Some(editor);
                }
            }
            return;
        }

        let albums = Self::group_albums(tracks);

        if let Some(selected_title) = self.selected_album.clone() {
            // Detailed Album View
            if let Some(album) = albums.iter().find(|a| a.title == selected_title) {
                self.show_album_detail(ui, album, tracks, on_play_track, on_play_album, on_enqueue_album);
                return;
            } else {
                self.selected_album = None;
            }
        }

        // Albums Grid View
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Albums & Collections").strong().color(crate::theme::ACCENT));
            ui.label(egui::RichText::new(format!("{} albums", albums.len())).weak());
        });

        ui.separator();

        if albums.is_empty() {
            ui.add_space(24.0);
            ui.label(egui::RichText::new("No albums found in your music library.").weak());
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let card_w = 180.0;
                let available_w = ui.available_width();
                let cols = ((available_w / (card_w + 16.0)).floor() as usize).max(1);

                egui::Grid::new("albums_grid")
                    .spacing([16.0, 20.0])
                    .show(ui, |ui| {
                        for (i, album) in albums.iter().enumerate() {
                            ui.vertical(|ui| {
                                ui.set_width(card_w);

                                // Album cover thumbnail
                                let art_size = egui::vec2(card_w, card_w);
                                let resp = if let Some(p) = &album.artwork_path {
                                    let img = egui::Image::new(format!("file://{}", p.display()))
                                        .fit_to_exact_size(art_size)
                                        .corner_radius(12.0);
                                    ui.add(img.sense(egui::Sense::click()))
                                } else {
                                    let (rect, r) = ui.allocate_exact_size(art_size, egui::Sense::click());
                                    ui.painter().rect_filled(rect, 12.0, Color32::from_rgb(0x18, 0x1E, 0x2A));
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "💿",
                                        egui::FontId::proportional(42.0),
                                        Color32::from_white_alpha(60),
                                    );
                                    r
                                };

                                if resp.clicked() {
                                    self.selected_album = Some(album.title.clone());
                                }

                                ui.add_space(6.0);
                                ui.add(egui::Label::new(egui::RichText::new(&album.title).strong().size(14.0)).truncate());
                                ui.add(egui::Label::new(egui::RichText::new(&album.artist).weak().size(12.0)).truncate());
                                ui.label(
                                    egui::RichText::new(format!("{} tracks • {}", album.track_indices.len(), fmt_ms(album.total_duration_ms)))
                                        .small()
                                        .weak(),
                                );
                            });

                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
    }

    fn show_album_detail(
        &mut self,
        ui: &mut Ui,
        album: &AlbumGroup,
        tracks: &[TrackEntry],
        on_play_track: &mut impl FnMut(usize),
        on_play_album: &mut impl FnMut(&[usize]),
        on_enqueue_album: &mut impl FnMut(&[usize]),
    ) {
        if ui.button("← Back to Albums").clicked() {
            self.selected_album = None;
            return;
        }

        ui.add_space(8.0);

        // Header Banner with Artwork & M3 Split Pill
        liquid_glass_card(ui, 18.0, crate::theme::ACCENT, |ui| {
            ui.horizontal(|ui| {
                let art_size = egui::vec2(120.0, 120.0);
                if let Some(p) = &album.artwork_path {
                    ui.add(
                        egui::Image::new(format!("file://{}", p.display()))
                            .fit_to_exact_size(art_size)
                            .corner_radius(12.0),
                    );
                } else {
                    let (rect, _) = ui.allocate_exact_size(art_size, egui::Sense::hover());
                    ui.painter().rect_filled(rect, 12.0, Color32::from_rgb(0x18, 0x1E, 0x2A));
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "💿", egui::FontId::proportional(36.0), Color32::from_white_alpha(70));
                }

                ui.add_space(14.0);

                ui.vertical(|ui| {
                    ui.heading(egui::RichText::new(&album.title).size(22.0).strong());
                    ui.label(egui::RichText::new(&album.artist).size(16.0).weak());
                    ui.label(
                        egui::RichText::new(format!(
                            "Collection of {} tracks • Total Duration: {}",
                            album.track_indices.len(),
                            fmt_ms(album.total_duration_ms)
                        ))
                        .small()
                        .weak(),
                    );

                    ui.add_space(10.0);

                    // M3 Split Pill Button: [ ▶ Play Album | + Add to Queue ] + [ ✏ Edit Album & Group ]
                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new("▶ Play Album").strong().color(Color32::from_rgb(0x0A, 0x20, 0x18))).clicked() {
                            on_play_album(&album.track_indices);
                        }
                        if ui.button("+ Add to Queue").clicked() {
                            on_enqueue_album(&album.track_indices);
                        }
                        ui.add_space(8.0);
                        if ui.button("✏ Edit Album & Group").on_hover_text("Edit album metadata, splash art, and track grouping").clicked() {
                            self.editing_album = Some(AlbumEditorState::from_album_group(
                                &album.title,
                                &album.artist,
                                &album.track_indices,
                                tracks,
                                album.artwork_path.as_ref(),
                            ));
                        }
                    });
                });
            });
        });

        ui.add_space(14.0);
        ui.strong("Tracklist");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (order, &idx) in album.track_indices.iter().enumerate() {
                let Some(t) = tracks.get(idx) else { continue };

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{:02}.", order + 1)).monospace().weak());
                    let resp = ui.add_sized(
                        [ui.available_width() * 0.50, 24.0],
                        egui::Label::new(egui::RichText::new(&t.title).strong()).truncate().sense(egui::Sense::click()),
                    );
                    if resp.double_clicked() {
                        on_play_track(idx);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(fmt_ms(t.duration_ms)).monospace().weak());
                        if let Some(bpm) = t.bpm {
                            ui.label(egui::RichText::new(format!("{bpm:.0} BPM")).small().weak());
                        }
                        if let Some(k) = &t.key {
                            ui.label(egui::RichText::new(k).small().weak());
                        }
                    });
                });
                ui.separator();
            }
        });
    }
}
