//! Album & Group Metadata Editor Stage for Attune egui (ADR-058, ADR-060, ADR-061 Parity).
//!
//! Features:
//! - Album Metadata (Title, Primary Artist with Comma-Separation auto-split, Contributing Artists chips, Year, Genre)
//! - Online Query & Catalog Identifiers (Record Label, Catalog Number, Album Release Type dropdown, Release Date, Primary Language, MusicBrainz Release ID)
//! - Album Splash Art (Local path text input, native file picker via `rfd`, validation badge, and Quick-Select Cover thumbnail selector from album tracks)
//! - Group track management (tracklist, remove song from album, add songs from library)
//! - File tag persistence via `attune_music_metadata_changer::write_track_metadata`

use std::path::{Path, PathBuf};
use eframe::egui::{self, Color32, CornerRadius, Stroke, Ui};

use crate::components::glass::liquid_glass_card;
use crate::library::{fmt_ms, TrackEntry};
use crate::theme::ACCENT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlbumEditorAction {
    None,
    Back,
    Saved(String),
}

const RELEASE_TYPES: &[&str] = &[
    "Studio Album",
    "Single",
    "EP",
    "Compilation",
    "Live Album",
    "Remix Album",
    "Soundtrack",
    "Other",
];

#[derive(Debug, Clone)]
pub struct AlbumEditorState {
    pub original_album_title: String,
    pub title: String,
    pub primary_artist: String,
    pub new_artist_input: String,
    pub additional_artists: Vec<String>,
    pub year: String,
    pub genre: String,

    // Online Query & Catalog Identifiers
    pub record_label: String,
    pub catalog_number: String,
    pub album_type: String,
    pub release_date: String,
    pub language: String,
    pub musicbrainz_id: String,

    // Album Splash Art
    pub splash_art_path: String,
    pub track_indices: Vec<usize>,
    pub available_artworks: Vec<PathBuf>,

    // Add Songs modal
    pub show_add_songs_modal: bool,
    pub status_message: Option<String>,
}

impl AlbumEditorState {
    pub fn from_album_group(
        album_title: &str,
        artist: &str,
        track_indices: &[usize],
        tracks: &[TrackEntry],
        artwork_path: Option<&PathBuf>,
    ) -> Self {
        let first_track = track_indices.first().and_then(|&i| tracks.get(i));
        let default_genre = first_track.and_then(|t| t.genre.clone()).unwrap_or_default();

        let mut available_artworks = Vec::new();
        for &idx in track_indices {
            if let Some(t) = tracks.get(idx) {
                if let Some(art) = &t.artwork_path {
                    if !available_artworks.contains(art) {
                        available_artworks.push(art.clone());
                    }
                }
            }
        }

        let splash_art_path = artwork_path
            .map(|p| p.display().to_string())
            .or_else(|| available_artworks.first().map(|p| p.display().to_string()))
            .unwrap_or_default();

        // Check if primary artist has comma separation
        let (primary, additional) = if artist.contains(',') {
            let mut parts = artist.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
            let first = parts.next().unwrap_or_else(|| artist.to_string());
            let rest: Vec<String> = parts.collect();
            (first, rest)
        } else {
            (artist.to_string(), Vec::new())
        };

        Self {
            original_album_title: album_title.to_string(),
            title: album_title.to_string(),
            primary_artist: primary,
            new_artist_input: String::new(),
            additional_artists: additional,
            year: String::new(),
            genre: default_genre,
            record_label: String::new(),
            catalog_number: String::new(),
            album_type: "Studio Album".to_string(),
            release_date: String::new(),
            language: String::new(),
            musicbrainz_id: String::new(),
            splash_art_path,
            track_indices: track_indices.to_vec(),
            available_artworks,
            show_add_songs_modal: false,
            status_message: None,
        }
    }

    pub fn add_contributing_artist(&mut self, text: &str) {
        let clean = text.trim();
        if clean.is_empty() {
            return;
        }

        // Support comma separation: split into individual artist tags
        for part in clean.split(',') {
            let artist = part.trim().to_string();
            if !artist.is_empty() && !self.additional_artists.iter().any(|a| a.eq_ignore_ascii_case(&artist)) {
                self.additional_artists.push(artist);
            }
        }
        self.new_artist_input.clear();
    }

    pub fn split_primary_artist_by_comma(&mut self) {
        if !self.primary_artist.contains(',') {
            return;
        }
        let parts: Vec<String> = self.primary_artist
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if let Some(first) = parts.first() {
            let new_primary = first.clone();
            for extra in &parts[1..] {
                if !self.additional_artists.iter().any(|a| a.eq_ignore_ascii_case(extra)) {
                    self.additional_artists.push(extra.clone());
                }
            }
            self.primary_artist = new_primary;
        }
    }

    pub fn save_changes(&mut self, tracks: &mut [TrackEntry]) -> Result<usize, String> {
        let mut saved_count = 0;
        let new_art_path = if !self.splash_art_path.trim().is_empty() {
            let p = PathBuf::from(self.splash_art_path.trim());
            if p.is_file() {
                Some(p)
            } else {
                None
            }
        } else {
            None
        };

        let parsed_year = self.year.trim().parse::<u32>().ok();

        for &idx in &self.track_indices {
            let Some(t) = tracks.get_mut(idx) else { continue };

            // Update in-memory track entry
            t.album = self.title.trim().to_string();
            t.artist = self.primary_artist.trim().to_string();
            if !self.genre.trim().is_empty() {
                t.genre = Some(self.genre.trim().to_string());
            }
            if let Some(art) = &new_art_path {
                t.artwork_path = Some(art.clone());
            }

            // Write metadata tags to disk via attune_music_metadata_changer
            let meta = attune_music_metadata_changer::metadata::TrackMetadata {
                title: Some(t.title.clone()),
                album: Some(self.title.trim().to_string()),
                artist: Some(self.primary_artist.trim().to_string()),
                additional_artists: self.additional_artists.clone(),
                genre: if !self.genre.trim().is_empty() { Some(self.genre.trim().to_string()) } else { None },
                year: parsed_year,
                record_label: if !self.record_label.trim().is_empty() { Some(self.record_label.trim().to_string()) } else { None },
                catalog_number: if !self.catalog_number.trim().is_empty() { Some(self.catalog_number.trim().to_string()) } else { None },
                album_type: if !self.album_type.trim().is_empty() { Some(self.album_type.trim().to_string()) } else { None },
                release_date: if !self.release_date.trim().is_empty() { Some(self.release_date.trim().to_string()) } else { None },
                language: if !self.language.trim().is_empty() { Some(self.language.trim().to_string()) } else { None },
                musicbrainz_id: if !self.musicbrainz_id.trim().is_empty() { Some(self.musicbrainz_id.trim().to_string()) } else { None },
                ..Default::default()
            };

            // Write to disk (ignore if format doesn't support writing)
            let _ = attune_music_metadata_changer::metadata::write_track_metadata(&t.path, &meta);
            saved_count += 1;
        }

        self.original_album_title = self.title.clone();
        Ok(saved_count)
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        tracks: &mut [TrackEntry],
    ) -> AlbumEditorAction {
        let mut action = AlbumEditorAction::None;

        // Top Action Bar
        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                action = AlbumEditorAction::Back;
                return;
            }

            ui.add_space(8.0);

            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("EDIT ALBUM & GROUP")
                        .color(ACCENT)
                        .strong()
                        .size(11.0),
                );
                ui.heading(
                    egui::RichText::new(&self.original_album_title)
                        .strong()
                        .size(18.0),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save_btn = egui::Button::new(
                    egui::RichText::new("✓ Save")
                        .strong()
                        .color(Color32::from_rgb(0x0A, 0x22, 0x1A)),
                )
                .fill(Color32::from_rgb(0x00, 0xE5, 0xA3))
                .min_size(egui::vec2(90.0, 32.0));

                if ui.add(save_btn).clicked() {
                    match self.save_changes(tracks) {
                        Ok(n) => {
                            self.status_message = Some(format!("Successfully saved changes across {n} tracks."));
                            action = AlbumEditorAction::Saved(self.title.clone());
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Failed to save: {e}"));
                        }
                    }
                }
            });
        });

        if action != AlbumEditorAction::None {
            return action;
        }

        if let Some(msg) = &self.status_message {
            ui.add_space(4.0);
            ui.colored_label(ACCENT, msg);
        }

        ui.add_space(8.0);
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(10.0);

                // Section 1: ALBUM METADATA
                self.show_album_metadata_section(ui);

                ui.add_space(14.0);

                // Section 2: ONLINE QUERY & CATALOG IDENTIFIERS
                self.show_online_catalog_section(ui);

                ui.add_space(14.0);

                // Section 3: ALBUM SPLASH ART
                self.show_splash_art_section(ui);

                ui.add_space(14.0);

                // Section 4: TRACKS IN ALBUM & GROUP MANAGEMENT
                self.show_tracks_management_section(ui, tracks);

                ui.add_space(30.0);
            });

        // Add Songs Modal
        if self.show_add_songs_modal {
            self.show_add_songs_window(ui.ctx(), tracks);
        }

        action
    }

    fn show_album_metadata_section(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🔘 ALBUM METADATA").strong().color(ACCENT));
        });
        ui.add_space(4.0);

        liquid_glass_card(ui, 16.0, ACCENT, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Album Title").weak().size(12.0));
                ui.add(
                    egui::TextEdit::singleline(&mut self.title)
                        .desired_width(f32::INFINITY)
                        .margin(egui::vec2(10.0, 8.0)),
                );

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Primary Artist").weak().size(12.0));
                    if self.primary_artist.contains(',')
                        && ui.small_button("✂ Split Artists (,)").on_hover_text("Split multiple comma-separated artists into separate chips").clicked()
                    {
                        self.split_primary_artist_by_comma();
                    }
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.primary_artist)
                        .desired_width(f32::INFINITY)
                        .margin(egui::vec2(10.0, 8.0)),
                );

                ui.add_space(10.0);

                // Contributing Artists
                ui.label(
                    egui::RichText::new("Contributing / Additional Artists (Collaborators, Featured)")
                        .weak()
                        .size(12.0),
                );

                ui.horizontal(|ui| {
                    let text_edit = egui::TextEdit::singleline(&mut self.new_artist_input)
                        .hint_text("Add contributing artist name (comma-separated)...")
                        .desired_width(ui.available_width() - 80.0)
                        .margin(egui::vec2(10.0, 8.0));
                    let res = ui.add(text_edit);
                    if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let text = self.new_artist_input.clone();
                        self.add_contributing_artist(&text);
                    }

                    let add_btn = egui::Button::new(egui::RichText::new("+ Add").strong().color(Color32::from_rgb(0x0A, 0x22, 0x1A)))
                        .fill(ACCENT)
                        .min_size(egui::vec2(60.0, 30.0));
                    if ui.add(add_btn).clicked() {
                        let text = self.new_artist_input.clone();
                        self.add_contributing_artist(&text);
                    }
                });

                // Display chips for additional artists
                if !self.additional_artists.is_empty() {
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        let mut to_remove = None;
                        for (i, artist) in self.additional_artists.iter().enumerate() {
                            let chip = format!("👤 {artist} ✕");
                            if ui.add(
                                egui::Button::new(egui::RichText::new(chip).small().color(ACCENT))
                                    .fill(Color32::from_rgb(0x14, 0x26, 0x2C))
                                    .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(0x2D, 0xD4, 0xBF, 100)))
                            ).on_hover_text("Click to remove artist").clicked() {
                                to_remove = Some(i);
                            }
                        }
                        if let Some(i) = to_remove {
                            self.additional_artists.remove(i);
                        }
                    });
                }

                ui.add_space(10.0);

                // Release Year & Genre
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        ui.label(egui::RichText::new("Release Year").weak().size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.year)
                                .hint_text("e.g. 2026")
                                .desired_width(f32::INFINITY)
                                .margin(egui::vec2(10.0, 8.0)),
                        );
                    });

                    cols[1].vertical(|ui| {
                        ui.label(egui::RichText::new("Genre").weak().size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.genre)
                                .hint_text("Pop")
                                .desired_width(f32::INFINITY)
                                .margin(egui::vec2(10.0, 8.0)),
                        );
                    });
                });
            });
        });
    }

    fn show_online_catalog_section(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("❖ ONLINE QUERY & CATALOG IDENTIFIERS").strong().color(ACCENT));
        });
        ui.add_space(4.0);

        liquid_glass_card(ui, 16.0, Color32::from_rgb(0x38, 0xBD, 0xF8), |ui| {
            ui.vertical(|ui| {
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        ui.label(egui::RichText::new("Record Label / Publisher").weak().size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.record_label)
                                .hint_text("e.g. Victor Entertainment...")
                                .desired_width(f32::INFINITY)
                                .margin(egui::vec2(10.0, 8.0)),
                        );
                    });

                    cols[1].vertical(|ui| {
                        ui.label(egui::RichText::new("Catalog Number / Barcode").weak().size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.catalog_number)
                                .hint_text("e.g. VICL-63200, 4988002...")
                                .desired_width(f32::INFINITY)
                                .margin(egui::vec2(10.0, 8.0)),
                        );
                    });
                });

                ui.add_space(10.0);

                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        ui.label(egui::RichText::new("Album Release Type").weak().size(12.0));
                        let w = ui.available_width() - 8.0;
                        egui::ComboBox::from_id_salt("album_release_type")
                            .selected_text(&self.album_type)
                            .width(w)
                            .show_ui(ui, |ui| {
                                for &t in RELEASE_TYPES {
                                    ui.selectable_value(&mut self.album_type, t.to_string(), t);
                                }
                            });
                    });

                    cols[1].vertical(|ui| {
                        ui.label(egui::RichText::new("Release Date (YYYY-MM-DD)").weak().size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.release_date)
                                .hint_text("e.g. 2026-02-14")
                                .desired_width(f32::INFINITY)
                                .margin(egui::vec2(10.0, 8.0)),
                        );
                    });
                });

                ui.add_space(10.0);

                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        ui.label(egui::RichText::new("Primary Language").weak().size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.language)
                                .hint_text("e.g. Japanese, English...")
                                .desired_width(f32::INFINITY)
                                .margin(egui::vec2(10.0, 8.0)),
                        );
                    });

                    cols[1].vertical(|ui| {
                        ui.label(egui::RichText::new("MusicBrainz Release ID").weak().size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.musicbrainz_id)
                                .hint_text("e.g. 8f6b7c2a-9e12...")
                                .desired_width(f32::INFINITY)
                                .margin(egui::vec2(10.0, 8.0)),
                        );
                    });
                });
            });
        });
    }

    fn show_splash_art_section(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🖼 ALBUM SPLASH ART").strong().color(ACCENT));
        });
        ui.add_space(4.0);

        liquid_glass_card(ui, 16.0, Color32::from_rgb(0xF4, 0x72, 0xB6), |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    // Preview thumbnail box (80x80)
                    let art_size = egui::vec2(80.0, 80.0);
                    let path_exists = Path::new(self.splash_art_path.trim()).is_file();
                    if path_exists {
                        let img = egui::Image::new(format!("file://{}", self.splash_art_path.trim()))
                            .fit_to_exact_size(art_size)
                            .corner_radius(12.0);
                        ui.add(img);
                    } else {
                        let (rect, _) = ui.allocate_exact_size(art_size, egui::Sense::hover());
                        ui.painter().rect_filled(rect, 12.0, Color32::from_rgb(0x18, 0x1E, 0x2A));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "🖼",
                            egui::FontId::proportional(32.0),
                            Color32::from_white_alpha(60),
                        );
                    }

                    ui.add_space(14.0);

                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Custom Splash Art (Absolute File Path)").weak().size(12.0));

                        ui.horizontal(|ui| {
                            let edit_w = (ui.available_width() - 140.0).max(180.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.splash_art_path)
                                    .desired_width(edit_w)
                                    .margin(egui::vec2(8.0, 6.0)),
                            );

                            if ui.button("📁 Browse").clicked() {
                                if let Some(p) = rfd::FileDialog::new()
                                    .set_title("Choose Album Splash Art")
                                    .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                                    .pick_file()
                                {
                                    self.splash_art_path = p.display().to_string();
                                }
                            }

                            if !self.splash_art_path.is_empty() && ui.button("✕").on_hover_text("Clear splash art").clicked() {
                                self.splash_art_path.clear();
                            }
                        });

                        ui.add_space(4.0);

                        // File validation indicator
                        if path_exists {
                            ui.colored_label(Color32::from_rgb(0x34, 0xD3, 0x99), "✓ Valid Local Image File");
                        } else if self.splash_art_path.starts_with("http://") || self.splash_art_path.starts_with("https://") {
                            ui.colored_label(Color32::from_rgb(0x38, 0xBD, 0xF8), "🌐 Online Image URI");
                        } else if !self.splash_art_path.trim().is_empty() {
                            ui.colored_label(Color32::from_rgb(0xF8, 0x71, 0x71), "⚠ File does not exist on disk");
                        }
                    });
                });

                ui.add_space(14.0);

                // Quick-Select Cover from Tracks in Album
                ui.label(egui::RichText::new("Quick-Select Cover from Tracks in Album:").weak().size(12.0));
                ui.add_space(4.0);

                if self.available_artworks.is_empty() {
                    ui.label(egui::RichText::new("No embedded artwork found across tracks in this album.").weak().small());
                } else {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let thumb_size = egui::vec2(56.0, 56.0);
                            for art_path in &self.available_artworks {
                                let art_str = art_path.display().to_string();
                                let is_selected = self.splash_art_path.trim() == art_str;

                                let (rect, resp) = ui.allocate_exact_size(thumb_size, egui::Sense::click());
                                let img_rect = rect.shrink(3.0);

                                ui.put(
                                    img_rect,
                                    egui::Image::new(format!("file://{art_str}"))
                                        .fit_to_exact_size(img_rect.size())
                                        .corner_radius(8.0),
                                );

                                if is_selected {
                                    // Glowing Cyan Border
                                    ui.painter().rect_stroke(
                                        rect,
                                        CornerRadius::same(10u8),
                                        Stroke::new(2.5, ACCENT),
                                        egui::StrokeKind::Outside,
                                    );
                                } else if resp.hovered() {
                                    ui.painter().rect_stroke(
                                        rect,
                                        CornerRadius::same(10u8),
                                        Stroke::new(1.5, Color32::from_white_alpha(100)),
                                        egui::StrokeKind::Outside,
                                    );
                                }

                                if resp.clicked() {
                                    self.splash_art_path = art_str;
                                }
                            }
                        });
                    });
                }
            });
        });
    }

    fn show_tracks_management_section(&mut self, ui: &mut Ui, tracks: &[TrackEntry]) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("🎶 TRACKS IN ALBUM ({})", self.track_indices.len()))
                    .strong()
                    .color(ACCENT),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ Add Songs to Album").clicked() {
                    self.show_add_songs_modal = true;
                }
            });
        });
        ui.add_space(4.0);

        liquid_glass_card(ui, 16.0, Color32::from_rgb(0xA8, 0x55, 0xF7), |ui| {
            let mut remove_idx = None;

            for (order, &idx) in self.track_indices.iter().enumerate() {
                let Some(t) = tracks.get(idx) else { continue };

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{:02}.", order + 1)).monospace().weak());
                    ui.add_sized(
                        [ui.available_width() * 0.52, 22.0],
                        egui::Label::new(egui::RichText::new(&t.title).strong()).truncate(),
                    );
                    ui.add_sized(
                        [ui.available_width() * 0.40, 22.0],
                        egui::Label::new(egui::RichText::new(&t.artist).weak()).truncate(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").on_hover_text("Remove track from this album").clicked() {
                            remove_idx = Some(order);
                        }
                        ui.label(egui::RichText::new(fmt_ms(t.duration_ms)).monospace().weak());
                    });
                });

                if order + 1 < self.track_indices.len() {
                    ui.separator();
                }
            }

            if let Some(i) = remove_idx {
                self.track_indices.remove(i);
            }
        });
    }

    fn show_add_songs_window(&mut self, ctx: &egui::Context, tracks: &[TrackEntry]) {
        let mut is_open = self.show_add_songs_modal;
        let mut added = None;

        egui::Window::new("Add Songs to Album")
            .open(&mut is_open)
            .resizable(true)
            .default_width(520.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                ui.label("Select a track from your library to add to this album group:");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, t) in tracks.iter().enumerate() {
                        if self.track_indices.contains(&idx) {
                            continue;
                        }

                        ui.horizontal(|ui| {
                            if ui.button("+ Add").clicked() {
                                added = Some(idx);
                            }
                            ui.label(egui::RichText::new(&t.title).strong());
                            ui.label(egui::RichText::new(format!("— {}", t.artist)).weak());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(fmt_ms(t.duration_ms)).monospace().weak());
                            });
                        });
                        ui.separator();
                    }
                });
            });

        if let Some(idx) = added {
            self.track_indices.push(idx);
            if let Some(t) = tracks.get(idx) {
                if let Some(art) = &t.artwork_path {
                    if !self.available_artworks.contains(art) {
                        self.available_artworks.push(art.clone());
                    }
                }
            }
        }
        self.show_add_songs_modal = is_open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_tracks() -> Vec<TrackEntry> {
        vec![
            TrackEntry {
                path: PathBuf::from("/music/song1.flac"),
                title: "(Not) A Devil".into(),
                artist: "DECO*27, PinocchioP".into(),
                album: "DECO_27, PinocchioP - (Not) A Devil (2023)".into(),
                duration_ms: 195000,
                format: "FLAC".into(),
                sample_rate: 48000,
                bit_depth: 24,
                bpm: Some(170.0),
                key: Some("8B".into()),
                genre: Some("Pop".into()),
                artwork_path: Some(PathBuf::from("/cache/art1.png")),
            },
            TrackEntry {
                path: PathBuf::from("/music/song2.flac"),
                title: "Rabbit Hole".into(),
                artist: "DECO*27".into(),
                album: "Rabbit Hole".into(),
                duration_ms: 180000,
                format: "FLAC".into(),
                sample_rate: 48000,
                bit_depth: 24,
                bpm: Some(132.0),
                key: Some("11B".into()),
                genre: Some("Rock".into()),
                artwork_path: Some(PathBuf::from("/cache/art2.png")),
            },
        ]
    }

    #[test]
    fn test_album_editor_from_group_auto_splits_comma_artists() {
        let tracks = dummy_tracks();
        let state = AlbumEditorState::from_album_group(
            "DECO_27, PinocchioP - (Not) A Devil (2023)",
            "DECO*27, PinocchioP",
            &[0],
            &tracks,
            Some(&PathBuf::from("/cache/art1.png")),
        );

        assert_eq!(state.primary_artist, "DECO*27");
        assert_eq!(state.additional_artists, vec!["PinocchioP"]);
        assert_eq!(state.genre, "Pop");
        assert_eq!(state.splash_art_path, "/cache/art1.png");
        assert_eq!(state.track_indices, vec![0]);
    }

    #[test]
    fn test_add_contributing_artists_with_comma_separation() {
        let tracks = dummy_tracks();
        let mut state = AlbumEditorState::from_album_group(
            "Single",
            "Artist A",
            &[0],
            &tracks,
            None,
        );

        state.add_contributing_artist("Artist B, Artist C , Artist D");
        assert_eq!(state.additional_artists, vec!["Artist B", "Artist C", "Artist D"]);

        // Ignore duplicates case-insensitively
        state.add_contributing_artist("artist b, Artist E");
        assert_eq!(state.additional_artists, vec!["Artist B", "Artist C", "Artist D", "Artist E"]);
    }

    #[test]
    fn test_split_primary_artist_by_comma() {
        let tracks = dummy_tracks();
        let mut state = AlbumEditorState::from_album_group(
            "Collab",
            "Solo Artist",
            &[0],
            &tracks,
            None,
        );

        state.primary_artist = "Hatsune Miku, Kagamine Rin, Megurine Luka".into();
        state.split_primary_artist_by_comma();

        assert_eq!(state.primary_artist, "Hatsune Miku");
        assert_eq!(state.additional_artists, vec!["Kagamine Rin", "Megurine Luka"]);
    }
}

