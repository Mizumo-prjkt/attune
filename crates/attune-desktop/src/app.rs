//! Attune egui application shell: pure desktop experience, multi-stage architecture, and Smart DJ transitions.
//!
//! Features:
//! - Pure Desktop Experience (Left Sidebar, Top Navigation, Right Lyrics/Queue Dock, Bottom Transport Bar)
//! - Direct in-process `DualDeckMixer` + cpal `AudioOutputDriver` (no C-ABI hop)
//! - Smart Shuffle DJ transitions (harmonic Camelot match, BPM sync, recency protection)
//! - Live transition pill with progress and interactive candidate verdict modal
//! - 10-Band Graphic Equalizer, Attune Pure Bass+, Clarity & Vocal Air, Stereo Expander
//! - Drag & drop audio files and folders directly into the player
//! - Dynamic environment resolution (Mandate 4: zero hardcoded paths or versions)

use std::collections::HashSet;
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
use crate::stages::for_you::ForYouStage;
use crate::stages::home::HomeStage;
use crate::stages::lyrics::LyricsStage;
use crate::stages::now_playing::{NowPlayingSkin, NowPlayingStage};
use crate::theme::DynamicPalette;

const STORAGE_KEY_MUSIC_DIR: &str = "attune.music_dir";
const STORAGE_KEY_VOLUME_DB: &str = "attune.volume_db";
const STORAGE_KEY_SHUFFLE: &str = "attune.shuffle_mode";
const STORAGE_KEY_REPEAT: &str = "attune.repeat_mode";
const STORAGE_KEY_STAGE: &str = "attune.stage_view";
const STORAGE_KEY_NOW_PLAYING_SKIN: &str = "attune.now_playing_skin";
const STORAGE_KEY_DOCK_MODE: &str = "attune.dock_mode";
const STORAGE_KEY_FAVORITES: &str = "attune.favorites";
const VOLUME_DB_RANGE: std::ops::RangeInclusive<f32> = -40.0..=0.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StageView {
    Home,
    NowPlaying,
    ForYou,
    Library,
    Albums,
    Bubblegrid,
    Favorites,
    About,
}

impl StageView {
    #[allow(dead_code)]
    pub const ALL: [StageView; 8] = [
        StageView::Home,
        StageView::NowPlaying,
        StageView::ForYou,
        StageView::Library,
        StageView::Albums,
        StageView::Bubblegrid,
        StageView::Favorites,
        StageView::About,
    ];

    pub fn label(self) -> &'static str {
        match self {
            StageView::Home => "🏠 Home",
            StageView::NowPlaying => "▶ Now Playing",
            StageView::ForYou => "✨ For You",
            StageView::Library => "♫ Library",
            StageView::Albums => "💿 Albums",
            StageView::Bubblegrid => "🫧 Bubblegrid",
            StageView::Favorites => "❤️ Favorites",
            StageView::About => "⚙ Settings / About",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DockMode {
    Lyrics,
    Queue,
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
    // Navigation & Stages
    current_stage: StageView,
    stage_history: Vec<StageView>,
    history_index: usize,
    dock_mode: Option<DockMode>,
    favorites: HashSet<PathBuf>,
    // Stage instances
    home_stage: HomeStage,
    now_playing_stage: NowPlayingStage,
    albums_stage: AlbumsStage,
    lyrics_stage: LyricsStage,
    bubblegrid_stage: BubblegridStage,
    current_lyrics: LyricsSheet,
    dynamic_palette: DynamicPalette,
    animation_time: f32,
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
            .unwrap_or(StageView::Home);
        let now_playing_skin = cc
            .storage
            .and_then(|s| eframe::get_value::<NowPlayingSkin>(s, STORAGE_KEY_NOW_PLAYING_SKIN))
            .unwrap_or(NowPlayingSkin::LiquidGlass);
        let dock_mode = cc
            .storage
            .and_then(|s| eframe::get_value::<Option<DockMode>>(s, STORAGE_KEY_DOCK_MODE))
            .unwrap_or(Some(DockMode::Lyrics));
        let favorites: HashSet<PathBuf> = cc
            .storage
            .and_then(|s| eframe::get_value::<Vec<String>>(s, STORAGE_KEY_FAVORITES))
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();

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
            stage_history: vec![current_stage],
            history_index: 0,
            dock_mode,
            favorites,
            home_stage: HomeStage::new(),
            now_playing_stage,
            albums_stage: AlbumsStage::new(),
            lyrics_stage: LyricsStage::new(),
            bubblegrid_stage: BubblegridStage::new(),
            current_lyrics: LyricsSheet::empty(),
            dynamic_palette: DynamicPalette::default(),
            animation_time: 0.0,
        };
        app.start_scan(&cc.egui_ctx);
        app
    }

    // ------------------------------------------------------------------ Navigation

    fn navigate_to(&mut self, stage: StageView) {
        if self.current_stage == stage {
            return;
        }
        if self.history_index + 1 < self.stage_history.len() {
            self.stage_history.truncate(self.history_index + 1);
        }
        self.stage_history.push(stage);
        self.history_index = self.stage_history.len() - 1;
        self.current_stage = stage;
    }

    fn can_navigate_back(&self) -> bool {
        self.history_index > 0
    }

    fn can_navigate_forward(&self) -> bool {
        self.history_index + 1 < self.stage_history.len()
    }

    fn navigate_back(&mut self) {
        if self.can_navigate_back() {
            self.history_index -= 1;
            self.current_stage = self.stage_history[self.history_index];
        }
    }

    fn navigate_forward(&mut self) {
        if self.can_navigate_forward() {
            self.history_index += 1;
            self.current_stage = self.stage_history[self.history_index];
        }
    }

    // ------------------------------------------------------------------ Library & Playback

    fn start_scan(&mut self, ctx: &egui::Context) {
        let Some(dir) = self.music_dir.clone() else {
            self.status = Some("No music folder set — choose one in the sidebar to build your library.".into());
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

    // ------------------------------------------------------------------ UI Layout Panels

    /// Left Sidebar: Search at top, navigation categories, music sources, playlists, and settings.
    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(210.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                // 1. Search Box
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("🔍 Search...")
                            .desired_width(ui.available_width() - 4.0),
                    );
                    if resp.changed() {
                        if self.current_stage != StageView::Library && !self.search.trim().is_empty() {
                            self.navigate_to(StageView::Library);
                        }
                        ctx.request_repaint();
                    }
                });

                ui.add_space(14.0);

                // 2. Section: ATTUNE
                ui.label(egui::RichText::new("ATTUNE").strong().size(11.0).color(Color32::from_white_alpha(110)));
                ui.add_space(4.0);

                let main_stages = [
                    StageView::Home,
                    StageView::NowPlaying,
                    StageView::ForYou,
                    StageView::Library,
                    StageView::Albums,
                ];

                for stage in main_stages {
                    let is_selected = self.current_stage == stage;
                    let text = egui::RichText::new(stage.label()).size(14.0);
                    let text = if is_selected {
                        text.color(crate::theme::ACCENT).strong()
                    } else {
                        text.color(Color32::from_white_alpha(190))
                    };

                    let btn = egui::Button::new(text)
                        .min_size(egui::vec2(ui.available_width(), 32.0))
                        .fill(if is_selected {
                            Color32::from_rgba_premultiplied(0x14, 0x30, 0x2A, 220)
                        } else {
                            Color32::TRANSPARENT
                        });

                    if ui.add(btn).clicked() {
                        self.navigate_to(stage);
                    }
                    ui.add_space(2.0);
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // 3. Section: SOURCES
                ui.label(egui::RichText::new("SOURCES").strong().size(11.0).color(Color32::from_white_alpha(110)));
                ui.add_space(4.0);

                let dir_label = self
                    .music_dir
                    .as_ref()
                    .and_then(|d| d.file_name())
                    .map(|f| format!("📁 {}", f.to_string_lossy()))
                    .unwrap_or_else(|| "📁 No folder set".to_string());
                ui.add(egui::Label::new(egui::RichText::new(dir_label).small().monospace()).truncate())
                    .on_hover_text(format!(
                        "{}\nSource: {}",
                        self.music_dir
                            .as_ref()
                            .map(|d| d.display().to_string())
                            .unwrap_or_else(|| "Click Choose to set music folder".to_string()),
                        self.music_dir_source.label()
                    ));

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.small_button("Choose").clicked() {
                        self.pick_music_dir(ctx);
                    }
                    let rescan_btn = ui.add_enabled(
                        self.music_dir.is_some() && self.scan_rx.is_none(),
                        egui::Button::new("Rescan").small(),
                    );
                    if rescan_btn.clicked() {
                        self.start_scan(ctx);
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // 4. Section: PLAYLISTS & MIXES
                ui.label(egui::RichText::new("PLAYLISTS & MIXES").strong().size(11.0).color(Color32::from_white_alpha(110)));
                ui.add_space(4.0);

                // Favorites button
                {
                    let is_selected = self.current_stage == StageView::Favorites;
                    let fav_count = self.favorites.len();
                    let fav_label = format!("❤️ Favorites ({fav_count})");
                    let text = egui::RichText::new(fav_label).size(13.0);
                    let text = if is_selected {
                        text.color(crate::theme::ACCENT).strong()
                    } else {
                        text.color(Color32::from_white_alpha(190))
                    };
                    let btn = egui::Button::new(text)
                        .min_size(egui::vec2(ui.available_width(), 30.0))
                        .fill(if is_selected {
                            Color32::from_rgba_premultiplied(0x14, 0x30, 0x2A, 220)
                        } else {
                            Color32::TRANSPARENT
                        });
                    if ui.add(btn).clicked() {
                        self.navigate_to(StageView::Favorites);
                    }
                }

                ui.add_space(2.0);

                // Bubblegrid button
                {
                    let is_selected = self.current_stage == StageView::Bubblegrid;
                    let text = egui::RichText::new(StageView::Bubblegrid.label()).size(13.0);
                    let text = if is_selected {
                        text.color(crate::theme::ACCENT).strong()
                    } else {
                        text.color(Color32::from_white_alpha(190))
                    };
                    let btn = egui::Button::new(text)
                        .min_size(egui::vec2(ui.available_width(), 30.0))
                        .fill(if is_selected {
                            Color32::from_rgba_premultiplied(0x14, 0x30, 0x2A, 220)
                        } else {
                            Color32::TRANSPARENT
                        });
                    if ui.add(btn).clicked() {
                        self.navigate_to(StageView::Bubblegrid);
                    }
                }

                // 5. Bottom Section: Settings & Equalizer
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(8.0);
                    let about_selected = self.current_stage == StageView::About;
                    let text = egui::RichText::new(StageView::About.label()).size(13.0);
                    let text = if about_selected {
                        text.color(crate::theme::ACCENT).strong()
                    } else {
                        text.color(Color32::from_white_alpha(160))
                    };
                    let btn = egui::Button::new(text)
                        .min_size(egui::vec2(ui.available_width(), 30.0))
                        .fill(if about_selected {
                            Color32::from_rgba_premultiplied(0x14, 0x30, 0x2A, 220)
                        } else {
                            Color32::TRANSPARENT
                        });
                    if ui.add(btn).clicked() {
                        self.navigate_to(StageView::About);
                    }

                    ui.add_space(2.0);
                    let fx_label = if self.master_fx_ui.params.enabled { "🎛 Equalizer (ON)" } else { "🎛 Equalizer" };
                    if ui.button(egui::RichText::new(fx_label).size(13.0)).clicked() {
                        self.master_fx_ui.is_open = !self.master_fx_ui.is_open;
                    }
                    ui.add_space(4.0);
                    ui.separator();
                });
            });
    }

    /// Top Navigation Bar (inside Central Stage area): History navigation `<` `>`, status, and right toolbar toggles.
    fn top_nav_bar(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.horizontal(|ui| {
            // History Navigation Buttons
            let back_btn = ui.add_enabled(
                self.can_navigate_back(),
                egui::Button::new(egui::RichText::new(" < ").size(15.0).strong()),
            );
            if back_btn.clicked() {
                self.navigate_back();
            }

            let fwd_btn = ui.add_enabled(
                self.can_navigate_forward(),
                egui::Button::new(egui::RichText::new(" > ").size(15.0).strong()),
            );
            if fwd_btn.clicked() {
                self.navigate_forward();
            }

            ui.add_space(8.0);

            // Scan / Driver Telemetry
            if self.scan_rx.is_some() {
                ui.spinner();
                ui.label(egui::RichText::new(format!("Scanning {}/{}...", self.tracks.len(), self.scan_expected)).small().weak());
            } else if let Some(msg) = &self.status {
                ui.label(egui::RichText::new(msg).small().weak());
            } else if !self.player.has_hardware_output() {
                ui.colored_label(
                    Color32::from_rgb(0xE0, 0x7A, 0x5F),
                    format!("⚠ Headless ({})", self.player.driver_error().unwrap_or("no device")),
                );
            }

            // Right-aligned toolbar toggles
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Equalizer toggle
                let eq_active = self.master_fx_ui.is_open;
                let eq_label = if self.master_fx_ui.params.enabled { "🎚️ EQ (ON)" } else { "🎚️ EQ" };
                let eq_btn = ui.selectable_label(eq_active, eq_label);
                if eq_btn.clicked() {
                    self.master_fx_ui.is_open = !self.master_fx_ui.is_open;
                }

                // Queue dock toggle
                let q_active = self.dock_mode == Some(DockMode::Queue);
                let q_btn = ui.selectable_label(q_active, "☰ Queue");
                if q_btn.clicked() {
                    self.dock_mode = if q_active { None } else { Some(DockMode::Queue) };
                }

                // Lyrics dock toggle
                let l_active = self.dock_mode == Some(DockMode::Lyrics);
                let l_btn = ui.selectable_label(l_active, "💬 Lyrics");
                if l_btn.clicked() {
                    self.dock_mode = if l_active { None } else { Some(DockMode::Lyrics) };
                }
            });
        });
    }

    /// Right Docked Panel: shows Synchronized Lyrics or Up Next Queue.
    fn docked_right_panel(&mut self, ctx: &egui::Context, position_ms: u64) {
        let Some(mode) = self.dock_mode else { return };

        egui::SidePanel::right("right_dock")
            .resizable(true)
            .default_width(320.0)
            .min_width(260.0)
            .max_width(480.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                match mode {
                    DockMode::Lyrics => {
                        self.lyrics_stage.show_docked(ui, &self.current_lyrics, position_ms);
                    }
                    DockMode::Queue => {
                        self.queue_dock_content(ui, ctx);
                    }
                }
            });
    }

    fn queue_dock_content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let is_smart = self.shuffle == ShuffleMode::Smart;

        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Up Next").strong().size(18.0).color(Color32::from_white_alpha(230)));
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
                "⚠ Less than 20 tracks; Smart Shuffle works best with 20+ tracks for harmonic variety.",
            );
        }

        ui.add_space(4.0);
        ui.separator();

        if self.queue.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Queue is empty").size(15.0).weak());
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Double-click a track in the library or use “+” to add songs.").small().weak());
            });
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
    }

    /// Bottom Transport Bar:
    /// - Row 1: Full-width scrubber (`0:42 ━━━━━━●━━━━━━ -2:01` with negative remaining time)
    /// - Row 2: Controls (`🔀 ⏮ ▶/⏸ ⏭ 🔁`), Mini Now Playing Pill in center, Actions (`❤️ 🔊━━ ⛶`) on right.
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

        egui::TopBottomPanel::bottom("transport").min_height(104.0).show(ctx, |ui| {
            ui.add_space(4.0);

            // Row 1: Full-width Scrubber with elapsed time and negative remaining time
            ui.horizontal(|ui| {
                let shown = self.scrub_ms.unwrap_or(position_ms as f64);
                ui.label(egui::RichText::new(fmt_ms(shown as u64)).monospace().size(12.0));

                let max = duration_ms.max(1) as f64;
                let mut val = shown.min(max);
                let width = (ui.available_width() - 65.0).max(120.0);
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

                let remaining_ms = duration_ms.saturating_sub(shown as u64);
                let rem_text = if duration_ms > 0 {
                    format!("-{}", fmt_ms(remaining_ms))
                } else {
                    "--:--".to_string()
                };
                ui.label(egui::RichText::new(rem_text).monospace().size(12.0).weak());
            });

            ui.add_space(2.0);

            // Row 2: Left Playback Controls, Center Mini Now Playing Pill, Right Actions & Volume
            ui.horizontal(|ui| {
                // Left Controls: Shuffle, Previous, Play/Pause, Next, Repeat
                let btn = |s: &str| {
                    egui::Button::new(egui::RichText::new(s).size(16.0))
                        .min_size(egui::vec2(34.0, 30.0))
                };

                let shuffle_btn = ui.button(self.shuffle.label());
                if shuffle_btn.clicked() {
                    self.shuffle = self.shuffle.next();
                    if self.shuffle != ShuffleMode::Smart {
                        self.player.clear_pending_transition();
                    }
                }

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

                // Center: Mini Now Playing Pill
                let thumb_size = egui::vec2(44.0, 44.0);
                match self.current_track().and_then(|t| t.artwork_path.clone()) {
                    Some(p) => {
                        let resp = ui.add(
                            egui::Image::new(format!("file://{}", p.display()))
                                .fit_to_exact_size(thumb_size)
                                .corner_radius(6.0)
                                .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            self.navigate_to(StageView::NowPlaying);
                        }
                    }
                    None => {
                        let (rect, resp) = ui.allocate_exact_size(thumb_size, egui::Sense::click());
                        ui.painter().rect_filled(rect, 6.0, ui.visuals().faint_bg_color);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "♪",
                            egui::FontId::proportional(22.0),
                            ui.visuals().weak_text_color(),
                        );
                        if resp.clicked() {
                            self.navigate_to(StageView::NowPlaying);
                        }
                    }
                }

                ui.vertical(|ui| {
                    ui.set_width(220.0);
                    match self.current_track() {
                        Some(t) => {
                            ui.add(egui::Label::new(egui::RichText::new(&t.title).strong().size(13.0)).truncate());
                            ui.add(egui::Label::new(egui::RichText::new(&t.artist).weak().size(11.0)).truncate());
                            ui.horizontal(|ui| {
                                render_audio_quality_badge(ui, &t.format, t.sample_rate);
                                if let Some(key) = &t.key {
                                    ui.label(egui::RichText::new(key).small().color(Color32::from_rgb(0x38, 0xBD, 0xF8)));
                                }
                            });
                        }
                        None => {
                            ui.label(egui::RichText::new("Nothing playing").weak().size(12.0));
                        }
                    }
                });

                // DJ Transition Status Pill
                if let Some((_elapsed, total, progress, archetype)) = &active_trans {
                    let pct = (progress * 100.0).clamp(0.0, 100.0) as u32;
                    let sec = (*total as f32 / sample_rate as f32).max(0.5);
                    let label = format!("⚡ LIVE: {archetype} ({pct}%)");
                    if ui.button(egui::RichText::new(label).strong().color(Color32::from_rgb(0x10, 0x30, 0x25)))
                        .on_hover_text(format!("Transitioning over {sec:.1}s"))
                        .clicked()
                    {
                        self.show_verdict_modal = true;
                    }
                } else if let Some(info) = &pending_trans {
                    let label = format!("⚡ {} >", info.archetype_name);
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
                }

                // Right Actions: Favorite, Volume, Expand Now Playing
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Expand Now Playing Stage Button
                    if ui.button(egui::RichText::new("⛶").size(16.0)).on_hover_text("Open Now Playing stage").clicked() {
                        self.navigate_to(StageView::NowPlaying);
                    }

                    // Volume Slider
                    let vol = ui.add(
                        egui::Slider::new(&mut self.volume_db, VOLUME_DB_RANGE)
                            .suffix(" dB")
                            .fixed_decimals(0),
                    );
                    if vol.changed() {
                        self.player.set_gain_db(self.volume_db);
                    }
                    ui.label("🔊");

                    // Favorite Button
                    let is_fav = self
                        .current_track()
                        .map(|t| self.favorites.contains(&t.path))
                        .unwrap_or(false);
                    let fav_color = if is_fav {
                        Color32::from_rgb(0xFB, 0x71, 0x85)
                    } else {
                        Color32::from_white_alpha(100)
                    };
                    let fav_btn = egui::Button::new(egui::RichText::new("❤️").color(fav_color));
                    if ui.add(fav_btn).on_hover_text(if is_fav { "Remove from Favorites" } else { "Add to Favorites" }).clicked() {
                        if let Some(t) = self.current_track() {
                            let path = t.path.clone();
                            if is_fav {
                                self.favorites.remove(&path);
                            } else {
                                self.favorites.insert(path);
                            }
                        }
                    }
                });
            });

            // Bottom Sub-line: Telemetry
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
            ui.add_space(2.0);
        });

        if is_playing || matches!(load_state, LoadState::Decoding(_)) || has_active_trans {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        (position_ms, duration_ms, is_playing)
    }

    /// Central stage rendering: ambient mesh background + top navigation bar + stage view.
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

            // Top Navigation Bar
            self.top_nav_bar(ui, ctx);
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            match self.current_stage {
                StageView::Home => {
                    let cur_track = self.current_track().cloned();
                    let mut play_idx = None;
                    let mut toggle_play = false;
                    let mut skip_next = false;
                    let mut open_np = false;

                    self.home_stage.show(
                        ui,
                        &self.tracks,
                        cur_track.as_ref(),
                        is_playing,
                        &self.player,
                        &mut |idx| play_idx = Some(idx),
                        &mut || toggle_play = true,
                        &mut || skip_next = true,
                        &mut || open_np = true,
                    );

                    if let Some(idx) = play_idx {
                        self.play_track_now(idx, ctx);
                    }
                    if toggle_play {
                        if is_playing {
                            self.player.pause();
                        } else if self.queue_pos.is_none() && !self.queue.is_empty() {
                            self.play_queue_pos(0, ctx);
                        } else {
                            self.player.play();
                        }
                    }
                    if skip_next {
                        self.next(ctx, false);
                    }
                    if open_np {
                        self.navigate_to(StageView::NowPlaying);
                    }
                }
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
                StageView::ForYou => {
                    let mut play_idx = None;
                    let mut play_mix = None;

                    ForYouStage::show(
                        ui,
                        &self.tracks,
                        &mut |idx| play_idx = Some(idx),
                        &mut |indices| play_mix = Some(indices.to_vec()),
                    );

                    if let Some(idx) = play_idx {
                        self.play_track_now(idx, ctx);
                    }
                    if let Some(indices) = play_mix {
                        if !indices.is_empty() {
                            self.queue.clear();
                            self.queue.extend_from_slice(&indices);
                            self.play_queue_pos(0, ctx);
                        }
                    }
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
                StageView::Bubblegrid => {
                    let mut play_track = None;
                    self.bubblegrid_stage.show(ui, &self.tracks, &mut |idx| play_track = Some(idx));
                    if let Some(idx) = play_track {
                        self.play_track_now(idx, ctx);
                    }
                }
                StageView::Favorites => {
                    self.favorites_content(ui, ctx);
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

    fn favorites_content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let fav_indices: Vec<usize> = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| self.favorites.contains(&t.path))
            .map(|(i, _)| i)
            .collect();

        let total_duration_ms: u64 = fav_indices.iter().map(|&i| self.tracks[i].duration_ms).sum();

        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("❤️ Favorites").strong().color(crate::theme::ACCENT));
            ui.label(egui::RichText::new(format!("{} songs • {}", fav_indices.len(), fmt_ms(total_duration_ms))).weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(!fav_indices.is_empty(), egui::Button::new("▶ Play All Favorites")).clicked() {
                    self.queue.clear();
                    self.queue.extend_from_slice(&fav_indices);
                    self.play_queue_pos(0, ctx);
                }
            });
        });

        ui.separator();

        if fav_indices.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No favorite tracks yet.").size(16.0).weak());
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Click the ❤️ button in the transport bar to add songs to your favorites.").weak());
            });
            return;
        }

        let mut play_now = None;
        let mut remove_fav = None;
        let row_height = 28.0;
        let current_idx = self.current_track_idx();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(
            ui,
            row_height,
            fav_indices.len(),
            |ui, range| {
                for &idx in &fav_indices[range] {
                    let t = &self.tracks[idx];
                    ui.horizontal(|ui| {
                        ui.set_height(row_height);

                        if ui.small_button("✕").on_hover_text("Remove from favorites").clicked() {
                            remove_fav = Some(t.path.clone());
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

                        let artist_w = ui.available_width() * 0.55;
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

        if let Some(path) = remove_fav {
            self.favorites.remove(&path);
        }
        if let Some(idx) = play_now {
            self.play_track_now(idx, ctx);
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

        // Desktop layout hierarchy: Sidebar (left) -> Right Dock (right) -> Transport Bar (bottom) -> Central Stage (middle)
        self.sidebar(ctx);
        let (position_ms, duration_ms, is_playing) = self.transport_bar(ctx);
        self.docked_right_panel(ctx, position_ms);
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
        eframe::set_value(storage, STORAGE_KEY_DOCK_MODE, &self.dock_mode);
        let fav_vec: Vec<String> = self.favorites.iter().map(|p| p.display().to_string()).collect();
        eframe::set_value(storage, STORAGE_KEY_FAVORITES, &fav_vec);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_view_labels_and_all() {
        assert_eq!(StageView::ALL.len(), 8);
        for stage in StageView::ALL {
            assert!(!stage.label().is_empty());
        }
        assert_eq!(StageView::Home.label(), "🏠 Home");
        assert_eq!(StageView::NowPlaying.label(), "▶ Now Playing");
    }

    #[test]
    fn test_navigation_history_logic() {
        let mut history = vec![StageView::Home];
        let mut current = StageView::Home;
        let mut history_idx = 0;

        let nav_to = |stage: StageView, hist: &mut Vec<StageView>, idx: &mut usize, cur: &mut StageView| {
            if *cur == stage {
                return;
            }
            if *idx + 1 < hist.len() {
                hist.truncate(*idx + 1);
            }
            hist.push(stage);
            *idx = hist.len() - 1;
            *cur = stage;
        };

        nav_to(StageView::Library, &mut history, &mut history_idx, &mut current);
        assert_eq!(current, StageView::Library);
        assert_eq!(history_idx, 1);
        assert_eq!(history.len(), 2);

        nav_to(StageView::Albums, &mut history, &mut history_idx, &mut current);
        assert_eq!(current, StageView::Albums);
        assert_eq!(history_idx, 2);

        // Go back
        history_idx -= 1;
        current = history[history_idx];
        assert_eq!(current, StageView::Library);

        // Branching: navigate to ForYou from Library
        nav_to(StageView::ForYou, &mut history, &mut history_idx, &mut current);
        assert_eq!(current, StageView::ForYou);
        assert_eq!(history.len(), 3);
        assert_eq!(history, vec![StageView::Home, StageView::Library, StageView::ForYou]);
    }

    #[test]
    fn test_negative_remaining_time_formatting() {
        let duration_ms: u64 = 163_000; // 2:43
        let position_ms: u64 = 42_000;  // 0:42
        let remaining_ms = duration_ms.saturating_sub(position_ms); // 121_000 = 2:01
        assert_eq!(format!("-{}", fmt_ms(remaining_ms)), "-2:01");
    }
}
