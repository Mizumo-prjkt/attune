//! Attune egui application shell: library list, queue and transport bar.
//!
//! Barebones milestone scope (ADR-064): scan → browse/search → play/pause/seek/
//! skip with linear auto-advance. Smart Shuffle transitions and the full
//! Liquid Material Glass skin are ported in later milestones.

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use eframe::egui;

use crate::config::{self, MusicDirSource};
use crate::library::{self, fmt_ms, ScanEvent, TrackEntry};
use crate::player::{LoadState, Player};

const STORAGE_KEY_MUSIC_DIR: &str = "attune.music_dir";
const STORAGE_KEY_VOLUME_DB: &str = "attune.volume_db";
/// UI volume slider range in dB (attenuation only; boosting is left to the FX rack).
const VOLUME_DB_RANGE: std::ops::RangeInclusive<f32> = -40.0..=0.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    fn next(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        }
    }
    fn label(self) -> &'static str {
        match self {
            RepeatMode::Off => "Repeat: Off",
            RepeatMode::All => "Repeat: All",
            RepeatMode::One => "Repeat: One",
        }
    }
}

pub struct AttuneApp {
    player: Player,
    music_dir: Option<PathBuf>,
    music_dir_source: MusicDirSource,
    tracks: Vec<TrackEntry>,
    scan_rx: Option<Receiver<ScanEvent>>,
    scan_expected: usize,
    search: String,
    /// Playback queue: indices into `tracks`. Starts empty on fresh launch (ADR-057 parity).
    queue: Vec<usize>,
    queue_pos: Option<usize>,
    repeat: RepeatMode,
    volume_db: f32,
    /// Seek drag value while the user is scrubbing (ms).
    scrub_ms: Option<f64>,
    /// Guards auto-advance so a finished deck only triggers one skip.
    advance_armed: bool,
    status: Option<String>,
}

impl AttuneApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        crate::theme::apply(&cc.egui_ctx);
        crate::fonts::install_fallback_fonts(&cc.egui_ctx);

        let persisted_dir = cc
            .storage
            .and_then(|s| eframe::get_value::<String>(s, STORAGE_KEY_MUSIC_DIR))
            .map(PathBuf::from);
        let volume_db = cc
            .storage
            .and_then(|s| eframe::get_value::<f32>(s, STORAGE_KEY_VOLUME_DB))
            .unwrap_or(0.0)
            .clamp(*VOLUME_DB_RANGE.start(), *VOLUME_DB_RANGE.end());

        let (music_dir, music_dir_source) = config::resolve_music_dir(persisted_dir);
        let player = Player::new();
        player.set_gain_db(volume_db);

        let mut app = Self {
            player,
            music_dir,
            music_dir_source,
            tracks: Vec::new(),
            scan_rx: None,
            scan_expected: 0,
            search: String::new(),
            queue: Vec::new(),
            queue_pos: None,
            repeat: RepeatMode::Off,
            volume_db,
            scrub_ms: None,
            advance_armed: false,
            status: None,
        };
        app.start_scan(&cc.egui_ctx);
        app
    }

    fn start_scan(&mut self, ctx: &egui::Context) {
        let Some(dir) = self.music_dir.clone() else {
            self.status = Some("No music folder set — choose one to build your library.".into());
            return;
        };
        self.tracks.clear();
        self.queue.clear();
        self.queue_pos = None;
        self.scan_expected = 0;
        self.status = None;
        self.scan_rx = Some(library::spawn_scan(dir, ctx.clone()));
    }

    fn pick_music_dir(&mut self, ctx: &egui::Context) {
        let mut dialog = rfd::FileDialog::new().set_title("Choose your music folder");
        if let Some(dir) = &self.music_dir {
            dialog = dialog.set_directory(dir);
        }
        if let Some(dir) = dialog.pick_folder() {
            self.music_dir = Some(dir);
            self.music_dir_source = MusicDirSource::Persisted;
            self.start_scan(ctx);
        }
    }

    fn drain_scan(&mut self) {
        let Some(rx) = &self.scan_rx else { return };
        let mut finished = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                ScanEvent::Discovered(n) => self.scan_expected = n,
                ScanEvent::Track(t) => self.tracks.push(*t),
                ScanEvent::Finished => finished = true,
            }
        }
        if finished {
            self.scan_rx = None;
            self.tracks
                .sort_by(|a, b| (&a.artist, &a.album, &a.title).cmp(&(&b.artist, &b.album, &b.title)));
            if self.tracks.is_empty() {
                self.status = Some("No supported audio files found in this folder.".into());
            }
        }
    }

    fn current_track(&self) -> Option<&TrackEntry> {
        self.queue_pos
            .and_then(|p| self.queue.get(p))
            .and_then(|&i| self.tracks.get(i))
    }

    fn play_queue_pos(&mut self, pos: usize, ctx: &egui::Context) {
        let Some(&idx) = self.queue.get(pos) else { return };
        let Some(track) = self.tracks.get(idx) else { return };
        self.queue_pos = Some(pos);
        self.advance_armed = false;
        self.scrub_ms = None;
        self.player.play_file(track.path.clone(), ctx.clone());
    }

    /// Plays a library track immediately, appending it to the queue if absent.
    fn play_track_now(&mut self, idx: usize, ctx: &egui::Context) {
        let pos = match self.queue.iter().position(|&q| q == idx) {
            Some(p) => p,
            None => {
                let insert_at = self.queue_pos.map(|p| p + 1).unwrap_or(self.queue.len());
                self.queue.insert(insert_at, idx);
                insert_at
            }
        };
        self.play_queue_pos(pos, ctx);
    }

    fn enqueue(&mut self, idx: usize) {
        self.queue.push(idx);
    }

    fn remove_from_queue(&mut self, pos: usize) {
        if pos >= self.queue.len() {
            return;
        }
        self.queue.remove(pos);
        self.queue_pos = match self.queue_pos {
            Some(cur) if cur == pos => None,
            Some(cur) if cur > pos => Some(cur - 1),
            other => other,
        };
    }

    fn next(&mut self, ctx: &egui::Context, from_auto: bool) {
        if self.queue.is_empty() {
            return;
        }
        if from_auto && self.repeat == RepeatMode::One {
            if let Some(p) = self.queue_pos {
                self.play_queue_pos(p, ctx);
            }
            return;
        }
        let next = self.queue_pos.map(|p| p + 1).unwrap_or(0);
        if next < self.queue.len() {
            self.play_queue_pos(next, ctx);
        } else if self.repeat == RepeatMode::All {
            self.play_queue_pos(0, ctx);
        } else if from_auto {
            self.player.pause();
        }
    }

    fn previous(&mut self, ctx: &egui::Context, position_ms: u64) {
        // Standard player behaviour: restart current track if >3s in.
        const RESTART_THRESHOLD_MS: u64 = 3_000;
        if position_ms > RESTART_THRESHOLD_MS {
            self.player.seek_ms(0);
            return;
        }
        if let Some(p) = self.queue_pos {
            if p > 0 {
                self.play_queue_pos(p - 1, ctx);
            } else {
                self.player.seek_ms(0);
            }
        }
    }

    // ------------------------------------------------------------------ UI

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("Attune").strong().color(crate::theme::ACCENT));
                ui.label(egui::RichText::new(format!("v{}", config::app_version())).weak());
                ui.separator();
                let dir_text = self
                    .music_dir
                    .as_ref()
                    .map(|d| d.display().to_string())
                    .unwrap_or_else(|| "No music folder".into());
                ui.label(egui::RichText::new(dir_text).monospace())
                    .on_hover_text(format!("Source: {}", self.music_dir_source.label()));
                if ui.button("Choose Folder…").clicked() {
                    self.pick_music_dir(ctx);
                }
                let rescan = ui.add_enabled(
                    self.music_dir.is_some() && self.scan_rx.is_none(),
                    egui::Button::new("Rescan"),
                );
                if rescan.clicked() {
                    self.start_scan(ctx);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search title, artist, album")
                            .desired_width(240.0),
                    );
                    if resp.changed() {
                        ctx.request_repaint();
                    }
                });
            });
            if !self.player.has_hardware_output() {
                ui.colored_label(
                    egui::Color32::from_rgb(0xE0, 0x7A, 0x5F),
                    format!(
                        "⚠ No audio output device — running headless ({})",
                        self.player.driver_error().unwrap_or("unknown error")
                    ),
                );
            }
            ui.add_space(4.0);
        });
    }

    fn queue_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("queue_panel")
            .resizable(true)
            .default_width(300.0)
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.strong("Up Next");
                    ui.label(egui::RichText::new(format!("{} tracks", self.queue.len())).weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(!self.queue.is_empty(), egui::Button::new("Clear")).clicked() {
                            self.queue.clear();
                            self.queue_pos = None;
                        }
                    });
                });
                ui.separator();
                if self.queue.is_empty() {
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Queue is empty.\nDouble-click a track to play,\nor use “+” to add it.").weak());
                    return;
                }
                let mut play_pos = None;
                let mut remove_pos = None;
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for (pos, &idx) in self.queue.iter().enumerate() {
                        let Some(t) = self.tracks.get(idx) else { continue };
                        let is_current = self.queue_pos == Some(pos);
                        ui.horizontal(|ui| {
                            let label = egui::RichText::new(format!("{}. {}", pos + 1, t.title));
                            let label = if is_current { label.color(crate::theme::ACCENT).strong() } else { label };
                            let resp = ui
                                .add(egui::Label::new(label).truncate().sense(egui::Sense::click()))
                                .on_hover_text(format!("{} — {}", t.artist, t.album));
                            if resp.double_clicked() {
                                play_pos = Some(pos);
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("✕").on_hover_text("Remove from queue").clicked() {
                                    remove_pos = Some(pos);
                                }
                            });
                        });
                    }
                });
                if let Some(p) = play_pos {
                    self.play_queue_pos(p, ctx);
                }
                if let Some(p) = remove_pos {
                    self.remove_from_queue(p);
                }
            });
    }

    fn transport_bar(&mut self, ctx: &egui::Context) {
        let snap = self.player.snapshot();
        let (position_ms, duration_ms, is_playing, finished, sample_rate, underruns) = match &snap {
            Some(s) => {
                let d = if s.active_deck_str == "DeckB" { &s.deck_b } else { &s.deck_a };
                (d.position_ms, d.duration_ms, s.is_playing, d.is_audible_finished, s.sample_rate, s.underrun_count)
            }
            None => (0, 0, false, false, 0, 0),
        };
        let load_state = self.player.load_state();

        // Linear auto-advance: arm while audibly playing, fire once when finished.
        if load_state == LoadState::Ready && duration_ms > 0 {
            if is_playing && !finished {
                self.advance_armed = true;
            } else if finished && self.advance_armed {
                self.advance_armed = false;
                self.next(ctx, true);
            }
        }

        egui::TopBottomPanel::bottom("transport").min_height(96.0).show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // Artwork
                let art_size = egui::vec2(64.0, 64.0);
                match self.current_track().and_then(|t| t.artwork_path.clone()) {
                    Some(p) => {
                        ui.add(
                            egui::Image::new(format!("file://{}", p.display()))
                                .fit_to_exact_size(art_size)
                                .corner_radius(8.0),
                        );
                    }
                    None => {
                        let (rect, _) = ui.allocate_exact_size(art_size, egui::Sense::hover());
                        ui.painter().rect_filled(rect, 8.0, ui.visuals().faint_bg_color);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "♪",
                            egui::FontId::proportional(28.0),
                            ui.visuals().weak_text_color(),
                        );
                    }
                }

                ui.vertical(|ui| {
                    ui.set_width(220.0);
                    match self.current_track() {
                        Some(t) => {
                            ui.add(egui::Label::new(egui::RichText::new(&t.title).strong().size(15.0)).truncate());
                            ui.add(egui::Label::new(egui::RichText::new(&t.artist).weak()).truncate());
                            let mut meta = vec![t.format.clone()];
                            if t.sample_rate > 0 {
                                meta.push(format!("{:.1} kHz", t.sample_rate as f32 / 1000.0));
                            }
                            if t.bit_depth > 0 {
                                meta.push(format!("{}-bit", t.bit_depth));
                            }
                            if let Some(bpm) = t.bpm {
                                meta.push(format!("{:.1} BPM", bpm));
                            }
                            ui.label(egui::RichText::new(meta.join(" • ")).small().weak());
                        }
                        None => {
                            ui.label(egui::RichText::new("Nothing playing").weak());
                        }
                    }
                    match &load_state {
                        LoadState::Decoding(_) => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(egui::RichText::new("Decoding…").small());
                            });
                        }
                        LoadState::Failed(e) => {
                            ui.colored_label(egui::Color32::LIGHT_RED, format!("Decode failed: {e}"));
                        }
                        _ => {}
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let btn = |s: &str| egui::Button::new(egui::RichText::new(s).size(18.0)).min_size(egui::vec2(36.0, 32.0));
                        if ui.add(btn("⏮")).on_hover_text("Previous").clicked() {
                            self.previous(ctx, position_ms);
                        }
                        let play_label = if is_playing { "⏸" } else { "▶" };
                        if ui.add(btn(play_label)).clicked() {
                            if is_playing {
                                self.player.pause();
                            } else if self.queue_pos.is_none() && !self.queue.is_empty() {
                                self.play_queue_pos(0, ctx);
                            } else {
                                self.player.play();
                            }
                        }
                        if ui.add(btn("⏭")).on_hover_text("Next").clicked() {
                            self.next(ctx, false);
                        }
                        if ui.button(self.repeat.label()).clicked() {
                            self.repeat = self.repeat.next();
                        }
                        ui.separator();
                        ui.label("Vol");
                        let vol = ui.add(
                            egui::Slider::new(&mut self.volume_db, VOLUME_DB_RANGE)
                                .suffix(" dB")
                                .fixed_decimals(0),
                        );
                        if vol.changed() {
                            self.player.set_gain_db(self.volume_db);
                        }
                    });

                    ui.horizontal(|ui| {
                        let shown = self.scrub_ms.unwrap_or(position_ms as f64);
                        ui.label(egui::RichText::new(fmt_ms(shown as u64)).monospace());
                        let max = duration_ms.max(1) as f64;
                        let mut val = shown.min(max);
                        let width = (ui.available_width() - 60.0).max(120.0);
                        ui.spacing_mut().slider_width = width;
                        let resp = ui.add_enabled(
                            duration_ms > 0,
                            egui::Slider::new(&mut val, 0.0..=max).show_value(false),
                        );
                        if resp.dragged() || resp.changed() {
                            self.scrub_ms = Some(val);
                        }
                        if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                            self.player.seek_ms(val as u64);
                            self.scrub_ms = None;
                        }
                        ui.label(egui::RichText::new(fmt_ms(duration_ms)).monospace());
                    });
                });
            });
            ui.horizontal(|ui| {
                let hw = if self.player.has_hardware_output() { "hardware" } else { "headless" };
                ui.label(
                    egui::RichText::new(format!("Output: {hw} @ {sample_rate} Hz • underruns: {underruns}"))
                        .small()
                        .weak(),
                );
            });
            ui.add_space(4.0);
        });

        if is_playing || matches!(load_state, LoadState::Decoding(_)) {
            // Keep the playhead moving without burning CPU at uncapped FPS.
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn library_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Library");
                ui.label(egui::RichText::new(format!("{} tracks", self.tracks.len())).weak());
                if self.scan_rx.is_some() {
                    ui.spinner();
                    ui.label(format!("Scanning {}/{}", self.tracks.len(), self.scan_expected));
                }
            });
            if let Some(msg) = &self.status {
                ui.label(egui::RichText::new(msg).weak());
            }
            if self.music_dir.is_none() && ui.button("Choose Music Folder…").clicked() {
                self.pick_music_dir(ctx);
            }
            ui.separator();

            let q = self.search.trim().to_lowercase();
            let filtered: Vec<usize> = (0..self.tracks.len()).filter(|&i| self.tracks[i].matches(&q)).collect();
            let current_idx = self.queue_pos.and_then(|p| self.queue.get(p)).copied();

            let mut play_now = None;
            let mut enqueue = None;
            let row_height = 24.0;
            egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(
                ui,
                row_height,
                filtered.len(),
                |ui, range| {
                    for &idx in &filtered[range] {
                        let t = &self.tracks[idx];
                        ui.horizontal(|ui| {
                            ui.set_height(row_height);
                            if ui.small_button("+").on_hover_text("Add to queue").clicked() {
                                enqueue = Some(idx);
                            }
                            let title = egui::RichText::new(&t.title);
                            let title = if current_idx == Some(idx) { title.color(crate::theme::ACCENT).strong() } else { title };
                            // Left-aligned fixed-width columns (add_sized would centre the text).
                            let left = egui::Layout::left_to_right(egui::Align::Center);
                            let title_w = ui.available_width() * 0.45;
                            let resp = ui
                                .allocate_ui_with_layout(egui::vec2(title_w, row_height), left, |ui| {
                                    ui.set_width(title_w);
                                    ui.add(egui::Label::new(title).truncate().sense(egui::Sense::click()))
                                })
                                .inner;
                            let resp = resp.on_hover_text(t.detail_line());
                            if resp.double_clicked() {
                                play_now = Some(idx);
                            }
                            let artist_w = ui.available_width() * 0.6;
                            ui.allocate_ui_with_layout(egui::vec2(artist_w, row_height), left, |ui| {
                                ui.set_width(artist_w);
                                ui.add(egui::Label::new(egui::RichText::new(&t.artist).weak()).truncate());
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(fmt_ms(t.duration_ms)).monospace().weak());
                                if let Some(bpm) = t.bpm {
                                    ui.label(egui::RichText::new(format!("{bpm:.0}")).small().weak());
                                }
                            });
                        });
                    }
                },
            );
            if let Some(idx) = play_now {
                self.play_track_now(idx, ctx);
            }
            if let Some(idx) = enqueue {
                self.enqueue(idx);
            }
        });
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Space toggles play/pause only when no text field has focus.
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            match self.player.snapshot() {
                Some(s) if s.is_playing => self.player.pause(),
                _ => self.player.play(),
            }
        }
    }
}

impl eframe::App for AttuneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_scan();
        self.handle_shortcuts(ctx);
        self.top_bar(ctx);
        self.transport_bar(ctx);
        self.queue_panel(ctx);
        self.library_panel(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Only persist user-chosen folders; env/OS defaults are re-resolved each launch.
        if self.music_dir_source == MusicDirSource::Persisted {
            if let Some(d) = &self.music_dir {
                eframe::set_value(storage, STORAGE_KEY_MUSIC_DIR, &d.display().to_string());
            }
        }
        eframe::set_value(storage, STORAGE_KEY_VOLUME_DB, &self.volume_db);
    }
}
