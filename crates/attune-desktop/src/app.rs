use dioxus::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use attune_core::MasterFxParams;
use attune_player_syscore_playertoast_broadcast::{
    MediaControlCommand, SystemMediaService, SystemPlaybackState, SystemRepeatMode, SystemTrackMetadata,
};
use crate::app_icon::app_icon_data_url;
use crate::components::lyrics_parser::{resolve_companion_lyrics, LyricsSheet};
use crate::config;
use crate::library::{self, fmt_ms, ScanEvent, TrackEntry};
use crate::player::{LoadState, PendingTransitionInfo, Player};
use crate::smart_shuffle::{self, ShuffleMode, SmartShuffleHistory};
use crate::stages::{
    about::About, album_editor::AlbumEditor, albums::{AlbumGroup, Albums},
    artists::{ArtistDetail, Artists}, equalizer::Equalizer,
    favorites::Favorites, for_you::ForYou, fullscreen_now_playing::FullscreenNowPlaying,
    home::Home, library::Library, process_manager::ProcessManager,
    settings::open_settings_window, setup::SetupStage,
};

#[derive(Debug, Clone, PartialEq)]
pub enum StageView {
    Setup,
    Home,
    ForYou,
    ForYouDetail(String),
    Library,
    Albums,
    AlbumDetail(String),
    Artists,
    ArtistDetail(String),
    Favorites,
    Equalizer,
    AlbumEditor(AlbumGroup),
    ProcessManager,
    About,
    FullscreenNowPlaying,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationState {
    pub history: Vec<StageView>,
    pub index: usize,
}

impl NavigationState {
    pub fn new(initial: StageView) -> Self {
        Self {
            history: vec![initial],
            index: 0,
        }
    }

    pub fn current(&self) -> &StageView {
        &self.history[self.index]
    }

    pub fn can_go_back(&self) -> bool {
        self.index > 0
            || matches!(
                self.current(),
                StageView::AlbumDetail(_)
                    | StageView::AlbumEditor(_)
                    | StageView::ForYouDetail(_)
                    | StageView::ArtistDetail(_)
                    | StageView::FullscreenNowPlaying
            )
    }

    pub fn can_go_forward(&self) -> bool {
        self.index + 1 < self.history.len()
    }

    pub fn push(&mut self, next: StageView) -> bool {
        if self.current() == &next {
            return false;
        }
        // If the immediate previous history entry is identical to `next`,
        // step back to avoid ping-pong history loops [A, B, A, B].
        if self.index > 0 && self.history[self.index - 1] == next {
            self.index -= 1;
            return true;
        }
        self.history.truncate(self.index + 1);
        self.history.push(next);
        self.index = self.history.len() - 1;
        true
    }

    pub fn back(&mut self) -> Option<&StageView> {
        if self.can_go_back() {
            self.index -= 1;
            Some(self.current())
        } else {
            None
        }
    }

    pub fn forward(&mut self) -> Option<&StageView> {
        if self.can_go_forward() {
            self.index += 1;
            Some(self.current())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockMode {
    Lyrics,
    Queue,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

/// Helper: Select next index using current ShuffleMode (Off, Linear, Smart) and recency history.
/// Respects queue sanctity: if the queue has songs, selection is strictly confined to the queue
/// and never escapes into the broader library or injects external songs.
pub fn select_next_playback_index(
    mode: ShuffleMode,
    curr_idx: usize,
    queue: &[TrackEntry],
    all_tracks: &[TrackEntry],
    history: &SmartShuffleHistory,
) -> Option<usize> {
    if !queue.is_empty() {
        match mode {
            ShuffleMode::Off => {
                if curr_idx + 1 < queue.len() {
                    Some(curr_idx + 1)
                } else {
                    None
                }
            }
            ShuffleMode::Linear => {
                if queue.len() <= 1 {
                    None
                } else {
                    use std::time::SystemTime;
                    let seed = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0);
                    let mut next = (seed as usize) % queue.len();
                    if next == curr_idx {
                        next = (next + 1) % queue.len();
                    }
                    Some(next)
                }
            }
            ShuffleMode::Smart => {
                if queue.len() <= 1 {
                    None
                } else {
                    let current = queue.get(curr_idx)?;
                    let candidate_indices: Vec<usize> = (0..queue.len()).filter(|&i| i != curr_idx).collect();
                    let evaluated = smart_shuffle::evaluate_candidates(current, &candidate_indices, queue, history);
                    smart_shuffle::select_smart_candidate(&evaluated).or(Some((curr_idx + 1) % queue.len()))
                }
            }
        }
    } else if !all_tracks.is_empty() {
        match mode {
            ShuffleMode::Off => {
                if curr_idx + 1 < all_tracks.len() {
                    Some(curr_idx + 1)
                } else {
                    None
                }
            }
            ShuffleMode::Linear => {
                if all_tracks.len() <= 1 {
                    None
                } else {
                    use std::time::SystemTime;
                    let seed = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0);
                    let mut idx = (seed as usize) % all_tracks.len();
                    if idx == curr_idx {
                        idx = (idx + 1) % all_tracks.len();
                    }
                    Some(idx)
                }
            }
            ShuffleMode::Smart => {
                if all_tracks.len() <= 1 {
                    None
                } else {
                    let current = all_tracks.get(curr_idx)?;
                    let candidate_indices: Vec<usize> = (0..all_tracks.len()).filter(|&i| i != curr_idx).collect();
                    let evaluated = smart_shuffle::evaluate_candidates(current, &candidate_indices, all_tracks, history);
                    smart_shuffle::select_smart_candidate(&evaluated).or(Some((curr_idx + 1) % all_tracks.len()))
                }
            }
        }
    } else {
        None
    }
}

#[component]
pub fn App() -> Element {
    // 1. Audio Engine & Library State
    let player = use_signal(|| Arc::new(Player::new()));
    let mut tracks = use_signal(Vec::<TrackEntry>::new);
    let app_config = use_signal(config::load_config);
    let strings = crate::i18n::get_strings(crate::i18n::Language::from_code(&app_config.read().language));
    let mut music_dir = use_signal(|| {
        let (dir, _) = config::resolve_music_dir(None);
        dir
    });
    let mut extra_music_dirs = use_signal(|| {
        config::load_config().extra_music_dirs
    });
    let song_exclusions = use_signal(|| {
        config::load_config().song_exclusions
    });
    let mut is_scanning = use_signal(|| false);
    let mut scan_progress = use_signal(|| (0usize, 0usize));
    let mut last_scanned_file = use_signal(String::new);
    let mut scan_rx = use_signal(|| Arc::new(Mutex::new(None::<mpsc::Receiver<ScanEvent>>)));

    // 2. Playback State
    let mut current_track = use_signal(|| None::<TrackEntry>);
    let mut queue = use_signal(Vec::<TrackEntry>::new);
    let mut queue_pos = use_signal(|| 0usize);
    let mut recent_tracks = use_signal(Vec::<TrackEntry>::new);
    let mut favorites = use_signal(HashSet::<PathBuf>::new);

    let mut is_playing = use_signal(|| false);
    let mut playback_pos_ms = use_signal(|| 0u64);
    let mut playback_dur_ms = use_signal(|| 0u64);
    let mut volume_pct = use_signal(|| 85u32); // 0 - 100
    let mut shuffle_mode = use_signal(|| ShuffleMode::Smart); // Smart DJ Shuffle active by default
    let mut shuffle_history = use_signal(SmartShuffleHistory::default);
    let mut has_handled_track_end = use_signal(|| false);
    let mut repeat_mode = use_signal(|| RepeatMode::Off);

    // 3. Lyrics State
    let mut lyrics_sheet = use_signal(LyricsSheet::empty);
    let mut active_lyric_index = use_signal(|| None::<usize>);
    let mut auto_scroll_lyrics = use_signal(|| true);

    // 4. Transitions & DSP / EQ State
    let mut pending_trans_info = use_signal(|| None::<PendingTransitionInfo>);
    let mut active_trans_name = use_signal(|| None::<String>);
    let mut last_verdict = use_signal(|| None::<PendingTransitionInfo>);
    let mut show_verdict_modal = use_signal(|| false);
    let mut is_transition_active = use_signal(|| false);
    let mut transitioning_to_track = use_signal(|| None::<TrackEntry>);
    let mut show_candidates_modal = use_signal(|| false);
    let mut master_fx = use_signal(MasterFxParams::default);

    // 5. Navigation & Docks
    let initial_stage = if crate::config::is_setup_completed() {
        StageView::Home
    } else {
        StageView::Setup
    };
    let mut nav_state = use_signal(move || NavigationState::new(initial_stage));
    let mut dock_mode = use_signal(|| DockMode::Lyrics);
    let mut search_query = use_signal(String::new);

    // Helper: Scans all configured libraries with exclusion filtering
    let scan_all_libraries = {
        let music_dir = music_dir.clone();
        let extra_music_dirs = extra_music_dirs.clone();
        let song_exclusions = song_exclusions.clone();
        let mut tracks = tracks.clone();
        let mut scan_rx = scan_rx.clone();
        let mut is_scanning = is_scanning.clone();
        move || {
            let mut all_dirs = Vec::new();
            if let Some(ref d) = music_dir.read().clone() {
                if d.is_dir() && !all_dirs.contains(d) {
                    all_dirs.push(d.clone());
                }
            }
            for d in extra_music_dirs.read().iter() {
                if d.is_dir() && !all_dirs.contains(d) {
                    all_dirs.push(d.clone());
                }
            }
            if all_dirs.is_empty() {
                return;
            }
            tracks.set(Vec::new());
            let rx = library::spawn_scan_multi(all_dirs, song_exclusions.read().clone());
            *scan_rx.write() = Arc::new(Mutex::new(Some(rx)));
            is_scanning.set(true);
        }
    };

    // Initial cache loading or scan on launch across all resolved music directories
    use_hook(|| {
        let mut all_dirs = Vec::new();
        if let Some(dir) = music_dir.read().clone() {
            if dir.is_dir() {
                all_dirs.push(dir);
            }
        }
        for d in extra_music_dirs.read().iter() {
            if d.is_dir() && !all_dirs.contains(d) {
                all_dirs.push(d.clone());
            }
        }
        let exclusions = song_exclusions.read().clone();

        let mut loaded_tracks = Vec::new();
        let mut any_uncached = false;
        for dir in &all_dirs {
            if let Some(cached) = library::load_library_cache(dir) {
                for t in cached {
                    if !crate::config::AppConfig::is_path_or_title_excluded(&exclusions, &t.path, Some(&t.title)) {
                        loaded_tracks.push(t);
                    }
                }
            } else {
                any_uncached = true;
            }
        }

        if !loaded_tracks.is_empty() && !any_uncached {
            tracks.set(loaded_tracks.clone());
            if queue.read().is_empty() {
                queue.set(loaded_tracks);
            }
        } else if !all_dirs.is_empty() && crate::config::is_setup_completed() {
            let rx = library::spawn_scan_multi(all_dirs, exclusions);
            *scan_rx.write() = Arc::new(Mutex::new(Some(rx)));
            is_scanning.set(true);
        }
    });

    // Helper: Navigation change with history tracking
    let mut navigate_to = move |new_stage: StageView| {
        let is_entering_fs = new_stage == StageView::FullscreenNowPlaying;
        let is_leaving_fs = *nav_state.read().current() == StageView::FullscreenNowPlaying && !is_entering_fs;
        if is_entering_fs {
            dioxus::desktop::window().set_fullscreen(true);
        } else if is_leaving_fs {
            dioxus::desktop::window().set_fullscreen(false);
        }
        nav_state.write().push(new_stage);
    };

    // Helper: History back navigation
    let mut go_back = move || {
        let was_fs = *nav_state.read().current() == StageView::FullscreenNowPlaying;
        if was_fs {
            dioxus::desktop::window().set_fullscreen(false);
        }
        let mut nav = nav_state.write();
        if nav.back().is_none() {
            match nav.current().clone() {
                StageView::AlbumDetail(_) | StageView::AlbumEditor(_) => {
                    nav.push(StageView::Albums);
                }
                StageView::ArtistDetail(_) => {
                    nav.push(StageView::Artists);
                }
                StageView::ForYouDetail(_) => {
                    nav.push(StageView::ForYou);
                }
                _ => {}
            }
        }
    };

    // Helper: History forward navigation
    let mut go_forward = move || {
        nav_state.write().forward();
    };

    // Helper: Select next index using current ShuffleMode (Off, Linear, Smart) and recency history
    let pick_next_index = move |curr_idx: usize| -> Option<usize> {
        let mode = *shuffle_mode.read();
        let q = queue.read().clone();
        let all = tracks.read().clone();
        let hist = shuffle_history.read().clone();
        select_next_playback_index(mode, curr_idx, &q, &all, &hist)
    };

    // Helper: Play a specific track
    let mut play_track = {
        let player = player.clone();
        move |track: TrackEntry| {
            let p = player.read();
            let mut q = queue.read().clone();
            let curr_pos = if let Some(pos) = q.iter().position(|t| t.path == track.path) {
                pos
            } else {
                let curr = *queue_pos.read();
                if q.is_empty() {
                    q.push(track.clone());
                    0
                } else {
                    let insert_pos = (curr + 1).min(q.len());
                    q.insert(insert_pos, track.clone());
                    insert_pos
                }
            };
            queue.set(q);
            queue_pos.set(curr_pos);

            p.play_file(track.path.clone());
            current_track.set(Some(track.clone()));
            has_handled_track_end.set(false);

            if app_config.read().system_media_toast {
                let payload = attune_player_syscore_playertoast_broadcast::PlayerToastPayload {
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    album: track.album.clone(),
                    duration_ms: track.duration_ms,
                    is_playing: true,
                    artwork_path: track.artwork_path.clone(),
                };
                let _ = attune_player_syscore_playertoast_broadcast::broadcast_player_toast(&payload);
            }

            // Record track in rotation history
            shuffle_history.write().record_played(track.path.clone());

            // Update recent tracks
            let mut recents = recent_tracks.read().clone();
            recents.retain(|t| t.path != track.path);
            recents.insert(0, track.clone());
            recents.truncate(16);
            recent_tracks.set(recents);

            // Load companion lyrics
            let sheet = resolve_companion_lyrics(&track.path).unwrap_or_else(LyricsSheet::empty);
            lyrics_sheet.set(sheet);
            active_lyric_index.set(None);

            // Prepare transition to next track in queue/library if Smart Shuffle is active
            if *shuffle_mode.read() == ShuffleMode::Smart {
                if let Some(next_idx) = pick_next_index(curr_pos) {
                    let q = queue.read().clone();
                    let all = tracks.read().clone();
                    let next_track = if !q.is_empty() && next_idx < q.len() {
                        Some(&q[next_idx])
                    } else if q.is_empty() && !all.is_empty() && next_idx < all.len() {
                        Some(&all[next_idx])
                    } else {
                        None
                    };

                    if let Some(nt) = next_track {
                        p.prepare_transition(
                            nt.path.clone(),
                            nt.title.clone(),
                            nt.artist.clone(),
                        );
                    } else {
                        p.clear_pending_transition();
                    }
                } else {
                    p.clear_pending_transition();
                }
            } else {
                p.clear_pending_transition();
            }
        }
    };

    // Helper: Start playlist / album / mix
    let mut play_track_list = {
        let mut play_track = play_track.clone();
        let player = player.clone();
        move |list: Vec<TrackEntry>| {
            if list.is_empty() {
                let mut q = queue.read().clone();
                q.clear();
                queue.set(q);
                queue_pos.set(0);
                player.read().stop();
                is_playing.set(false);
                current_track.set(None);
                pending_trans_info.set(None);
                last_verdict.set(None);
                is_transition_active.set(false);
                return;
            }
            queue.set(list.clone());
            queue_pos.set(0);
            play_track(list[0].clone());
        }
    };

    // Helper: Insert track to play next in queue
    let mut play_next_in_queue = move |track: TrackEntry| {
        let mut q = queue.read().clone();
        let curr = *queue_pos.read();
        if q.is_empty() {
            q.push(track);
            queue.set(q);
        } else {
            let insert_pos = (curr + 1).min(q.len());
            if let Some(pos) = q.iter().position(|t| t.path == track.path) {
                let item = q.remove(pos);
                let new_slot = if pos < insert_pos { insert_pos.saturating_sub(1) } else { insert_pos };
                q.insert(new_slot.min(q.len()), item);
            } else {
                q.insert(insert_pos, track);
            }
            queue.set(q);
        }
    };

    // Helper: Next track
    let play_next_track = {
        let mut play_track = play_track.clone();
        let player = player.clone();
        move || {
            let q = queue.read().clone();
            let curr_pos = *queue_pos.read();
            let next_idx_opt = pick_next_index(curr_pos);

            if let Some(next_idx) = next_idx_opt {
                if !q.is_empty() && next_idx < q.len() {
                    queue_pos.set(next_idx);
                    play_track(q[next_idx].clone());
                    return;
                }
            }

            // Handle repeat mode when sequential queue ends
            if *repeat_mode.read() == RepeatMode::All && !q.is_empty() {
                queue_pos.set(0);
                play_track(q[0].clone());
            } else if q.is_empty() {
                let all_tracks = tracks.read().clone();
                if !all_tracks.is_empty() && *shuffle_mode.read() != ShuffleMode::Off {
                    let next_idx = if let Some(idx) = pick_next_index(curr_pos) {
                        idx % all_tracks.len()
                    } else {
                        0
                    };
                    play_track(all_tracks[next_idx].clone());
                }
            } else {
                player.read().pause();
                is_playing.set(false);
            }
        }
    };

    // Helper: Previous track
    let mut play_prev_track = {
        let player = player.clone();
        let mut play_track = play_track.clone();
        move || {
            let pos_ms = *playback_pos_ms.read();
            if pos_ms > 3000 {
                // Restart song if played more than 3 seconds
                player.read().seek_ms(0);
            } else {
                let curr_pos = *queue_pos.read();
                let q = queue.read().clone();
                if curr_pos > 0 && curr_pos - 1 < q.len() {
                    queue_pos.set(curr_pos - 1);
                    play_track(q[curr_pos - 1].clone());
                } else {
                    player.read().seek_ms(0);
                }
            }
        }
    };

    // Helper: Override primed transition with a chosen alternative candidate
    let mut override_candidate = {
        let player = player.clone();
        move |target_track: TrackEntry| {
            let p = player.read();
            let mut q = queue.cloned();
            let curr_pos = *queue_pos.read();
            let next_slot = (curr_pos + 1).min(q.len());

            if let Some(existing_pos) = q.iter().position(|t| t.path == target_track.path) {
                let item = q.remove(existing_pos);
                let insert_pos = if existing_pos < next_slot { next_slot.saturating_sub(1) } else { next_slot };
                q.insert(insert_pos.min(q.len()), item);
            } else {
                q.insert(next_slot, target_track.clone());
            }
            queue.set(q);

            p.prepare_transition(
                target_track.path.clone(),
                target_track.title.clone(),
                target_track.artist.clone(),
            );
            show_candidates_modal.set(false);
        }
    };

    // Helper: Trigger manual transition now
    let mut trigger_transition_now = {
        let player = player.clone();
        let mut play_next_track = play_next_track.clone();
        move || {
            let p = player.read();
            if let Some(triggered) = p.trigger_transition_now() {
                last_verdict.set(Some(triggered.clone()));
                is_transition_active.set(true);
                let q = queue.cloned();
                let all = tracks.cloned();
                let incoming = q.iter().find(|t| t.path == triggered.incoming_path).cloned()
                    .or_else(|| all.iter().find(|t| t.path == triggered.incoming_path).cloned())
                    .unwrap_or_else(|| {
                        let mut probed = TrackEntry::from_probe(&triggered.incoming_path);
                        if probed.bpm.is_none() && triggered.incoming_bpm > 10.0 {
                            probed.bpm = Some(triggered.incoming_bpm);
                        }
                        if probed.key.is_none() && !triggered.incoming_key.is_empty() {
                            probed.key = Some(triggered.incoming_key.clone());
                        }
                        probed
                    });
                transitioning_to_track.set(Some(incoming));
            } else {
                play_next_track();
            }
        }
    };

    // Background Async Poller (50ms Interval): Updates playback, auto-transitions, scan events, lyrics, Watchguard telemetry, and OS media controls (MPRIS / SMTC)
    let play_next_track_poller = play_next_track.clone();
    let play_prev_track_poller = play_prev_track.clone();
    let app_config_poller = app_config;
    let music_dir_poller = music_dir;
    let extra_music_dirs_poller = extra_music_dirs;
    let song_exclusions_poller = song_exclusions;
    let hwnd: Option<u64> = {
        #[cfg(target_os = "windows")]
        {
            use dioxus::desktop::tao::platform::windows::WindowExtWindows;
            Some(dioxus::desktop::window().window.hwnd() as usize as u64)
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    };
    use_future(move || {
        let player = player.clone();
        let scan_rx = scan_rx.clone();
        let mut play_next_track = play_next_track_poller.clone();
        let mut play_prev_track = play_prev_track_poller.clone();
        let mut app_config = app_config_poller;
        let mut music_dir = music_dir_poller;
        let mut extra_music_dirs = extra_music_dirs_poller;
        let mut song_exclusions = song_exclusions_poller;
        async move {
            let mut media_service = SystemMediaService::new(hwnd);
            let mut last_media_sync_instant = std::time::Instant::now();
            let mut last_synced_track_path: Option<PathBuf> = None;
            let mut last_synced_playing = false;
            let mut last_hw_sample_instant = std::time::Instant::now();
            let mut last_clipping_count = 0u32;
            let mut last_underrun_count = 0u32;

            loop {
                tokio::time::sleep(Duration::from_millis(50)).await;

                // 0. Process Inbound OS-level Media Player Commands (MPRIS / SMTC / Hardware Media Keys)
                while let Some(cmd) = media_service.poll_command() {
                    match cmd {
                        MediaControlCommand::Play => {
                            player.read().play();
                            is_playing.set(true);
                        }
                        MediaControlCommand::Pause => {
                            player.read().pause();
                            is_playing.set(false);
                        }
                        MediaControlCommand::TogglePlayPause => {
                            let p = player.read();
                            if *is_playing.read() {
                                p.pause();
                                is_playing.set(false);
                            } else {
                                p.play();
                                is_playing.set(true);
                            }
                        }
                        MediaControlCommand::Next => {
                            play_next_track();
                        }
                        MediaControlCommand::Previous => {
                            play_prev_track();
                        }
                        MediaControlCommand::Stop => {
                            player.read().stop();
                            is_playing.set(false);
                        }
                        MediaControlCommand::SeekToMs(pos) => {
                            player.read().seek_ms(pos);
                            playback_pos_ms.set(pos);
                        }
                        MediaControlCommand::SeekBySec(offset) => {
                            let cur = *playback_pos_ms.read() as f64 / 1000.0;
                            let target = (cur + offset).max(0.0);
                            let ms = (target * 1000.0) as u64;
                            player.read().seek_ms(ms);
                            playback_pos_ms.set(ms);
                        }
                        MediaControlCommand::SetVolume(vol) => {
                            let db = if vol <= 0.001 { -60.0 } else { 20.0 * (vol as f32).log10() };
                            player.read().set_gain_db(db.clamp(-60.0, 6.0));
                        }
                        MediaControlCommand::SetShuffle(enable) => {
                            let next_mode = if enable {
                                if *shuffle_mode.read() == ShuffleMode::Off {
                                    ShuffleMode::Smart
                                } else {
                                    *shuffle_mode.read()
                                }
                            } else {
                                ShuffleMode::Off
                            };
                            shuffle_mode.set(next_mode);
                        }
                        MediaControlCommand::SetRepeat(rep) => {
                            let next_mode = match rep {
                                SystemRepeatMode::Off => RepeatMode::Off,
                                SystemRepeatMode::One => RepeatMode::One,
                                SystemRepeatMode::All => RepeatMode::All,
                            };
                            repeat_mode.set(next_mode);
                        }
                        MediaControlCommand::Raise => {
                            #[cfg(target_os = "linux")]
                            dioxus::desktop::window().window.set_focus();
                        }
                        MediaControlCommand::Quit => {
                            std::process::exit(0);
                        }
                    }
                }

                // Record real hardware resource telemetry and sync config every 1000ms on desktop
                if last_hw_sample_instant.elapsed() >= Duration::from_millis(1000) {
                    last_hw_sample_instant = std::time::Instant::now();

                    // Sync settings modified in standalone settings window
                    let fresh_cfg = config::load_config();
                    if *app_config.read() != fresh_cfg {
                        if fresh_cfg.music_dir != *music_dir.read() {
                            music_dir.set(fresh_cfg.music_dir.clone());
                        }
                        if fresh_cfg.extra_music_dirs != *extra_music_dirs.read() {
                            extra_music_dirs.set(fresh_cfg.extra_music_dirs.clone());
                        }
                        if fresh_cfg.song_exclusions != *song_exclusions.read() {
                            song_exclusions.set(fresh_cfg.song_exclusions.clone());
                        }
                        app_config.set(fresh_cfg);
                    }

                    if let Ok(m) = crate::process_manager::sample_metrics() {
                        let gpu_usage = if let (Some(cur), Some(max)) = (m.gpu_cur_freq_mhz, m.gpu_max_freq_mhz) {
                            if max > 0 { (cur as f32 / max as f32) * 100.0 } else { 0.0 }
                        } else {
                            0.0
                        };
                        let _ = crate::dynamic_components::watchguard::record_hardware_resources(
                            m.cpu_usage_pct,
                            m.rss_bytes,
                            gpu_usage,
                            m.thread_count,
                        );
                    }
                }

                // 1. Process Library Scan Events
                let rx_arc = scan_rx.read().clone();
                let rx_guard = rx_arc.lock().ok();
                if let Some(guard) = rx_guard {
                    if let Some(rx) = guard.as_ref() {
                        while let Ok(event) = rx.try_recv() {
                            match event {
                                ScanEvent::Discovered(total) => {
                                    scan_progress.set((0, total));
                                }
                                ScanEvent::Track(track) => {
                                    let mut curr_tracks = tracks.read().clone();
                                    last_scanned_file.set(track.title.clone());
                                    curr_tracks.push(*track);
                                    let (count, total) = *scan_progress.read();
                                    scan_progress.set((count + 1, total));
                                    tracks.set(curr_tracks);
                                }
                                ScanEvent::Finished => {
                                    is_scanning.set(false);
                                    // Populate queue if empty
                                    if queue.read().is_empty() && !tracks.read().is_empty() {
                                        queue.set(tracks.read().clone());
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Process Audio Playback Snapshot
                let p = player.read();
                if let Some(snap) = p.snapshot() {
                    is_playing.set(snap.is_playing);

                    // Feed real audio signal & underrun/clipping events into Watchguard dylib
                    let new_clipping = snap.clipping_count > last_clipping_count;
                    let new_underrun = snap.underrun_count > last_underrun_count;
                    last_clipping_count = snap.clipping_count;
                    last_underrun_count = snap.underrun_count;
                    if snap.is_playing || new_clipping || new_underrun {
                        let _ = crate::dynamic_components::watchguard::record_audio_event(
                            snap.peak_amplitude,
                            new_underrun,
                            new_clipping,
                        );
                    }

                    // Determine active deck position and duration
                    let (pos, dur) = if snap.active_deck_str == "Deck B" {
                        (snap.deck_b.position_ms, snap.deck_b.duration_ms)
                    } else {
                        (snap.deck_a.position_ms, snap.deck_a.duration_ms)
                    };
                    playback_pos_ms.set(pos);
                    playback_dur_ms.set(dur);

                    // Synchronize Playback State to OS (MPRIS / SMTC) every 250ms or immediately on track/play change
                    let curr_opt = current_track.read().clone();
                    let curr_path = curr_opt.as_ref().map(|t| t.path.clone());
                    let track_or_play_changed = curr_path != last_synced_track_path || snap.is_playing != last_synced_playing;
                    if track_or_play_changed || last_media_sync_instant.elapsed() >= Duration::from_millis(250) {
                        last_media_sync_instant = std::time::Instant::now();
                        last_synced_track_path = curr_path;
                        last_synced_playing = snap.is_playing;

                        let track_meta = curr_opt.map(|t| {
                            SystemTrackMetadata {
                                id: t.path.to_string_lossy().to_string(),
                                title: t.title.clone(),
                                artists: vec![t.artist.clone()],
                                album: t.album.clone(),
                                artwork_path: t.artwork_path.clone(),
                                artwork_data_url: t.artwork_data_url.clone(),
                                url: format!("file://{}", t.path.display()),
                            }
                        });

                        let sys_repeat = match *repeat_mode.read() {
                            RepeatMode::Off => SystemRepeatMode::Off,
                            RepeatMode::One => SystemRepeatMode::One,
                            RepeatMode::All => SystemRepeatMode::All,
                        };

                        let q_len = queue.read().len();
                        let sys_state = SystemPlaybackState {
                            track: track_meta,
                            is_playing: snap.is_playing,
                            position_ms: pos,
                            duration_ms: dur,
                            volume: 1.0,
                            shuffle: *shuffle_mode.read() != ShuffleMode::Off,
                            repeat: sys_repeat,
                            can_go_next: q_len > 0,
                            can_go_previous: q_len > 0 || pos > 3000,
                            can_seek: dur > 0,
                        };
                        media_service.update_state(&sys_state);
                    }

                    if snap.is_playing && pos + 1500 < dur {
                        has_handled_track_end.set(false);
                    }

                    // Dynamic BPM & Key synchronization from AudioProfiler
                    let maybe_curr = current_track.cloned();
                    if let Some(mut curr) = maybe_curr {
                        let need_bpm = curr.bpm.map(|b| b < 10.0).unwrap_or(true);
                        let need_key = curr.key.as_deref().map(|k| k.is_empty() || k == "—" || k == "Unknown").unwrap_or(true);
                        if need_bpm || need_key {
                            if let Some((bpm, key)) = p.current_profile() {
                                let mut changed = false;
                                if need_bpm && bpm > 10.0 {
                                    curr.bpm = Some(bpm);
                                    changed = true;
                                }
                                if need_key && !key.is_empty() && key != "Unknown" {
                                    curr.key = Some(key.clone());
                                    changed = true;
                                }
                                if changed {
                                    current_track.set(Some(curr.clone()));
                                    let mut all = tracks.cloned();
                                    if let Some(t) = all.iter_mut().find(|t| t.path == curr.path) {
                                        t.bpm = curr.bpm;
                                        t.key = curr.key.clone();
                                        tracks.set(all);
                                    }
                                }
                            }
                        }
                    }

                    if let Some((_, _, pct, archetype)) = &snap.transition_info {
                        is_transition_active.set(true);
                        active_trans_name.set(Some(format!("Blending: {archetype} ({:.0}%)", pct * 100.0)));
                    } else {
                        active_trans_name.set(None);
                    }

                    // 3. Check for Automatic JIT Transition Trigger
                    if !p.is_transitioning() {
                        if let Some(triggered) = p.check_auto_transition() {
                            last_verdict.set(Some(triggered.clone()));
                            is_transition_active.set(true);
                            let q = queue.cloned();
                            let all = tracks.cloned();
                            let incoming = q.iter().find(|t| t.path == triggered.incoming_path).cloned()
                                .or_else(|| all.iter().find(|t| t.path == triggered.incoming_path).cloned())
                                .unwrap_or_else(|| {
                                    let mut probed = TrackEntry::from_probe(&triggered.incoming_path);
                                    if probed.bpm.is_none() && triggered.incoming_bpm > 10.0 {
                                        probed.bpm = Some(triggered.incoming_bpm);
                                    }
                                    if probed.key.is_none() && !triggered.incoming_key.is_empty() {
                                        probed.key = Some(triggered.incoming_key.clone());
                                    }
                                    probed
                                });
                            transitioning_to_track.set(Some(incoming));
                        }
                    }

                    // 4. Handle Transition Completion or Natural Track End
                    if let Some(completed) = p.poll_transition_completed() {
                        // Transition officially concluded in DualDeckMixer (Deck B swapped into Deck A)
                        is_transition_active.set(false);
                        active_trans_name.set(None);
                        let q = queue.cloned();
                        let all = tracks.cloned();
                        let curr_pos = *queue_pos.read();

                        let (next_idx, mut new_track) = if let Some(idx) = q.iter().position(|t| t.path == completed.incoming_path) {
                            (idx, q[idx].clone())
                        } else if q.is_empty() {
                            if let Some(t) = all.iter().find(|t| t.path == completed.incoming_path) {
                                (0, t.clone())
                            } else {
                                let probed = TrackEntry::from_probe(&completed.incoming_path);
                                (0, probed)
                            }
                        } else {
                            (curr_pos.min(q.len().saturating_sub(1)), current_track.read().clone().unwrap_or_else(|| q[0].clone()))
                        };

                        queue_pos.set(next_idx);
                        if new_track.bpm.is_none() && completed.incoming_bpm > 10.0 {
                            new_track.bpm = Some(completed.incoming_bpm);
                        }
                        if (new_track.key.is_none() || new_track.key.as_deref() == Some("Unknown") || new_track.key.as_deref() == Some("—")) && !completed.incoming_key.is_empty() && completed.incoming_key != "Unknown" {
                            new_track.key = Some(completed.incoming_key.clone());
                        }
                        p.set_current_profile(completed.incoming_bpm, completed.incoming_key.clone());
                        current_track.set(Some(new_track.clone()));
                        if app_config.read().system_media_toast {
                            let payload = attune_player_syscore_playertoast_broadcast::PlayerToastPayload {
                                title: new_track.title.clone(),
                                artist: new_track.artist.clone(),
                                album: new_track.album.clone(),
                                duration_ms: new_track.duration_ms,
                                is_playing: true,
                                artwork_path: new_track.artwork_path.clone(),
                            };
                            let _ = attune_player_syscore_playertoast_broadcast::broadcast_player_toast(&payload);
                        }
                        shuffle_history.write().record_played(new_track.path.clone());
                        let sheet = resolve_companion_lyrics(&new_track.path).unwrap_or_else(LyricsSheet::empty);
                        lyrics_sheet.set(sheet);
                        active_lyric_index.set(None);
                        transitioning_to_track.set(None);
                        has_handled_track_end.set(false);

                        // Pre-plan next transition for the new incoming track
                        if *shuffle_mode.read() == ShuffleMode::Smart {
                            if let Some(upcoming_idx) = pick_next_index(next_idx) {
                                let cand_track = if upcoming_idx < q.len() {
                                    Some(&q[upcoming_idx])
                                } else if q.is_empty() && upcoming_idx < all.len() {
                                    Some(&all[upcoming_idx])
                                } else {
                                    None
                                };
                                if let Some(ct) = cand_track {
                                    p.prepare_transition(
                                        ct.path.clone(),
                                        ct.title.clone(),
                                        ct.artist.clone(),
                                    );
                                } else {
                                    p.clear_pending_transition();
                                }
                            } else {
                                p.clear_pending_transition();
                            }
                        }
                    } else if !p.is_transitioning() {
                        // Natural track completion auto-advance check (no transition occurred)
                        let is_deck_finished = snap.deck_a.state_str == "Finished";
                        let is_at_end = dur > 3000 && pos >= dur.saturating_sub(400);
                        let is_not_playing = !snap.is_playing;
                        let is_ready = p.load_state() == LoadState::Ready;

                        if (is_deck_finished || (is_at_end && is_not_playing)) && is_ready {
                            if !*has_handled_track_end.read() {
                                has_handled_track_end.set(true);
                                if *repeat_mode.read() == RepeatMode::One {
                                    p.seek_ms(0);
                                    p.play();
                                } else {
                                    play_next_track();
                                }
                            }
                        }
                    }
                } else if last_synced_playing || last_synced_track_path.is_some() {
                    last_synced_playing = false;
                    last_synced_track_path = None;
                    media_service.update_state(&SystemPlaybackState::default());
                }

                // 5. Update Pending Transition Info
                pending_trans_info.set(p.pending_transition());

                // 6. Update Synchronized Lyrics Active Line
                let pos = *playback_pos_ms.read();
                let active_idx = lyrics_sheet.read().active_line_index(pos);
                if active_idx != *active_lyric_index.read() {
                    active_lyric_index.set(active_idx);
                }
            }
        }
    });

    // Compute harmonic matches for the currently playing track using Camelot wheel
    let harmonic_matches: Vec<TrackEntry> = {
        let all = tracks.read().clone();
        if let Some(curr) = current_track.read().as_ref() {
            if let Some(curr_raw_key) = curr.key.as_deref().filter(|k| !k.is_empty() && *k != "—" && *k != "Unknown") {
                let curr_camelot = smart_shuffle::normalize_to_camelot(curr_raw_key)
                    .unwrap_or_else(|| curr_raw_key.to_string());
                all.into_iter()
                    .filter(|t| {
                        if t.path == curr.path {
                            return false;
                        }
                        if let Some(cand_raw_key) = t.key.as_deref().filter(|k| !k.is_empty() && *k != "—" && *k != "Unknown") {
                            let cand_camelot = smart_shuffle::normalize_to_camelot(cand_raw_key)
                                .unwrap_or_else(|| cand_raw_key.to_string());
                            let (compat, _) = smart_shuffle::camelot_compatibility(&curr_camelot, &cand_camelot);
                            compat >= 0.70
                        } else {
                            false
                        }
                    })
                    .take(6)
                    .collect()
            } else {
                all.into_iter().filter(|t| t.path != curr.path).take(6).collect()
            }
        } else {
            all.into_iter().take(6).collect()
        }
    };

    // Calculate time labels and percentages for scrubber & volume
    let elapsed_str = fmt_ms(*playback_pos_ms.read());
    let rem_ms = playback_dur_ms.read().saturating_sub(*playback_pos_ms.read());
    let remaining_str = format!("-{}", fmt_ms(rem_ms));
    let progress_pct = if *playback_dur_ms.read() > 0 {
        ((*playback_pos_ms.read() as f64) / (*playback_dur_ms.read() as f64) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let vol_pct = (*volume_pct.read()).clamp(0, 100);
    let (scan_curr, scan_total) = *scan_progress.read();
    let extra_sources_count = extra_music_dirs.read().len();

    let curr_track_clone = current_track.read().clone();
    let mini_pill_info = curr_track_clone.as_ref().map(|track| {
        let art_src = track.artwork_data_url.clone().unwrap_or_default();
        (track.title.clone(), format!("{} • {}", track.artist, track.album), art_src)
    });

    rsx! {
        div {
            class: if app_config.read().auto_scroll_long_text {
                "app-container text-moving-enabled"
            } else {
                "app-container text-moving-disabled"
            },
            tabindex: "0",
            onkeydown: move |e| {
                let cfg = app_config.read();
                if !cfg.enable_shortcuts {
                    return;
                }
                if cfg.alt_shortcut_prefix && !e.modifiers().alt() {
                    return;
                }
                match e.key() {
                    Key::Character(ref c) if c == " " => {
                        let p = player.read();
                        if *is_playing.read() { p.pause(); } else { p.play(); }
                    }
                    Key::ArrowLeft => {
                        let curr = *playback_pos_ms.read();
                        player.read().seek_ms(curr.saturating_sub(5000));
                    }
                    Key::ArrowRight => {
                        let curr = *playback_pos_ms.read();
                        let dur = *playback_dur_ms.read();
                        player.read().seek_ms((curr + 5000).min(dur));
                    }
                    Key::Escape => {
                        if *nav_state.read().current() == StageView::FullscreenNowPlaying {
                            dioxus::desktop::window().set_fullscreen(false);
                            nav_state.write().back();
                        }
                    }
                    _ => {}
                }
            },
            div { class: "ambient-mesh" }

            // ==========================================
            // LEFT SIDEBAR
            // ==========================================
            div { class: "sidebar",
                // Brand Header
                div { style: "display: flex; align-items: center; gap: 10px; padding: 6px 8px 12px 8px;",
                    img {
                        src: "{app_icon_data_url()}",
                        style: "width: 28px; height: 28px; border-radius: 7px; object-fit: contain; filter: drop-shadow(0 2px 6px rgba(45,212,191,0.35));",
                        alt: "Attune Logo",
                    }
                    span { style: "font-weight: 700; font-size: 16px; letter-spacing: 0.05em;", "ATTUNE" }
                }

                // Global Search Input
                div { class: "search-bar",
                    span { class: "material-icons md-18", "search" }
                    input {
                        placeholder: "{strings.search_library_placeholder}",
                        value: "{search_query}",
                        oninput: move |evt| {
                            search_query.set(evt.value());
                            if *nav_state.read().current() != StageView::Library {
                                navigate_to(StageView::Library);
                            }
                        },
                    }
                }

                // Attune Main Navigation
                div { class: "nav-section",
                    span { class: "section-label", "{strings.app_name.to_uppercase()}" }
                    div {
                        class: if *nav_state.read().current() == StageView::Home { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Home),
                        span { class: "material-icons md-20", "home" }
                        span { "{strings.home}" }
                    }
                    div {
                        class: if matches!(nav_state.read().current(), StageView::ForYou | StageView::ForYouDetail(_)) { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::ForYou),
                        span { class: "material-icons md-20", "auto_awesome" }
                        span { "{strings.for_you}" }
                    }
                    div {
                        class: if *nav_state.read().current() == StageView::Library { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Library),
                        span { class: "material-icons md-20", "library_music" }
                        span { "{strings.library}" }
                    }
                    div {
                        class: if matches!(nav_state.read().current(), StageView::Albums | StageView::AlbumDetail(_)) { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Albums),
                        span { class: "material-icons md-20", "album" }
                        span { "{strings.albums}" }
                    }
                    div {
                        class: if matches!(nav_state.read().current(), StageView::Artists | StageView::ArtistDetail(_)) { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Artists),
                        span { class: "material-icons md-20", "person" }
                        span { "{strings.artists}" }
                    }
                    div {
                        class: if *nav_state.read().current() == StageView::Favorites { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Favorites),
                        span { class: "material-icons md-20", "favorite" }
                        span { "{strings.favorite_songs}" }
                    }
                }

                // Sources Section
                div { class: "nav-section", style: "margin-top: 8px;",
                    span { class: "section-label", "{strings.sources}" }
                    div { style: "display: flex; align-items: center; gap: 6px; padding: 6px 8px; font-size: 11.5px; color: var(--text-muted); word-break: break-all; background: rgba(255,255,255,0.03); border-radius: 6px; border: 1px solid var(--border-glass); margin-bottom: 8px;",
                        span { class: "material-icons md-16", "folder" }
                        if let Some(dir) = music_dir.read().as_ref() {
                            if extra_sources_count > 0 {
                                span { "{dir.file_name().and_then(|s| s.to_str()).unwrap_or(\"Music\")} (+{extra_sources_count} more)" }
                            } else {
                                span { "{dir.display()}" }
                            }
                        } else {
                            span { "{strings.no_folder_chosen}" }
                        }
                    }
                    div { style: "display: flex; gap: 6px;",
                        button {
                            class: "btn-pill",
                            style: "flex: 1; padding: 4px 8px; font-size: 11.5px;",
                            onclick: {
                                let mut scan_all = scan_all_libraries.clone();
                                move |_| {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        config::add_music_dir(path.clone());
                                        if music_dir.read().is_none() {
                                            music_dir.set(Some(path));
                                        } else {
                                            let mut extras = extra_music_dirs.read().clone();
                                            if !extras.contains(&path) {
                                                extras.push(path);
                                                extra_music_dirs.set(extras);
                                            }
                                        }
                                        scan_all();
                                    }
                                }
                            },
                            "{strings.add_folder}"
                        }
                        button {
                            class: "btn-pill",
                            style: "flex: 1; padding: 4px 8px; font-size: 11.5px;",
                            disabled: music_dir.read().is_none() || *is_scanning.read(),
                            onclick: {
                                let mut scan_all = scan_all_libraries.clone();
                                move |_| scan_all()
                            },
                            if *is_scanning.read() { "{strings.scanning}" } else { "{strings.rescan}" }
                        }
                    }
                }

                // Sidebar Bottom
                div { class: "sidebar-bottom",
                    div {
                        class: if *nav_state.read().current() == StageView::Equalizer { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Equalizer),
                        span { class: "material-icons md-20", "tune" }
                        span { "{strings.equalizer}" }
                    }
                    div {
                        class: "nav-item",
                        title: "Open Settings in New Window",
                        onclick: move |_| open_settings_window(),
                        span { class: "material-icons md-20", "settings" }
                        span { "{strings.settings}" }
                    }
                    div {
                        class: if *nav_state.read().current() == StageView::ProcessManager { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::ProcessManager),
                        span { class: "material-icons md-20", "analytics" }
                        span { "{strings.process_manager}" }
                    }
                    div {
                        class: if *nav_state.read().current() == StageView::About { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::About),
                        span { class: "material-icons md-20", "info" }
                        span { "{strings.about_attune}" }
                    }
                }
            }

            // ==========================================
            // MAIN STAGE
            // ==========================================
            div { class: "main-stage",
                // Top Navigation Bar
                div { class: "top-nav",
                    // History Back / Forward
                    div { class: "history-nav",
                        button {
                            class: "btn-icon",
                            disabled: !nav_state.read().can_go_back(),
                            onclick: move |_| go_back(),
                            span { class: "material-icons md-18", "chevron_left" }
                        }
                        button {
                            class: "btn-icon",
                            disabled: !nav_state.read().can_go_forward(),
                            onclick: move |_| go_forward(),
                            span { class: "material-icons md-18", "chevron_right" }
                        }
                    }

                    // Scan Status or Active Transition Telemetry
                    div { style: "display: flex; align-items: center; gap: 10px;",
                        if *is_scanning.read() {
                            div { class: "quality-badge quality-good",
                                span { class: "material-icons md-14", style: "margin-right: 4px; vertical-align: -2px;", "sync" }
                                "Scanning {scan_curr}/{scan_total}: {last_scanned_file.read()}"
                            }
                        } else if let Some(trans) = active_trans_name.read().as_ref() {
                            div { class: "quality-badge quality-very-good",
                                span { class: "material-icons md-14", style: "margin-right: 4px; vertical-align: -2px;", "auto_awesome" }
                                "{trans}"
                            }
                        }
                    }

                    // Top Right Tools
                    div { class: "top-tools",
                        button {
                            class: if *dock_mode.read() == DockMode::Lyrics { "btn-pill active" } else { "btn-pill" },
                            onclick: move |_| {
                                if *dock_mode.read() == DockMode::Lyrics {
                                    dock_mode.set(DockMode::Closed);
                                } else {
                                    dock_mode.set(DockMode::Lyrics);
                                }
                            },
                            span { class: "material-icons md-16", style: "margin-right: 4px;", "subtitles" }
                            span { "{strings.lyrics}" }
                        }
                        button {
                            class: if *dock_mode.read() == DockMode::Queue { "btn-pill active" } else { "btn-pill" },
                            onclick: move |_| {
                                if *dock_mode.read() == DockMode::Queue {
                                    dock_mode.set(DockMode::Closed);
                                } else {
                                    dock_mode.set(DockMode::Queue);
                                }
                            },
                            span { class: "material-icons md-16", style: "margin-right: 4px;", "queue_music" }
                            span { "{strings.queue}" }
                        }
                        button {
                            class: if *nav_state.read().current() == StageView::Equalizer { "btn-icon active" } else { "btn-icon" },
                            title: "Master Equalizer Rack",
                            onclick: move |_| navigate_to(StageView::Equalizer),
                            span { class: "material-icons md-18", "tune" }
                        }
                    }
                }

                // Stage View Content Scroll
                div { class: "content-scroll",
                    match nav_state.read().current().clone() {
                        StageView::Home => rsx! {
                            Home {
                                current_track: current_track.read().clone(),
                                is_playing: *is_playing.read(),
                                playback_pos_ms: *playback_pos_ms.read(),
                                playback_dur_ms: *playback_dur_ms.read(),
                                recent_tracks: recent_tracks.read().clone(),
                                harmonic_matches: harmonic_matches.clone(),
                                all_tracks: tracks.read().clone(),
                                home_subtext: app_config.read().home_subtext.clone(),
                                on_play_track: move |t| play_track(t),
                                on_play_tracks: move |list| play_track_list(list),
                                on_queue_track: move |t| queue.write().push(t),
                                on_select_artist: move |a| navigate_to(StageView::ArtistDetail(a)),
                                on_toggle_play: move |_| {
                                    let p = player.read();
                                    if *is_playing.read() { p.pause(); } else { p.play(); }
                                },
                            }
                        },
                        StageView::Library => rsx! {
                            Library {
                                tracks: tracks.read().clone(),
                                search_query: search_query.read().clone(),
                                current_track_path: current_track.read().as_ref().map(|t| t.path.clone()),
                                favorites: favorites.read().clone(),
                                on_play_track: move |t| play_track(t),
                                on_queue_track: move |t| {
                                    let mut q = queue.read().clone();
                                    q.push(t);
                                    queue.set(q);
                                },
                                on_toggle_favorite: move |path: PathBuf| {
                                    let mut favs = favorites.read().clone();
                                    if favs.contains(&path) { favs.remove(&path); } else { favs.insert(path); }
                                    favorites.set(favs);
                                },
                                on_search_change: move |val| search_query.set(val),
                            }
                        },
                        StageView::Albums => rsx! {
                            Albums {
                                tracks: tracks.read().clone(),
                                selected_album_name: None,
                                current_track_path: current_track.read().as_ref().map(|t| t.path.clone()),
                                is_playing: *is_playing.read(),
                                favorites: favorites.read().clone(),
                                on_select_album: move |name: String| navigate_to(StageView::AlbumDetail(name)),
                                on_select_artist: move |artist: String| navigate_to(StageView::ArtistDetail(artist)),
                                on_back: move |_| go_back(),
                                on_play_track: move |t| play_track(t),
                                on_play_tracks: move |list| play_track_list(list),
                                on_queue_track: move |t| queue.write().push(t),
                                on_play_next: move |t| play_next_in_queue(t),
                                on_toggle_favorite: move |path: PathBuf| {
                                    let mut favs = favorites.read().clone();
                                    if favs.contains(&path) { favs.remove(&path); } else { favs.insert(path); }
                                    favorites.set(favs);
                                },
                                on_toggle_play: move |_| {
                                    let p = player.read();
                                    if *is_playing.read() { p.pause(); } else { p.play(); }
                                },
                                on_edit_album: move |group| navigate_to(StageView::AlbumEditor(group)),
                            }
                        },
                        StageView::AlbumDetail(name) => rsx! {
                            Albums {
                                tracks: tracks.read().clone(),
                                selected_album_name: Some(name.clone()),
                                current_track_path: current_track.read().as_ref().map(|t| t.path.clone()),
                                is_playing: *is_playing.read(),
                                favorites: favorites.read().clone(),
                                on_select_album: move |name: String| navigate_to(StageView::AlbumDetail(name)),
                                on_select_artist: move |artist: String| navigate_to(StageView::ArtistDetail(artist)),
                                on_back: move |_| go_back(),
                                on_play_track: move |t| play_track(t),
                                on_play_tracks: move |list| play_track_list(list),
                                on_queue_track: move |t| queue.write().push(t),
                                on_play_next: move |t| play_next_in_queue(t),
                                on_toggle_favorite: move |path: PathBuf| {
                                    let mut favs = favorites.read().clone();
                                    if favs.contains(&path) { favs.remove(&path); } else { favs.insert(path); }
                                    favorites.set(favs);
                                },
                                on_toggle_play: move |_| {
                                    let p = player.read();
                                    if *is_playing.read() { p.pause(); } else { p.play(); }
                                },
                                on_edit_album: move |group| navigate_to(StageView::AlbumEditor(group)),
                            }
                        },
                        StageView::Artists => rsx! {
                            Artists {
                                tracks: tracks.read().clone(),
                                on_select_artist: move |name: String| navigate_to(StageView::ArtistDetail(name)),
                            }
                        },
                        StageView::ArtistDetail(name) => rsx! {
                            ArtistDetail {
                                artist_name: name.clone(),
                                tracks: tracks.read().clone(),
                                current_track_path: current_track.read().as_ref().map(|t| t.path.clone()),
                                is_playing: *is_playing.read(),
                                favorites: favorites.read().clone(),
                                on_select_album: move |name: String| navigate_to(StageView::AlbumDetail(name)),
                                on_select_artist: move |a: String| navigate_to(StageView::ArtistDetail(a)),
                                on_play_track: move |t| play_track(t),
                                on_play_tracks: move |list| play_track_list(list),
                                on_queue_track: move |t| queue.write().push(t),
                                on_play_next: move |t| play_next_in_queue(t),
                                on_toggle_favorite: move |path: PathBuf| {
                                    let mut favs = favorites.read().clone();
                                    if favs.contains(&path) { favs.remove(&path); } else { favs.insert(path); }
                                    favorites.set(favs);
                                },
                                on_toggle_play: move |_| {
                                    let p = player.read();
                                    if *is_playing.read() { p.pause(); } else { p.play(); }
                                },
                            }
                        },
                        StageView::AlbumEditor(group) => rsx! {
                            AlbumEditor {
                                group,
                                on_back: move |_| go_back(),
                                on_saved: move |_| {
                                    if let Some(dir) = music_dir.read().clone() {
                                        tracks.set(Vec::new());
                                        let rx = library::spawn_scan(dir);
                                        *scan_rx.write() = Arc::new(Mutex::new(Some(rx)));
                                        is_scanning.set(true);
                                    }
                                },
                            }
                        },
                        StageView::Equalizer => rsx! {
                            Equalizer {
                                params: master_fx.read().clone(),
                                on_change: move |new_params: MasterFxParams| {
                                    master_fx.set(new_params.clone());
                                    player.read().set_master_fx_params(new_params);
                                },
                            }
                        },
                        StageView::ForYou => rsx! {
                            ForYou {
                                tracks: tracks.read().clone(),
                                selected_mix_id: None,
                                current_track_path: current_track.read().as_ref().map(|t| t.path.clone()),
                                is_playing: *is_playing.read(),
                                favorites: favorites.read().clone(),
                                on_select_mix: move |id: String| navigate_to(StageView::ForYouDetail(id)),
                                on_back: move |_| go_back(),
                                on_start_mix: move |list| play_track_list(list),
                                on_play_track: move |t| play_track(t),
                                on_queue_track: move |t| queue.write().push(t),
                                on_toggle_favorite: move |path: PathBuf| {
                                    let mut favs = favorites.read().clone();
                                    if favs.contains(&path) { favs.remove(&path); } else { favs.insert(path); }
                                    favorites.set(favs);
                                },
                                on_toggle_play: move |_| {
                                    let p = player.read();
                                    if *is_playing.read() { p.pause(); } else { p.play(); }
                                },
                            }
                        },
                        StageView::ForYouDetail(id) => rsx! {
                            ForYou {
                                tracks: tracks.read().clone(),
                                selected_mix_id: Some(id.clone()),
                                current_track_path: current_track.read().as_ref().map(|t| t.path.clone()),
                                is_playing: *is_playing.read(),
                                favorites: favorites.read().clone(),
                                on_select_mix: move |id: String| navigate_to(StageView::ForYouDetail(id)),
                                on_back: move |_| go_back(),
                                on_start_mix: move |list| play_track_list(list),
                                on_play_track: move |t| play_track(t),
                                on_queue_track: move |t| queue.write().push(t),
                                on_toggle_favorite: move |path: PathBuf| {
                                    let mut favs = favorites.read().clone();
                                    if favs.contains(&path) { favs.remove(&path); } else { favs.insert(path); }
                                    favorites.set(favs);
                                },
                                on_toggle_play: move |_| {
                                    let p = player.read();
                                    if *is_playing.read() { p.pause(); } else { p.play(); }
                                },
                            }
                        },
                        StageView::Favorites => rsx! {
                            Favorites {
                                tracks: tracks.read().clone(),
                                favorites: favorites.read().clone(),
                                current_track_path: current_track.read().as_ref().map(|t| t.path.clone()),
                                on_play_track: move |t| play_track(t),
                                on_play_tracks: move |list| play_track_list(list),
                                on_queue_track: move |t| {
                                    let mut q = queue.read().clone();
                                    q.push(t);
                                    queue.set(q);
                                },
                                on_toggle_favorite: move |path: PathBuf| {
                                    let mut favs = favorites.read().clone();
                                    if favs.contains(&path) { favs.remove(&path); } else { favs.insert(path); }
                                    favorites.set(favs);
                                },
                            }
                        },
                        StageView::Setup => rsx! {
                            SetupStage {
                                initial_dir: music_dir.read().clone(),
                                on_complete: move |scanned_tracks: Vec<TrackEntry>| {
                                    tracks.set(scanned_tracks.clone());
                                    if queue.read().is_empty() {
                                        queue.set(scanned_tracks);
                                    }
                                    navigate_to(StageView::Home);
                                }
                            }
                        },
                        StageView::ProcessManager => rsx! {
                            ProcessManager {}
                        },
                        StageView::About => rsx! {
                            About {}
                        },
                        StageView::FullscreenNowPlaying => {
                            let curr_track = current_track.read().clone();
                            let is_p = *is_playing.read();
                            let pos_ms = *playback_pos_ms.read();
                            let dur_ms = *playback_dur_ms.read();
                            let sheet = lyrics_sheet.read().clone();
                            let active_idx = *active_lyric_index.read();
                            let s_mode = *shuffle_mode.read();
                            let r_mode = *repeat_mode.read();
                            let cfg = app_config.read().clone();

                            rsx! {
                                FullscreenNowPlaying {
                                    current_track: curr_track,
                                    is_playing: is_p,
                                    position_ms: pos_ms,
                                    duration_ms: dur_ms,
                                    shuffle_mode: s_mode,
                                    repeat_mode: r_mode,
                                    lyrics_sheet: sheet,
                                    active_lyric_index: active_idx,
                                    config: cfg,
                                    on_toggle_play: move |_| {
                                        let p = player.read();
                                        if *is_playing.read() { p.pause(); } else { p.play(); }
                                    },
                                    on_seek: move |ms| {
                                        let p = player.read();
                                        p.seek_ms(ms);
                                    },
                                    on_prev: move |_| {
                                        let q = queue.read();
                                        if !q.is_empty() {
                                            let curr = *queue_pos.read();
                                            let prev_idx = if curr > 0 { curr - 1 } else { q.len() - 1 };
                                            let track = q[prev_idx].clone();
                                            drop(q);
                                            play_track(track);
                                        }
                                    },
                                    on_next: move |_| {
                                        let q = queue.read();
                                        if !q.is_empty() {
                                            let curr = *queue_pos.read();
                                            let next_idx = (curr + 1) % q.len();
                                            let track = q[next_idx].clone();
                                            drop(q);
                                            play_track(track);
                                        }
                                    },
                                    on_cycle_shuffle: move |_| {
                                        let next = match *shuffle_mode.read() {
                                            ShuffleMode::Off => ShuffleMode::Linear,
                                            ShuffleMode::Linear => ShuffleMode::Smart,
                                            ShuffleMode::Smart => ShuffleMode::Off,
                                        };
                                        shuffle_mode.set(next);
                                    },
                                    on_cycle_repeat: move |_| {
                                        let next_mode = match *repeat_mode.read() {
                                            RepeatMode::Off => RepeatMode::All,
                                            RepeatMode::All => RepeatMode::One,
                                            RepeatMode::One => RepeatMode::Off,
                                        };
                                        repeat_mode.set(next_mode);
                                    },
                                    on_exit: move |_| {
                                        dioxus::desktop::window().set_fullscreen(false);
                                        nav_state.write().back();
                                    },
                                }
                            }
                        },
                    }
                }
            }

            // ==========================================
            // RIGHT DOCK (LYRICS / UP NEXT QUEUE)
            // ==========================================
            if *dock_mode.read() != DockMode::Closed {
                div { class: "right-dock",
                    match *dock_mode.read() {
                        DockMode::Lyrics => rsx! {
                            div { class: "dock-header",
                                span { style: "font-weight: 700; font-size: 15px;", "{strings.synchronized_lyrics}" }
                                div { style: "display: flex; gap: 8px; align-items: center;",
                                    button {
                                        class: if *auto_scroll_lyrics.read() { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 11px; padding: 2px 8px; display: inline-flex; align-items: center; gap: 4px;",
                                        onclick: move |_| {
                                            let curr = *auto_scroll_lyrics.read();
                                            auto_scroll_lyrics.set(!curr);
                                        },
                                        if *auto_scroll_lyrics.read() {
                                            span { class: "material-icons md-14", "lock" }
                                            span { "{strings.auto_scroll}" }
                                        } else {
                                            span { class: "material-icons md-14", "lock_open" }
                                            span { "Free-Scroll" }
                                        }
                                    }
                                    button {
                                        class: "btn-icon",
                                        style: "width: 24px; height: 24px;",
                                        onclick: move |_| dock_mode.set(DockMode::Closed),
                                        span { class: "material-icons md-16", "close" }
                                    }
                                }
                            }

                            div { class: "dock-scroll",
                                if lyrics_sheet.read().lines.is_empty() {
                                    div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; text-align: center; color: var(--text-muted); gap: 12px; padding: 40px 20px;",
                                        span { class: "material-icons md-48", style: "opacity: 0.6;", "description" }
                                        p { style: "font-size: 15px; font-weight: 600;", "{strings.no_lyrics_available}" }
                                        p { style: "font-size: 12px; line-height: 1.4;", "{strings.no_lyrics_guide}" }
                                    }
                                } else {
                                    div { class: "lyrics-container",
                                        for (idx, line) in lyrics_sheet.read().lines.iter().enumerate() {
                                            {
                                                let is_active = *active_lyric_index.read() == Some(idx);
                                                let ts = line.timestamp_ms;
                                                let text = line.text.clone();
                                                let player = player.clone();

                                                rsx! {
                                                    div {
                                                        key: "{idx}",
                                                        class: if is_active {
                                                            if app_config.read().sidebar_text_glow {
                                                                "lyric-line active sidebar-text-glow"
                                                            } else {
                                                                "lyric-line active"
                                                            }
                                                        } else {
                                                            "lyric-line"
                                                        },
                                                        onclick: move |_| player.read().seek_ms(ts),
                                                        "{text}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        DockMode::Queue => rsx! {
                            div { class: "dock-header",
                                span { style: "font-weight: 700; font-size: 15px;", "{strings.playback_queue}" }
                                div { style: "display: flex; gap: 6px; align-items: center;",
                                    button {
                                        class: if *shuffle_mode.read() == ShuffleMode::Smart { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 11px; padding: 2px 8px; display: inline-flex; align-items: center; gap: 4px;",
                                        title: "Cycle Shuffle Mode (Off / Linear / Smart)",
                                        onclick: move |_| {
                                            let next_mode = shuffle_mode.read().next();
                                            shuffle_mode.set(next_mode);
                                            let p = player.read();
                                            if next_mode == ShuffleMode::Smart {
                                                let curr_pos = *queue_pos.read();
                                                if let Some(next_idx) = pick_next_index(curr_pos) {
                                                    let q = queue.read().clone();
                                                    if next_idx < q.len() {
                                                        p.prepare_transition(
                                                            q[next_idx].path.clone(),
                                                            q[next_idx].title.clone(),
                                                            q[next_idx].artist.clone(),
                                                        );
                                                    }
                                                }
                                            } else {
                                                p.clear_pending_transition();
                                            }
                                        },
                                        span { class: "material-icons md-12",
                                            if *shuffle_mode.read() == ShuffleMode::Smart { "auto_awesome" } else { "shuffle" }
                                        }
                                        span { "{shuffle_mode.read().label()}" }
                                    }
                                    button {
                                        class: "btn-pill",
                                        style: "font-size: 11px; padding: 2px 8px;",
                                        onclick: move |_| {
                                            let mut q = queue.read().clone();
                                            q.clear();
                                            queue.set(q);
                                            queue_pos.set(0);
                                            player.read().stop();
                                            is_playing.set(false);
                                            current_track.set(None);
                                            pending_trans_info.set(None);
                                            last_verdict.set(None);
                                            is_transition_active.set(false);
                                        },
                                        "{strings.clear_queue}"
                                    }
                                    button {
                                        class: "btn-icon",
                                        style: "width: 24px; height: 24px;",
                                        onclick: move |_| dock_mode.set(DockMode::Closed),
                                        span { class: "material-icons md-16", "close" }
                                    }
                                }
                            }

                            // Pending Transition Card (if primed)
                            if let Some(pending) = pending_trans_info.read().as_ref() {
                                div { style: "background: rgba(45, 212, 191, 0.10); border: 1px solid rgba(45, 212, 191, 0.3); border-radius: 12px; margin: 12px 16px 0 16px; padding: 12px;",
                                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                                        span { style: "font-size: 11px; font-weight: 700; color: var(--accent-mint); text-transform: uppercase;", "PRIMED DJ RECIPE" }
                                        span { style: "font-size: 10px; color: var(--text-muted);", "{pending.confidence_pct}% Match" }
                                    }
                                    p { style: "font-weight: 600; font-size: 13px; margin: 4px 0;", "{pending.archetype_name}" }
                                    p { style: "font-size: 11.5px; color: var(--text-secondary);", "Incoming: {pending.incoming_title}" }
                                    div { style: "display: flex; gap: 8px; margin-top: 10px;",
                                        button {
                                            class: "btn-pill",
                                            style: "flex: 1; font-size: 11.5px; padding: 5px 6px; display: inline-flex; align-items: center; justify-content: center; gap: 4px; background: rgba(255,255,255,0.06);",
                                            onclick: move |_| show_candidates_modal.set(true),
                                            span { class: "material-icons md-14", "info" }
                                            "Check More Info"
                                        }
                                        button {
                                            class: "btn-pill active",
                                            style: "flex: 1; font-size: 11.5px; padding: 5px 6px; display: inline-flex; align-items: center; justify-content: center; gap: 4px;",
                                            onclick: move |_| trigger_transition_now(),
                                            span { class: "material-icons md-14", "bolt" }
                                            "Blend Now"
                                        }
                                    }
                                }
                            }

                            div { class: "dock-scroll",
                                if queue.read().is_empty() {
                                    div { style: "text-align: center; color: var(--text-muted); padding: 40px 10px;",
                                        "Queue is empty. Click '+' on any track in your library to add it."
                                    }
                                } else {
                                    div { style: "display: flex; flex-direction: column; gap: 4px;",
                                        for (idx, track) in queue.read().iter().enumerate() {
                                            {
                                                let t = track.clone();
                                                let t_click = t.clone();
                                                let is_current = *queue_pos.read() == idx;
                                                let dur = fmt_ms(t.duration_ms);
                                                let mut play_track = play_track.clone();

                                                rsx! {
                                                    div {
                                                        key: "{idx}-{t.path.display()}",
                                                        class: if is_current { "track-row playing" } else { "track-row" },
                                                        style: "display: flex; align-items: center; justify-content: space-between; padding: 8px 10px;",
                                                        onclick: move |_| {
                                                            queue_pos.set(idx);
                                                            play_track(t_click.clone());
                                                        },
                                                        div { style: "display: flex; flex-direction: column; overflow: hidden; flex: 1;",
                                                            span { style: "font-weight: 500; font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{t.title}" }
                                                            span { style: "font-size: 11.5px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{t.artist}" }
                                                        }
                                                        span { style: "font-size: 11px; color: var(--text-muted); font-family: monospace; margin-left: 8px;", "{dur}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        DockMode::Closed => rsx! {},
                    }
                }
            }

            // ==========================================
            // BOTTOM TRANSPORT BAR
            // ==========================================
            div { class: "transport-bar",
                // Left: Mini Now Playing Pill (Cover art + Title/Artist)
                div { class: "transport-left",
                    if let Some((title, artist_album, art_src)) = mini_pill_info {
                        div { class: "mini-pill",
                            if !art_src.is_empty() {
                                img { class: "mini-art", src: "{art_src}", alt: "{title}" }
                            } else {
                                div {
                                    class: "mini-art",
                                    style: "display: flex; align-items: center; justify-content: center;",
                                    span { class: "material-icons md-20", "music_note" }
                                }
                            }
                            div { class: "mini-meta",
                                div { class: "marquee-container",
                                    span { class: "mini-title marquee-text", title: "{title}", "{title}" }
                                }
                                div { class: "marquee-container",
                                    span { class: "mini-artist marquee-text", title: "{artist_album}", "{artist_album}" }
                                }
                            }
                        }
                    } else {
                        div { class: "mini-pill",
                            div {
                                class: "mini-art",
                                style: "display: flex; align-items: center; justify-content: center; color: var(--text-muted);",
                                span { class: "material-icons md-20", "music_note" }
                            }
                            div { class: "mini-meta",
                                span { class: "mini-title", style: "color: var(--text-muted);", "{strings.attune_player}" }
                                span { class: "mini-artist", "{strings.select_track_to_play}" }
                            }
                        }
                    }
                }

                // Center: Playback Controls on Top + Timeline Scrubber Directly Below
                div { class: "transport-center",
                    div { class: "transport-controls-row",
                        // Shuffle Button (Tri-State: Off -> Linear -> Smart)
                        {
                            let mode = *shuffle_mode.read();
                            let (icon_name, style_str, tooltip_text) = match mode {
                                ShuffleMode::Off => (
                                    "shuffle",
                                    "color: var(--text-muted); opacity: 0.55;",
                                    "Shuffle Off (Linear Sequence)",
                                ),
                                ShuffleMode::Linear => (
                                    "shuffle",
                                    "color: var(--accent-mint);",
                                    "Basic Shuffle (Random Track)",
                                ),
                                ShuffleMode::Smart => (
                                    "auto_awesome",
                                    "color: var(--accent-purple); filter: drop-shadow(0 0 6px rgba(168,85,247,0.7));",
                                    "Smart Shuffle (Algorithmic Transition Match)",
                                ),
                            };

                            rsx! {
                                button {
                                    class: "btn-ctrl",
                                    style: "{style_str}",
                                    title: "{tooltip_text}",
                                    onclick: move |_| {
                                        let next_mode = shuffle_mode.read().next();
                                        shuffle_mode.set(next_mode);
                                        let p = player.read();
                                        if next_mode == ShuffleMode::Smart {
                                            let curr_pos = *queue_pos.read();
                                            if let Some(next_idx) = pick_next_index(curr_pos) {
                                                let q = queue.read().clone();
                                                let all = tracks.read().clone();
                                                let next_track = if !q.is_empty() && next_idx < q.len() {
                                                    Some(&q[next_idx])
                                                } else if q.is_empty() && !all.is_empty() && next_idx < all.len() {
                                                    Some(&all[next_idx])
                                                } else {
                                                    None
                                                };
                                                if let Some(nt) = next_track {
                                                    p.prepare_transition(
                                                        nt.path.clone(),
                                                        nt.title.clone(),
                                                        nt.artist.clone(),
                                                    );
                                                } else {
                                                    p.clear_pending_transition();
                                                }
                                            } else {
                                                p.clear_pending_transition();
                                            }
                                        } else {
                                            p.clear_pending_transition();
                                        }
                                    },
                                    span { class: "material-icons md-20", "{icon_name}" }
                                }
                            }
                        }
                        // Previous
                        button {
                            class: "btn-ctrl",
                            title: "Previous Track",
                            onclick: move |_| play_prev_track(),
                            span { class: "material-icons md-22", "skip_previous" }
                        }
                        // Play / Pause
                        button {
                            class: "btn-ctrl btn-play",
                            title: if *is_playing.read() { "Pause" } else { "Play" },
                            onclick: move |_| {
                                let p = player.read();
                                if *is_playing.read() { p.pause(); } else { p.play(); }
                            },
                            if *is_playing.read() {
                                span { class: "material-icons md-24", "pause" }
                            } else {
                                span { class: "material-icons md-24", "play_arrow" }
                            }
                        }
                        // Next / Transition
                        button {
                            class: "btn-ctrl",
                            title: "Next Track (Seamless Blend)",
                            onclick: move |_| trigger_transition_now(),
                            span { class: "material-icons md-22", "skip_next" }
                        }
                        // Repeat
                        button {
                            class: "btn-ctrl",
                            style: match *repeat_mode.read() {
                                RepeatMode::Off => "",
                                RepeatMode::All => "color: var(--accent-mint);",
                                RepeatMode::One => "color: var(--accent-purple);",
                            },
                            title: "Repeat Mode",
                            onclick: move |_| {
                                let next_mode = match *repeat_mode.read() {
                                    RepeatMode::Off => RepeatMode::All,
                                    RepeatMode::All => RepeatMode::One,
                                    RepeatMode::One => RepeatMode::Off,
                                };
                                repeat_mode.set(next_mode);
                            },
                            if *repeat_mode.read() == RepeatMode::One {
                                span { class: "material-icons md-20", "repeat_one" }
                            } else {
                                span { class: "material-icons md-20", "repeat" }
                            }
                        }
                    }

                    // Scrubber Row: 0:42 ━━━━━━━━━━━━━━━━━━●────────────────────── -2:01
                    div { class: "scrubber-row",
                        span { class: "time-label", "{elapsed_str}" }
                        input {
                            r#type: "range",
                            class: "scrubber-slider",
                            min: "0",
                            max: "{playback_dur_ms.read().max(1)}",
                            value: "{playback_pos_ms.read()}",
                            style: "background: linear-gradient(to right, var(--accent-mint) 0%, var(--accent-mint) {progress_pct}%, rgba(255, 255, 255, 0.15) {progress_pct}%, rgba(255, 255, 255, 0.15) 100%);",
                            oninput: move |evt| {
                                if let Ok(val) = evt.value().parse::<u64>() {
                                    playback_pos_ms.set(val);
                                    player.read().seek_ms(val);
                                }
                            },
                        }
                        span { class: "time-label", style: "text-align: right;", "{remaining_str}" }
                    }
                }

                // Right: Actions (Favorites, Volume with Dynamic Fill, Fullscreen)
                div { class: "transport-right",
                    // Favorite Heart
                    button {
                        class: "btn-ctrl",
                        style: if current_track.read().as_ref().map(|t| favorites.read().contains(&t.path)).unwrap_or(false) {
                            "color: #fb7185;"
                        } else {
                            ""
                        },
                        title: "Toggle Favorite",
                        onclick: move |_| {
                            if let Some(t) = current_track.read().as_ref() {
                                let mut favs = favorites.read().clone();
                                if favs.contains(&t.path) {
                                    favs.remove(&t.path);
                                } else {
                                    favs.insert(t.path.clone());
                                }
                                favorites.set(favs);
                            }
                        },
                        if current_track.read().as_ref().map(|t| favorites.read().contains(&t.path)).unwrap_or(false) {
                            span { class: "material-icons md-20", "favorite" }
                        } else {
                            span { class: "material-icons md-20", "favorite_border" }
                        }
                    }

                    // Volume Slider with Mint Linear-Gradient Fill
                    div { style: "display: flex; align-items: center; gap: 8px;",
                        span { class: "material-icons md-18", style: "color: var(--text-muted);", "volume_up" }
                        input {
                            r#type: "range",
                            class: "vol-slider",
                            min: "0",
                            max: "100",
                            value: "{vol_pct}",
                            style: "background: linear-gradient(to right, var(--accent-mint) 0%, var(--accent-mint) {vol_pct}%, rgba(255, 255, 255, 0.18) {vol_pct}%, rgba(255, 255, 255, 0.18) 100%);",
                            oninput: move |evt| {
                                if let Ok(pct) = evt.value().parse::<u32>() {
                                    volume_pct.set(pct);
                                    let db = if pct == 0 {
                                        -60.0
                                    } else {
                                        ((pct as f32 / 100.0).log10() * 20.0).clamp(-60.0, 0.0)
                                    };
                                    player.read().set_gain_db(db);
                                }
                            },
                        }
                    }

                    // Fullscreen / Stage Toggle
                    button {
                        class: "btn-ctrl",
                        title: "Toggle Pure Fullscreen Now Playing",
                        onclick: move |_| {
                            let is_fs = *nav_state.read().current() == StageView::FullscreenNowPlaying;
                            if is_fs {
                                dioxus::desktop::window().set_fullscreen(false);
                                nav_state.write().back();
                            } else {
                                navigate_to(StageView::FullscreenNowPlaying);
                            }
                        },
                        span { class: "material-icons md-20", "fullscreen" }
                    }
                }
            }

            // ==========================================
            // MODALS: VERDICT
            // ==========================================

            if *show_verdict_modal.read() {
                if let Some(verdict) = last_verdict.read().as_ref() {
                    div {
                        style: "position: fixed; inset: 0; background: rgba(0,0,0,0.65); backdrop-filter: blur(12px); display: flex; align-items: center; justify-content: center; z-index: 100;",
                        div {
                            style: "background: var(--bg-card); border: 1px solid var(--border-glass); border-radius: 20px; padding: 28px; width: 500px; box-shadow: 0 16px 48px rgba(0,0,0,0.7); display: flex; flex-direction: column; gap: 16px;",
                            div { style: "display: flex; justify-content: space-between; align-items: center;",
                                h2 { style: "font-size: 20px; font-weight: 700;", "Transition Recipe Verdict" }
                                button {
                                    class: "btn-icon",
                                    onclick: move |_| show_verdict_modal.set(false),
                                    span { class: "material-icons md-18", "close" }
                                }
                            }
                            div { style: "display: flex; flex-direction: column; gap: 10px; font-size: 13.5px;",
                                div { style: "display: flex; justify-content: space-between; border-bottom: 1px solid var(--border-glass); padding-bottom: 8px;",
                                    span { style: "color: var(--text-muted);", "Archetype" }
                                    span { style: "font-weight: 600; color: var(--accent-mint);", "{verdict.archetype_name}" }
                                }
                                div { style: "display: flex; justify-content: space-between; border-bottom: 1px solid var(--border-glass); padding-bottom: 8px;",
                                    span { style: "color: var(--text-muted);", "Incoming Track" }
                                    span { style: "font-weight: 500;", "{verdict.incoming_title} — {verdict.incoming_artist}" }
                                }
                                div { style: "display: flex; justify-content: space-between; border-bottom: 1px solid var(--border-glass); padding-bottom: 8px;",
                                    span { style: "color: var(--text-muted);", "Duration" }
                                    span { style: "font-family: monospace;", "{verdict.duration_sec:.1}s" }
                                }
                                div { style: "display: flex; justify-content: space-between; border-bottom: 1px solid var(--border-glass); padding-bottom: 8px;",
                                    span { style: "color: var(--text-muted);", "Confidence Score" }
                                    span { style: "font-weight: 600; color: var(--accent-purple);", "{verdict.confidence_pct}%" }
                                }
                            }
                            button {
                                class: "btn-pill active",
                                style: "margin-top: 10px;",
                                onclick: move |_| show_verdict_modal.set(false),
                                "Close"
                            }
                        }
                    }
                }
            }

            // ==========================================
            // MODALS: CANDIDATES & TRANSITION RATIONALE
            // ==========================================

            if *show_candidates_modal.read() {
                {
                    let pending = pending_trans_info.cloned();
                    let curr_opt = current_track.cloned();
                    let q_list = queue.cloned();
                    let all_list = tracks.cloned();
                    let hist = shuffle_history.cloned();

                    let (cand_pool, current) = if let Some(c) = curr_opt {
                        let pool = if !q_list.is_empty() { q_list } else { all_list };
                        (pool, Some(c))
                    } else {
                        (q_list, None)
                    };

                    let evaluated = if let Some(ref c) = current {
                        let candidate_indices: Vec<usize> = (0..cand_pool.len())
                            .filter(|&i| cand_pool[i].path != c.path)
                            .collect();
                        smart_shuffle::evaluate_candidates(c, &candidate_indices, &cand_pool, &hist)
                    } else {
                        Vec::new()
                    };

                    rsx! {
                        div {
                            style: "position: fixed; inset: 0; background: rgba(0,0,0,0.72); backdrop-filter: blur(14px); display: flex; align-items: center; justify-content: center; z-index: 100;",
                            div {
                                style: "background: #141924; border: 1px solid var(--border-glass); border-radius: 24px; padding: 24px; width: 560px; max-height: 85vh; display: flex; flex-direction: column; gap: 16px; box-shadow: 0 20px 60px rgba(0,0,0,0.85);",
                                // Header
                                div { style: "display: flex; justify-content: space-between; align-items: center;",
                                    div { style: "display: flex; align-items: center; gap: 12px;",
                                        div {
                                            style: "width: 40px; height: 40px; border-radius: 50%; background: rgba(45,212,191,0.18); display: flex; align-items: center; justify-content: center;",
                                            span { class: "material-icons md-22", style: "color: var(--accent-mint);", "auto_awesome" }
                                        }
                                        div {
                                            h2 { style: "font-size: 17px; font-weight: 700; margin: 0;", "Transition Selection Rationale" }
                                            p { style: "font-size: 11.5px; color: var(--text-secondary); margin: 2px 0 0 0;",
                                                if let Some(ref p) = pending {
                                                    "{p.archetype_name} • {p.confidence_pct}% Harmonic Alignment"
                                                } else {
                                                    "Smart JIT Evaluation & Candidate Breakdown"
                                                }
                                            }
                                        }
                                    }
                                    button {
                                        class: "btn-icon",
                                        onclick: move |_| show_candidates_modal.set(false),
                                        span { class: "material-icons md-18", "close" }
                                    }
                                }

                                // Scrollable Body
                                div {
                                    style: "overflow-y: auto; display: flex; flex-direction: column; gap: 14px; max-height: 58vh; padding-right: 4px;",
                                    // Winner Card
                                    if let Some(ref p) = pending {
                                        div {
                                            style: "background: rgba(45,212,191,0.08); border: 1px solid rgba(45,212,191,0.28); border-radius: 16px; padding: 14px; display: flex; flex-direction: column; gap: 10px;",
                                            div { style: "display: flex; justify-content: space-between; align-items: center;",
                                                div { style: "display: flex; align-items: center; gap: 6px;",
                                                    span { class: "material-icons md-16", style: "color: var(--accent-mint);", "stars" }
                                                    span { style: "font-size: 10.5px; font-weight: 700; color: var(--accent-mint); letter-spacing: 0.5px;", "WINNING CANDIDATE • PRIMED UP NEXT" }
                                                }
                                                div {
                                                    style: "background: rgba(45,212,191,0.22); color: var(--accent-mint); font-weight: 700; font-size: 11px; padding: 3px 8px; border-radius: 6px;",
                                                    "{p.confidence_pct}% Match"
                                                }
                                            }
                                            div {
                                                h3 { style: "font-size: 14px; font-weight: 700; margin: 0;", "{p.incoming_title}" }
                                                p { style: "font-size: 11.5px; color: var(--text-secondary); margin: 2px 0 0 0;", "{p.incoming_artist}" }
                                            }
                                            div {
                                                style: "background: rgba(0,0,0,0.3); border-radius: 10px; padding: 10px; font-size: 11.5px; line-height: 1.5; color: rgba(255,255,255,0.85);",
                                                "Attune's JIT Transition Planner analyzed outgoing acoustic energy and selected {p.archetype_name} with {p.confidence_pct}% confidence. The transition window spans {p.duration_sec:.1}s with sample-accurate 3-band EQ and harmonic phase alignment."
                                            }
                                            // Factor chips
                                            div { style: "display: flex; flex-wrap: wrap; gap: 6px; margin-top: 2px;",
                                                div { style: "background: rgba(255,255,255,0.06); padding: 3px 8px; border-radius: 6px; font-size: 10.5px; display: flex; align-items: center; gap: 4px;",
                                                    span { class: "material-icons md-12", "music_note" }
                                                    "Harmonic Key Lock"
                                                }
                                                div { style: "background: rgba(255,255,255,0.06); padding: 3px 8px; border-radius: 6px; font-size: 10.5px; display: flex; align-items: center; gap: 4px;",
                                                    span { class: "material-icons md-12", "speed" }
                                                    "Tempo Synchronized"
                                                }
                                                div { style: "background: rgba(255,255,255,0.06); padding: 3px 8px; border-radius: 6px; font-size: 10.5px; display: flex; align-items: center; gap: 4px;",
                                                    span { class: "material-icons md-12", "bolt" }
                                                    "Energy Flow Continuous"
                                                }
                                            }
                                        }
                                    }

                                    // Alternative Runner-Up Candidates
                                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-top: 4px;",
                                        h4 { style: "font-size: 12.5px; font-weight: 700; margin: 0; color: var(--text-primary);",
                                            "Other Evaluated Candidates ({evaluated.len()})"
                                        }
                                        span { style: "font-size: 10.5px; color: var(--text-muted);", "Runner-ups • Click to Override" }
                                    }

                                    if evaluated.is_empty() {
                                        p { style: "font-size: 12px; color: var(--text-muted); text-align: center; padding: 12px 0;", "No alternative candidates found in current queue." }
                                    } else {
                                        for (rank_idx, eval) in evaluated.iter().enumerate() {
                                            {
                                                let track_clone = cand_pool[eval.track_idx].clone();
                                                let is_primed = pending.as_ref().map(|p| p.incoming_path == track_clone.path).unwrap_or(false);
                                                let score_pct = (eval.total_score * 100.0).clamp(10.0, 99.0) as u32;
                                                let rank_num = rank_idx + 1;

                                                rsx! {
                                                    div {
                                                        key: "{track_clone.path.to_string_lossy()}",
                                                        style: "background: rgba(255,255,255,0.035); border: 1px solid var(--border-glass); border-radius: 12px; padding: 10px 12px; display: flex; align-items: center; justify-content: space-between; gap: 10px;",
                                                        div { style: "display: flex; align-items: center; gap: 10px; min-width: 0; flex: 1;",
                                                            div {
                                                                style: "width: 26px; height: 26px; border-radius: 50%; background: rgba(255,255,255,0.07); display: flex; align-items: center; justify-content: center; font-size: 10px; font-weight: 700; color: var(--text-muted); flex-shrink: 0;",
                                                                "#{rank_num}"
                                                            }
                                                            div { style: "min-width: 0; flex: 1;",
                                                                div { style: "display: flex; align-items: center; gap: 6px;",
                                                                    span { style: "font-size: 12px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--text-primary);", "{eval.title}" }
                                                                    span { style: "background: rgba(168,85,247,0.18); color: var(--accent-purple); font-weight: 700; font-size: 9.5px; padding: 1px 5px; border-radius: 4px; flex-shrink: 0;", "{score_pct}%" }
                                                                }
                                                                p { style: "font-size: 10.5px; color: var(--text-secondary); margin: 2px 0 0 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                                                    "{eval.artist} • {eval.harmonic_verdict} • {eval.tempo_verdict}"
                                                                }
                                                            }
                                                        }
                                                        if is_primed {
                                                            div {
                                                                style: "font-size: 11px; font-weight: 600; color: var(--accent-mint); padding: 4px 10px; border-radius: 8px; background: rgba(45,212,191,0.12); flex-shrink: 0;",
                                                                "Active Up Next"
                                                            }
                                                        } else {
                                                            button {
                                                                class: "btn-pill",
                                                                style: "font-size: 11px; padding: 4px 10px; display: inline-flex; align-items: center; gap: 4px; background: rgba(45,212,191,0.15); color: var(--accent-mint); border: 1px solid rgba(45,212,191,0.3); flex-shrink: 0; cursor: pointer;",
                                                                onclick: {
                                                                    let t = track_clone.clone();
                                                                    move |_| override_candidate(t.clone())
                                                                },
                                                                span { class: "material-icons md-14", "swap_vert" }
                                                                "Promote"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Footer
                                div { style: "display: flex; justify-content: flex-end; padding-top: 8px; border-top: 1px solid var(--border-glass);",
                                    button {
                                        class: "btn-pill active",
                                        style: "padding: 6px 18px; font-size: 12px;",
                                        onclick: move |_| show_candidates_modal.set(false),
                                        "Close"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_initial_state() {
        let nav = NavigationState::new(StageView::Home);
        assert_eq!(nav.current(), &StageView::Home);
        assert_eq!(nav.index, 0);
        assert_eq!(nav.history.len(), 1);
        assert!(!nav.can_go_back());
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn test_navigation_push_and_back_single_press() {
        let mut nav = NavigationState::new(StageView::Home);

        // Home -> ForYou
        assert!(nav.push(StageView::ForYou));
        assert_eq!(nav.current(), &StageView::ForYou);
        assert_eq!(nav.index, 1);
        assert!(nav.can_go_back());
        assert!(!nav.can_go_forward());

        // ForYou -> Harmonic Flow
        assert!(nav.push(StageView::ForYouDetail("harmonic_flow".into())));
        assert_eq!(nav.current(), &StageView::ForYouDetail("harmonic_flow".into()));
        assert_eq!(nav.index, 2);
        assert!(nav.can_go_back());
        assert!(!nav.can_go_forward());

        // Single Back press MUST return to ForYou, NOT get stuck on Harmonic Flow and NOT yeet to Home
        let back_res = nav.back();
        assert_eq!(back_res, Some(&StageView::ForYou));
        assert_eq!(nav.current(), &StageView::ForYou);
        assert_eq!(nav.index, 1);
        assert!(nav.can_go_back());
        assert!(nav.can_go_forward());

        // Second Back press returns to Home
        let back_res2 = nav.back();
        assert_eq!(back_res2, Some(&StageView::Home));
        assert_eq!(nav.current(), &StageView::Home);
        assert_eq!(nav.index, 0);
        assert!(!nav.can_go_back());
        assert!(nav.can_go_forward());

        // Third Back press at root returns None
        assert_eq!(nav.back(), None);
        assert_eq!(nav.current(), &StageView::Home);
    }

    #[test]
    fn test_navigation_forward_flow() {
        let mut nav = NavigationState::new(StageView::Home);
        nav.push(StageView::ForYou);
        nav.push(StageView::ForYouDetail("harmonic_flow".into()));

        nav.back(); // At ForYou
        nav.back(); // At Home

        // Forward 1 -> ForYou
        assert_eq!(nav.forward(), Some(&StageView::ForYou));
        assert_eq!(nav.current(), &StageView::ForYou);

        // Forward 2 -> Harmonic Flow
        assert_eq!(nav.forward(), Some(&StageView::ForYouDetail("harmonic_flow".into())));
        assert_eq!(nav.current(), &StageView::ForYouDetail("harmonic_flow".into()));
        assert!(!nav.can_go_forward());

        // Forward 3 at tip returns None
        assert_eq!(nav.forward(), None);
    }

    #[test]
    fn test_navigation_duplicate_push_is_ignored() {
        let mut nav = NavigationState::new(StageView::Home);
        assert!(!nav.push(StageView::Home));
        assert_eq!(nav.history.len(), 1);

        nav.push(StageView::Albums);
        assert!(!nav.push(StageView::Albums));
        assert_eq!(nav.history.len(), 2);
        assert_eq!(nav.index, 1);
    }

    #[test]
    fn test_navigation_album_detail_single_press_back() {
        let mut nav = NavigationState::new(StageView::Home);
        nav.push(StageView::Albums);
        nav.push(StageView::AlbumDetail("Super Trouper".into()));

        assert_eq!(nav.current(), &StageView::AlbumDetail("Super Trouper".into()));

        // Exactly one back returns to Albums catalog
        assert_eq!(nav.back(), Some(&StageView::Albums));
        assert_eq!(nav.current(), &StageView::Albums);
    }

    #[test]
    fn test_navigation_branch_truncates_forward_history() {
        let mut nav = NavigationState::new(StageView::Home);
        nav.push(StageView::ForYou);
        nav.push(StageView::ForYouDetail("harmonic_flow".into()));
        assert_eq!(nav.history.len(), 3);

        nav.back(); // back at ForYou
        assert!(nav.can_go_forward());

        // Now branch to High Energy
        nav.push(StageView::ForYouDetail("high_energy".into()));
        assert_eq!(nav.history.len(), 3);
        assert_eq!(nav.history[2], StageView::ForYouDetail("high_energy".into()));
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn test_navigation_ping_pong_deduplication() {
        let mut nav = NavigationState::new(StageView::Home);
        nav.push(StageView::ForYou);
        nav.push(StageView::ForYouDetail("harmonic_flow".into()));
        assert_eq!(nav.index, 2);

        // Clicking ForYou while on Harmonic Flow steps back to ForYou instead of pushing a duplicate loop
        assert!(nav.push(StageView::ForYou));
        assert_eq!(nav.current(), &StageView::ForYou);
        assert_eq!(nav.index, 1);

        // Single back goes to Home
        assert_eq!(nav.back(), Some(&StageView::Home));
        assert_eq!(nav.current(), &StageView::Home);
    }

    #[test]
    fn test_navigation_detail_can_go_back_at_root() {
        let nav = NavigationState::new(StageView::ForYouDetail("harmonic_flow".into()));
        assert_eq!(nav.index, 0);
        // Can go back because it has a logical parent stage (ForYou)
        assert!(nav.can_go_back());
    }

    fn make_test_track(title: &str, artist: &str, bpm: Option<f32>, key: Option<&str>) -> TrackEntry {
        TrackEntry {
            path: PathBuf::from(format!("/music/{title}.flac")),
            title: title.into(),
            artist: artist.into(),
            album: "Test Album".into(),
            genre: None,
            duration_ms: 180_000,
            bpm,
            key: key.map(Into::into),
            format: "FLAC".into(),
            sample_rate: 48000,
            bit_depth: 24,
            artwork_path: None,
            artwork_data_url: None,
            aggression: None,
            vibe_family: None,
            ..Default::default()
        }
    }

    #[test]
    fn test_single_song_queue_smart_shuffle_never_escapes_or_injects() {
        let hist = SmartShuffleHistory::new(10);
        let queue = vec![make_test_track("Song 1", "Artist 1", Some(120.0), Some("1A"))];

        let mut all_tracks = Vec::new();
        for i in 0..20 {
            all_tracks.push(make_test_track(&format!("Library Song {i}"), "Artist X", Some(120.0 + (i as f32)), Some("1A")));
        }

        // When queue has only 1 track, Smart Shuffle MUST return None!
        // It must NEVER return an index into all_tracks or auto-inject library songs.
        let next_smart = select_next_playback_index(ShuffleMode::Smart, 0, &queue, &all_tracks, &hist);
        assert_eq!(next_smart, None, "Smart Shuffle must return None for a single-track queue");

        let next_linear = select_next_playback_index(ShuffleMode::Linear, 0, &queue, &all_tracks, &hist);
        assert_eq!(next_linear, None, "Linear Shuffle must return None for a single-track queue");

        let next_off = select_next_playback_index(ShuffleMode::Off, 0, &queue, &all_tracks, &hist);
        assert_eq!(next_off, None, "Sequential playback must return None for a single-track queue at end");
    }

    #[test]
    fn test_multi_song_queue_smart_shuffle_stays_strictly_within_queue() {
        let hist = SmartShuffleHistory::new(10);
        let queue = vec![
            make_test_track("Q1", "Artist 1", Some(120.0), Some("1A")),
            make_test_track("Q2", "Artist 2", Some(121.0), Some("1B")),
        ];

        let mut all_tracks = Vec::new();
        for i in 0..50 {
            all_tracks.push(make_test_track(&format!("Lib {i}"), "Lib Artist", Some(120.0), Some("1A")));
        }

        let next = select_next_playback_index(ShuffleMode::Smart, 0, &queue, &all_tracks, &hist);
        assert_eq!(next, Some(1), "Smart Shuffle must choose the remaining song in the queue (index 1), not from library");
    }

    #[test]
    fn test_empty_queue_selects_from_library() {
        let hist = SmartShuffleHistory::new(10);
        let queue = Vec::<TrackEntry>::new();
        let all_tracks = vec![
            make_test_track("T1", "A1", Some(120.0), Some("1A")),
            make_test_track("T2", "A2", Some(122.0), Some("2A")),
        ];

        let next = select_next_playback_index(ShuffleMode::Smart, 0, &queue, &all_tracks, &hist);
        assert_eq!(next, Some(1), "With empty queue, Smart Shuffle selects from library");
    }
}
