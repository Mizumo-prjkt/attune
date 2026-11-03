//! Attune egui application shell: library, queue, Smart Shuffle transitions, and Master FX EQ.
//!
//! Features:
//! - Direct in-process `DualDeckMixer` + cpal `AudioOutputDriver` (no C-ABI hop)
//! - Smart Shuffle DJ transitions (harmonic Camelot match, BPM sync, recency protection)
//! - Live transition pill with progress and interactive candidate verdict modal
//! - 10-Band Graphic Equalizer, Attune Pure Bass+, Clarity & Vocal Air, Stereo Expander
//! - Drag & drop audio files and folders directly into the player
//! - Dynamic environment resolution (Mandate 4: zero hardcoded paths or versions)

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use eframe::egui::{self, Color32};

use crate::components::glass::render_audio_quality_badge;
use crate::components::lyrics_parser::{self, LyricsSheet};
use crate::config::{self, MusicDirSource};
use crate::library::{self, fmt_ms, ScanEvent, TrackEntry};
use crate::master_fx_ui::MasterFxUi;
use crate::player::{LoadState, Player};
use crate::smart_shuffle::{self, CandidateEvaluation, ShuffleMode, SmartShuffleHistory};
use crate::stages::about::AboutStage;
use crate::stages::albums::AlbumsStage;
use crate::stages::bubblegrid::BubblegridStage;
use crate::stages::lyrics::LyricsStage;
use crate::stages::now_playing::{NowPlayingSkin, NowPlayingStage};
use crate::theme::DynamicPalette;

const STORAGE_KEY_MUSIC_DIR: &str = "attune.music_dir";
const STORAGE_KEY_VOLUME_DB: &str = "attune.volume_db";
const STORAGE_KEY_SHUFFLE: &str = "attune.shuffle_mode";
const STORAGE_KEY_REPEAT: &str = "attune.repeat_mode";
const STORAGE_KEY_STAGE: &str = "attune.stage_view";
const STORAGE_KEY_NOW_PLAYING_SKIN: &str = "attune.now_playing_skin";
const VOLUME_DB_RANGE: std::ops::RangeInclusive<f32> = -40.0..=0.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StageView {
    NowPlaying,
    Library,
    Albums,
    Lyrics,
    Bubblegrid,
    About,
}

impl StageView {
    pub const ALL: [StageView; 6] = [
        StageView::NowPlaying,
        StageView::Library,
        StageView::Albums,
        StageView::Lyrics,
        StageView::Bubblegrid,
        StageView::About,
    ];

    pub fn label(self) -> &'static str {
        match self {
            StageView::NowPlaying => "▶ Now Playing",
            StageView::Library => "♫ Library",
            StageView::Albums => "💿 Albums",
            StageView::Lyrics => "💬 Lyrics",
            StageView::Bubblegrid => "🫧 Bubblegrid",
            StageView::About => "ℹ About",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Playback queue: indices into `tracks`.
    queue: Vec<usize>,
    queue_pos: Option<usize>,
    shuffle: ShuffleMode,
    repeat: RepeatMode,
    volume_db: f32,
    scrub_ms: Option<f64>,
    advance_armed: bool,
    status: Option<String>,
    // Smart Shuffle & DJ Transition State
    smart_history: SmartShuffleHistory,
    prep_transition_started: bool,
    last_verdict: Option<CandidateEvaluation>,
    show_verdict_modal: bool,
    was_in_transition: bool,
    // Master FX DSP Rack
    master_fx_ui: MasterFxUi,
    // Stages & Liquid Glass UI
    current_stage: StageView,
    now_playing_stage: NowPlayingStage,
    albums_stage: AlbumsStage,
    lyrics_stage: LyricsStage,
    bubblegrid_stage: BubblegridStage,
    current_lyrics: LyricsSheet,
    dynamic_palette: DynamicPalette,
    animation_time: f32,
    show_queue_panel: bool,
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
        let shuffle = cc
            .storage
            .and_then(|s| eframe::get_value::<ShuffleMode>(s, STORAGE_KEY_SHUFFLE))
            .unwrap_or(ShuffleMode::Off);
        let repeat = cc
            .storage
            .and_then(|s| eframe::get_value::<RepeatMode>(s, STORAGE_KEY_REPEAT))
            .unwrap_or(RepeatMode::Off);

        let current_stage = cc
            .storage
            .and_then(|s| eframe::get_value::<StageView>(s, STORAGE_KEY_STAGE))
            .unwrap_or(StageView::NowPlaying);
        let now_playing_skin = cc
            .storage
            .and_then(|s| eframe::get_value::<NowPlayingSkin>(s, STORAGE_KEY_NOW_PLAYING_SKIN))
            .unwrap_or(NowPlayingSkin::LiquidGlass);

        let (music_dir, music_dir_source) = config::resolve_music_dir(persisted_dir);
        let player = Player::new();
        player.set_gain_db(volume_db);

        let mut now_playing_stage = NowPlayingStage::new();
        now_playing_stage.skin = now_playing_skin;

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
            shuffle,
            repeat,
            volume_db,
            scrub_ms: None,
            advance_armed: false,
            status: None,
            smart_history: SmartShuffleHistory::new(25),
            prep_transition_started: false,
            last_verdict: None,
            show_verdict_modal: false,
            was_in_transition: false,
            master_fx_ui: MasterFxUi::new(),
            current_stage,
            now_playing_stage,
            albums_stage: AlbumsStage::new(),
            lyrics_stage: LyricsStage::new(),
            bubblegrid_stage: BubblegridStage::new(),
            current_lyrics: LyricsSheet::empty(),
            dynamic_palette: DynamicPalette::default(),
            animation_time: 0.0,
            show_queue_panel: true,
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

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }

        let mut added = 0;
        for file in dropped {
            let Some(path) = file.path else { continue };
            if path.is_dir() {
                self.music_dir = Some(path);
                self.music_dir_source = MusicDirSource::Persisted;
                self.start_scan(ctx);
                return;
            } else if path.is_file() {
                let is_audio = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e.to_ascii_lowercase().as_str(), "flac" | "mp3" | "wav" | "ogg" | "m4a" | "aac" | "aiff" | "aif"))
                    .unwrap_or(false);
                if is_audio && !self.tracks.iter().any(|t| t.path == path) {
                    let entry = TrackEntry::from_probe(&path);
                    self.tracks.push(entry);
                    added += 1;
                }
            }
        }
        if added > 0 {
            self.status = Some(format!("Added {added} dropped audio files to library."));
            ctx.request_repaint();
        }
    }

    fn current_track_idx(&self) -> Option<usize> {
        self.queue_pos.and_then(|p| self.queue.get(p)).copied()
    }

    fn current_track(&self) -> Option<&TrackEntry> {
        self.current_track_idx().and_then(|i| self.tracks.get(i))
    }

    fn play_queue_pos(&mut self, pos: usize, ctx: &egui::Context) {
        let Some(&idx) = self.queue.get(pos) else { return };
        let Some(track) = self.tracks.get(idx) else { return };
        self.queue_pos = Some(pos);
        self.advance_armed = false;
        self.scrub_ms = None;
        self.prep_transition_started = false;
        self.was_in_transition = false;
        self.smart_history.record_played(track.path.clone());

        // Dynamic Palette Extraction
        if let Some(art) = &track.artwork_path {
            if let Some(pal) = DynamicPalette::from_image(art) {
                self.dynamic_palette = pal;
            }
        }

        // Companion Lyrics Resolution
        self.current_lyrics = lyrics_parser::resolve_companion_lyrics(&track.path)
            .unwrap_or_else(LyricsSheet::empty);

        // Update Now Playing animations and shape
        self.now_playing_stage.on_track_changed(track);

        self.player.play_file(track.path.clone(), ctx.clone());
    }

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

    fn play_track_next(&mut self, idx: usize) {
        let insert_at = self.queue_pos.map(|p| p + 1).unwrap_or(self.queue.len());
        self.queue.insert(insert_at, idx);
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

    fn move_queue_item(&mut self, pos: usize, delta: isize) {
        let new_pos = (pos as isize + delta) as usize;
        if new_pos < self.queue.len() && pos < self.queue.len() {
            self.queue.swap(pos, new_pos);
            if self.queue_pos == Some(pos) {
                self.queue_pos = Some(new_pos);
            } else if self.queue_pos == Some(new_pos) {
                self.queue_pos = Some(pos);
            }
        }
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

        if self.shuffle == ShuffleMode::Smart {
            // Smart Shuffle next selection
            if let Some(current) = self.current_track() {
                let candidate_indices: Vec<usize> = self
                    .queue
                    .iter()
                    .copied()
                    .filter(|&i| Some(i) != self.current_track_idx())
                    .collect();

                let pool = if candidate_indices.is_empty() {
                    (0..self.tracks.len()).filter(|&i| Some(i) != self.current_track_idx()).collect()
                } else {
                    candidate_indices
                };

                let evals = smart_shuffle::evaluate_candidates(current, &pool, &self.tracks, &self.smart_history);
                if let Some(winner) = evals.first() {
                    let win_idx = winner.track_idx;
                    self.last_verdict = Some(winner.clone());
                    let pos = match self.queue.iter().position(|&q| q == win_idx) {
                        Some(p) => p,
                        None => {
                            let next_slot = self.queue_pos.map(|p| p + 1).unwrap_or(self.queue.len());
                            self.queue.insert(next_slot, win_idx);
                            next_slot
                        }
                    };
                    self.play_queue_pos(pos, ctx);
                    return;
                }
            }
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

    /// Handles automated DJ transition triggers & Deck B JIT preloading.
    fn check_dj_transitions(&mut self, ctx: &egui::Context, position_ms: u64, duration_ms: u64, is_playing: bool) {
        let is_in_trans = self.player.snapshot().and_then(|s| s.transition_info).is_some();

        // 1. Detect transition completion: Deck B has finished transitioning into Deck A!
        if self.was_in_transition && !is_in_trans {
            self.was_in_transition = false;
            self.prep_transition_started = false;

            // If we had a planned winner, synchronize queue position
            if let Some(verdict) = &self.last_verdict {
                if let Some(pos) = self.queue.iter().position(|&q| q == verdict.track_idx) {
                    self.queue_pos = Some(pos);
                }
            }
        }
        if is_in_trans {
            self.was_in_transition = true;
        }

        // Only schedule automated JIT transitions in Smart Shuffle mode
        if self.shuffle != ShuffleMode::Smart || !is_playing || duration_ms < 10_000 {
            return;
        }

        // 2. Preload upcoming Track B into Deck B when remaining time <= 32 seconds
        let remaining_ms = duration_ms.saturating_sub(position_ms);
        if remaining_ms <= 32_000 && !self.prep_transition_started && self.player.pending_transition().is_none() && !is_in_trans {
            if let Some(current) = self.current_track() {
                let candidate_indices: Vec<usize> = self
                    .queue
                    .iter()
                    .copied()
                    .filter(|&i| Some(i) != self.current_track_idx())
                    .collect();

                let pool = if candidate_indices.is_empty() {
                    (0..self.tracks.len()).filter(|&i| Some(i) != self.current_track_idx()).collect()
                } else {
                    candidate_indices
                };

                let evals = smart_shuffle::evaluate_candidates(current, &pool, &self.tracks, &self.smart_history);
                if let Some(winner) = evals.first() {
                    if let Some(cand_track) = self.tracks.get(winner.track_idx) {
                        self.last_verdict = Some(winner.clone());
                        self.prep_transition_started = true;
                        self.player.prepare_transition(
                            cand_track.path.clone(),
                            cand_track.title.clone(),
                            cand_track.artist.clone(),
                            ctx.clone(),
                        );
                    }
                }
            }
        }

        // 3. Auto-trigger transition at the computed cue start time
        if let Some(_triggered) = self.player.check_auto_transition() {
            self.was_in_transition = true;
            ctx.request_repaint();
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

                ui.separator();

                // Master FX / Equalizer Button
                let fx_label = if self.master_fx_ui.params.enabled { "🎛 Equalizer (ON)" } else { "🎛 Equalizer" };
                let fx_btn = ui.selectable_label(self.master_fx_ui.is_open, fx_label);
                if fx_btn.clicked() {
                    self.master_fx_ui.is_open = !self.master_fx_ui.is_open;
                }

                let q_btn = ui.selectable_label(self.show_queue_panel, "☰ Queue");
                if q_btn.clicked() {
                    self.show_queue_panel = !self.show_queue_panel;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search title, artist, album")
                            .desired_width(220.0),
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
        if !self.show_queue_panel {
            return;
        }
        let is_smart = self.shuffle == ShuffleMode::Smart;

        egui::SidePanel::right("queue_panel")
            .resizable(true)
            .default_width(320.0)
            .min_width(240.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.strong("Up Next");
                    ui.label(egui::RichText::new(format!("{} tracks", self.queue.len())).weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(!self.queue.is_empty(), egui::Button::new("Clear")).clicked() {
                            self.queue.clear();
                            self.queue_pos = None;
                            self.player.clear_pending_transition();
                        }
                    });
                });

                if is_smart && self.queue.len() < 20 {
                    ui.add_space(4.0);
                    ui.colored_label(
                        Color32::from_rgb(0xE8, 0xB0, 0x50),
                        "⚠ Less than 20 tracks in queue; smart shuffle works best with 20+ tracks for harmonic variety.",
                    );
                }

                ui.separator();
                if self.queue.is_empty() {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(
                            "Queue is empty.\nDouble-click a track to play,\nor use “+” to add it.",
                        )
                        .weak(),
                    );
                    return;
                }

                let mut play_pos = None;
                let mut remove_pos = None;
                let mut move_op = None;

                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for (pos, &idx) in self.queue.iter().enumerate() {
                        let Some(t) = self.tracks.get(idx) else { continue };
                        let is_current = self.queue_pos == Some(pos);
                        ui.horizontal(|ui| {
                            let label = egui::RichText::new(format!("{}. {}", pos + 1, t.title));
                            let label = if is_current {
                                label.color(crate::theme::ACCENT).strong()
                            } else {
                                label
                            };
                            let resp = ui
                                .add(egui::Label::new(label).truncate().sense(egui::Sense::click()))
                                .on_hover_text(format!("{} — {}\n{}", t.artist, t.album, t.detail_line()));
                            if resp.double_clicked() {
                                play_pos = Some(pos);
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("✕").on_hover_text("Remove from queue").clicked() {
                                    remove_pos = Some(pos);
                                }
                                if !is_smart {
                                    if pos + 1 < self.queue.len() && ui.small_button("↓").on_hover_text("Move down").clicked() {
                                        move_op = Some((pos, 1));
                                    }
                                    if pos > 0 && ui.small_button("↑").on_hover_text("Move up").clicked() {
                                        move_op = Some((pos, -1));
                                    }
                                } else if is_current {
                                    ui.label(egui::RichText::new("▶ Playing").small().color(crate::theme::ACCENT));
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
                if let Some((pos, delta)) = move_op {
                    self.move_queue_item(pos, delta);
                }
            });
    }

    fn transport_bar(&mut self, ctx: &egui::Context) -> (u64, u64, bool) {
        let snap = self.player.snapshot();
        let (position_ms, duration_ms, is_playing, finished, sample_rate, underruns, clippings, peak, active_trans) =
            match &snap {
                Some(s) => {
                    let d = if s.active_deck_str == "DeckB" { &s.deck_b } else { &s.deck_a };
                    (
                        d.position_ms,
                        d.duration_ms,
                        s.is_playing,
                        d.is_audible_finished,
                        s.sample_rate,
                        s.underrun_count,
                        s.clipping_count,
                        s.peak_amplitude,
                        s.transition_info.clone(),
                    )
                }
                None => (0, 0, false, false, 0, 0, 0, 0.0, None),
            };

        let load_state = self.player.load_state();
        let pending_trans = self.player.pending_transition();
        let has_active_trans = active_trans.is_some();

        // Automated DJ transitions & background preloading check
        self.check_dj_transitions(ctx, position_ms, duration_ms, is_playing);

        // Standard linear auto-advance fallback if not in transition
        if self.shuffle != ShuffleMode::Smart && load_state == LoadState::Ready && duration_ms > 0 {
            if is_playing && !finished {
                self.advance_armed = true;
            } else if finished && self.advance_armed {
                self.advance_armed = false;
                self.next(ctx, true);
            }
        }

        egui::TopBottomPanel::bottom("transport").min_height(108.0).show(ctx, |ui| {
            ui.add_space(6.0);

            // Transition Banner / Pill (Shows live during active or primed DJ transition)
            if let Some((_elapsed, total, progress, archetype)) = active_trans {
                ui.horizontal(|ui| {
                    let pct = (progress * 100.0).clamp(0.0, 100.0) as u32;
                    let sec = (total as f32 / sample_rate as f32).max(0.5);
                    let label = format!("⚡ LIVE TRANSITION: {archetype} • {sec:.1}s ({pct}%)");
                    let resp = ui.button(egui::RichText::new(label).strong().color(Color32::from_rgb(0x10, 0x30, 0x25)))
                        .on_hover_text("Click to view transition match details");
                    if resp.clicked() {
                        self.show_verdict_modal = true;
                    }
                    ui.add(egui::ProgressBar::new(progress).animate(true).desired_width(180.0));
                });
                ui.add_space(2.0);
            } else if let Some(info) = &pending_trans {
                ui.horizontal(|ui| {
                    let label = format!("⚡ {} • {:.1}s ({}% match) >", info.archetype_name, info.duration_sec, info.confidence_pct);
                    let btn = ui.add(
                        egui::Button::new(egui::RichText::new(label).color(crate::theme::ACCENT))
                            .fill(Color32::from_rgb(0x0C, 0x2A, 0x24)),
                    );
                    if btn.clicked() {
                        self.show_verdict_modal = true;
                    }
                    if ui.small_button("Mix Now").on_hover_text("Trigger transition immediately").clicked() {
                        self.player.trigger_transition_now();
                    }
                });
                ui.add_space(2.0);
            }

            ui.horizontal(|ui| {
                // Artwork thumbnail
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

                // Title & Track Details
                ui.vertical(|ui| {
                    ui.set_width(240.0);
                    match self.current_track() {
                        Some(t) => {
                            ui.add(egui::Label::new(egui::RichText::new(&t.title).strong().size(15.0)).truncate());
                            ui.add(egui::Label::new(egui::RichText::new(&t.artist).weak()).truncate());
                            render_audio_quality_badge(ui, &t.format, t.sample_rate);
                            let mut meta = vec![t.format.clone()];
                            if t.sample_rate > 0 {
                                meta.push(format!("{:.1} kHz", t.sample_rate as f32 / 1000.0));
                            }
                            if let Some(key) = &t.key {
                                meta.push(format!("Key {key}"));
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

                // Player Controls & Volume
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let btn = |s: &str| {
                            egui::Button::new(egui::RichText::new(s).size(18.0))
                                .min_size(egui::vec2(36.0, 32.0))
                        };
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

                        ui.separator();

                        // Shuffle Mode Switcher
                        let shuffle_btn = ui.button(self.shuffle.label());
                        if shuffle_btn.clicked() {
                            self.shuffle = self.shuffle.next();
                            if self.shuffle != ShuffleMode::Smart {
                                self.player.clear_pending_transition();
                            }
                        }

                        // Repeat Mode Switcher
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

                    // Scrubber Bar
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

            // Live Telemetry Bar
            ui.horizontal(|ui| {
                let hw = if self.player.has_hardware_output() { "hardware" } else { "headless" };
                let peak_db = if peak > 0.0001 { 20.0 * peak.log10() } else { -90.0 };
                ui.label(
                    egui::RichText::new(format!(
                        "Output: {hw} @ {sample_rate} Hz • Peak: {peak_db:.1} dBFS • Underruns: {underruns} • Clips: {clippings}"
                    ))
                    .small()
                    .weak(),
                );
            });
            ui.add_space(4.0);
        });

        if is_playing || matches!(load_state, LoadState::Decoding(_)) || has_active_trans {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        (position_ms, duration_ms, is_playing)
    }

    fn nav_rail(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nav_rail")
            .resizable(false)
            .exact_width(176.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("✦ Attune").strong().color(crate::theme::ACCENT));
                });
                ui.label(egui::RichText::new(format!("Desktop v{}", config::app_version())).weak().small());

                ui.add_space(14.0);

                for stage in StageView::ALL {
                    let is_selected = self.current_stage == stage;
                    let text = egui::RichText::new(stage.label()).size(14.0);
                    let text = if is_selected {
                        text.color(crate::theme::ACCENT).strong()
                    } else {
                        text.color(Color32::from_white_alpha(190))
                    };

                    let btn = egui::Button::new(text)
                        .min_size(egui::vec2(ui.available_width(), 34.0))
                        .fill(if is_selected {
                            Color32::from_rgba_premultiplied(0x14, 0x30, 0x2A, 200)
                        } else {
                            Color32::TRANSPARENT
                        });

                    if ui.add(btn).clicked() {
                        self.current_stage = stage;
                    }
                    ui.add_space(3.0);
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(10.0);

                // Quick toggles in rail
                let fx_label = if self.master_fx_ui.params.enabled { "🎛 Equalizer (ON)" } else { "🎛 Equalizer" };
                if ui.button(egui::RichText::new(fx_label).size(13.0)).clicked() {
                    self.master_fx_ui.is_open = !self.master_fx_ui.is_open;
                }

                ui.add_space(4.0);
                let q_label = if self.show_queue_panel { "☰ Hide Queue" } else { "☰ Show Queue" };
                if ui.button(egui::RichText::new(q_label).size(13.0)).clicked() {
                    self.show_queue_panel = !self.show_queue_panel;
                }
            });
    }

    fn central_stage(
        &mut self,
        ctx: &egui::Context,
        position_ms: u64,
        duration_ms: u64,
        is_playing: bool,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Living Ambient Mesh Background
            let rect = ui.max_rect();
            crate::theme::paint_living_ambient_mesh(ui, rect, &self.dynamic_palette, self.animation_time);

            match self.current_stage {
                StageView::NowPlaying => {
                    let cur_track = self.current_track().cloned();
                    self.now_playing_stage.show(
                        ui,
                        cur_track.as_ref(),
                        &self.player,
                        &self.current_lyrics,
                        position_ms,
                        duration_ms,
                        is_playing,
                    );
                }
                StageView::Library => {
                    self.library_content(ui, ctx);
                }
                StageView::Albums => {
                    let mut play_track = None;
                    let mut play_album = None;
                    let mut enqueue_album = None;

                    self.albums_stage.show(
                        ui,
                        &mut self.tracks,
                        &mut |idx| play_track = Some(idx),
                        &mut |indices| play_album = Some(indices.to_vec()),
                        &mut |indices| enqueue_album = Some(indices.to_vec()),
                    );

                    if let Some(idx) = play_track {
                        self.play_track_now(idx, ctx);
                    }
                    if let Some(indices) = play_album {
                        if !indices.is_empty() {
                            self.queue.clear();
                            self.queue.extend_from_slice(&indices);
                            self.play_queue_pos(0, ctx);
                        }
                    }
                    if let Some(indices) = enqueue_album {
                        self.queue.extend_from_slice(&indices);
                    }
                }
                StageView::Lyrics => {
                    self.lyrics_stage.show(ui, &self.current_lyrics, position_ms);
                }
                StageView::Bubblegrid => {
                    let mut play_track = None;
                    self.bubblegrid_stage.show(ui, &self.tracks, &mut |idx| play_track = Some(idx));
                    if let Some(idx) = play_track {
                        self.play_track_now(idx, ctx);
                    }
                }
                StageView::About => {
                    AboutStage::show(ui);
                }
            }
        });
    }

    fn library_content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Music Library").strong().color(crate::theme::ACCENT));
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
        let current_idx = self.current_track_idx();

        let mut play_now = None;
        let mut play_next = None;
        let mut enqueue = None;
        let row_height = 26.0;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(
            ui,
            row_height,
            filtered.len(),
            |ui, range| {
                for &idx in &filtered[range] {
                    let t = &self.tracks[idx];
                    ui.horizontal(|ui| {
                        ui.set_height(row_height);

                        // Queue buttons
                        if ui.small_button("+").on_hover_text("Add to queue").clicked() {
                            enqueue = Some(idx);
                        }
                        if ui.small_button("↳").on_hover_text("Play next").clicked() {
                            play_next = Some(idx);
                        }

                        let title = egui::RichText::new(&t.title);
                        let title = if current_idx == Some(idx) {
                            title.color(crate::theme::ACCENT).strong()
                        } else {
                            title
                        };

                        let left = egui::Layout::left_to_right(egui::Align::Center);
                        let title_w = ui.available_width() * 0.44;
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

                        let artist_w = ui.available_width() * 0.58;
                        ui.allocate_ui_with_layout(egui::vec2(artist_w, row_height), left, |ui| {
                            ui.set_width(artist_w);
                            ui.add(egui::Label::new(egui::RichText::new(&t.artist).weak()).truncate());
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(fmt_ms(t.duration_ms)).monospace().weak());
                            if let Some(key) = &t.key {
                                ui.label(egui::RichText::new(key).small().weak());
                            }
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
        if let Some(idx) = play_next {
            self.play_track_next(idx);
        }
        if let Some(idx) = enqueue {
            self.enqueue(idx);
        }
    }

    fn verdict_modal(&mut self, ctx: &egui::Context) {
        if !self.show_verdict_modal {
            return;
        }

        let mut close_clicked = false;
        let mut is_open = self.show_verdict_modal;
        egui::Window::new("✨ Smart Shuffle Transition Rationale")
            .open(&mut is_open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                if let Some(info) = self.player.pending_transition() {
                    ui.group(|ui| {
                        ui.heading(egui::RichText::new(format!("Upcoming: {}", info.incoming_title)).color(crate::theme::ACCENT));
                        ui.label(egui::RichText::new(format!("By {}", info.incoming_artist)).weak());
                        ui.add_space(6.0);
                        ui.label(format!("Archetype: {}", info.archetype_name));
                        ui.label(format!("Duration: {:.1}s", info.duration_sec));
                        ui.label(format!("Confidence Score: {}%", info.confidence_pct));
                    });
                }

                if let Some(v) = &self.last_verdict {
                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.strong("Harmonic & Rhythm Breakdown:");
                        ui.label(format!("• Candidate: {} — {}", v.title, v.artist));
                        ui.label(format!("• Tempo: {} ({:.0}%)", v.tempo_verdict, v.tempo_score * 100.0));
                        ui.label(format!("• Musical Key: {} ({:.0}%)", v.harmonic_verdict, v.harmonic_score * 100.0));
                        ui.label(format!("• Genre Alignment: {:.0}%", v.genre_score * 100.0));
                        if v.recency_penalty != 0.0 {
                            ui.label(format!("• Recency Penalty: {:+.0}%", v.recency_penalty * 100.0));
                        }
                        ui.label(format!("• Rotation Pacing: {}", v.rotation_verdict));
                        ui.label(format!("• Overall Compatibility Match: {:.0}%", v.total_score * 100.0));
                    });
                } else {
                    ui.label("No active transition candidate evaluated yet.");
                }

                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    close_clicked = true;
                }
            });
        self.show_verdict_modal = is_open && !close_clicked;
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
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
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        self.animation_time += dt;
        self.now_playing_stage.update_animations(dt);

        self.drain_scan();
        self.handle_dropped_files(ctx);
        self.handle_shortcuts(ctx);

        self.top_bar(ctx);
        let (position_ms, duration_ms, is_playing) = self.transport_bar(ctx);
        self.nav_rail(ctx);
        self.queue_panel(ctx);
        self.central_stage(ctx, position_ms, duration_ms, is_playing);

        // Sub-modals
        let player = &self.player;
        self.master_fx_ui.show(ctx, |params| {
            player.set_master_fx_params(params.clone());
        });
        self.verdict_modal(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.music_dir_source == MusicDirSource::Persisted {
            if let Some(d) = &self.music_dir {
                eframe::set_value(storage, STORAGE_KEY_MUSIC_DIR, &d.display().to_string());
            }
        }
        eframe::set_value(storage, STORAGE_KEY_VOLUME_DB, &self.volume_db);
        eframe::set_value(storage, STORAGE_KEY_SHUFFLE, &self.shuffle);
        eframe::set_value(storage, STORAGE_KEY_REPEAT, &self.repeat);
        eframe::set_value(storage, STORAGE_KEY_STAGE, &self.current_stage);
        eframe::set_value(storage, STORAGE_KEY_NOW_PLAYING_SKIN, &self.now_playing_stage.skin);
    }
}
