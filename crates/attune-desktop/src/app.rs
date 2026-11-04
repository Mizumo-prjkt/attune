use dioxus::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use attune_core::MasterFxParams;
use crate::components::lyrics_parser::{resolve_companion_lyrics, LyricsSheet};
use crate::config;
use crate::library::{self, fmt_ms, ScanEvent, TrackEntry};
use crate::player::{PendingTransitionInfo, Player};
use crate::stages::{
    about::About, album_editor::AlbumEditor, albums::{AlbumGroup, Albums}, equalizer::Equalizer,
    favorites::Favorites, for_you::ForYou, home::Home, library::Library,
};

#[derive(Debug, Clone, PartialEq)]
pub enum StageView {
    Home,
    ForYou,
    Library,
    Albums,
    Favorites,
    Equalizer,
    AlbumEditor(AlbumGroup),
    About,
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

#[component]
pub fn App() -> Element {
    // 1. Audio Engine & Library State
    let player = use_signal(|| Arc::new(Player::new()));
    let mut tracks = use_signal(Vec::<TrackEntry>::new);
    let mut music_dir = use_signal(|| {
        let (dir, _) = config::resolve_music_dir(None);
        dir
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
    let mut shuffle_mode = use_signal(|| true); // Smart Shuffle on by default
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
    let mut master_fx = use_signal(MasterFxParams::default);

    // 5. Navigation & Docks
    let mut current_stage = use_signal(|| StageView::Home);
    let mut stage_history = use_signal(|| vec![StageView::Home]);
    let mut history_index = use_signal(|| 0usize);
    let mut dock_mode = use_signal(|| DockMode::Lyrics);
    let mut search_query = use_signal(String::new);

    // Initial scan on launch if music directory is resolved
    use_hook(|| {
        if let Some(dir) = music_dir.read().clone() {
            let rx = library::spawn_scan(dir);
            *scan_rx.write() = Arc::new(Mutex::new(Some(rx)));
            is_scanning.set(true);
        }
    });

    // Helper: Navigation change with history tracking
    let mut navigate_to = move |new_stage: StageView| {
        if *current_stage.read() != new_stage {
            let mut hist = stage_history.read().clone();
            let curr_idx = *history_index.read();
            hist.truncate(curr_idx + 1);
            hist.push(new_stage.clone());
            let next_idx = hist.len() - 1;
            stage_history.set(hist);
            history_index.set(next_idx);
            current_stage.set(new_stage);
        }
    };

    // Helper: Play a specific track
    let mut play_track = {
        let player = player.clone();
        move |track: TrackEntry| {
            let p = player.read();
            p.play_file(track.path.clone());
            current_track.set(Some(track.clone()));

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

            // Prepare transition to next track in queue if available
            let q = queue.read().clone();
            let pos = *queue_pos.read();
            if pos + 1 < q.len() {
                let next_track = &q[pos + 1];
                p.prepare_transition(
                    next_track.path.clone(),
                    next_track.title.clone(),
                    next_track.artist.clone(),
                );
            }
        }
    };

    // Helper: Start playlist / album / mix
    let mut play_track_list = {
        let mut play_track = play_track.clone();
        move |list: Vec<TrackEntry>| {
            if list.is_empty() { return; }
            queue.set(list.clone());
            queue_pos.set(0);
            play_track(list[0].clone());
        }
    };

    // Helper: Next track
    let play_next_track = {
        let mut play_track = play_track.clone();
        move || {
            let q = queue.read().clone();
            let curr_pos = *queue_pos.read();
            if curr_pos + 1 < q.len() {
                queue_pos.set(curr_pos + 1);
                play_track(q[curr_pos + 1].clone());
            } else if *repeat_mode.read() == RepeatMode::All && !q.is_empty() {
                queue_pos.set(0);
                play_track(q[0].clone());
            } else {
                let all_tracks = tracks.read().clone();
                if !all_tracks.is_empty() {
                    // Pick a random or harmonic track
                    use std::time::SystemTime;
                    let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
                    let idx = (seed as usize) % all_tracks.len();
                    play_track(all_tracks[idx].clone());
                }
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

    // Helper: Trigger manual transition now
    let mut trigger_transition_now = {
        let player = player.clone();
        let mut play_next_track = play_next_track.clone();
        move || {
            let p = player.read();
            if let Some(triggered) = p.trigger_transition_now() {
                last_verdict.set(Some(triggered.clone()));
                // Advance queue
                let q = queue.read().clone();
                let curr_pos = *queue_pos.read();
                if curr_pos + 1 < q.len() {
                    queue_pos.set(curr_pos + 1);
                    current_track.set(Some(q[curr_pos + 1].clone()));
                    let sheet = resolve_companion_lyrics(&q[curr_pos + 1].path).unwrap_or_else(LyricsSheet::empty);
                    lyrics_sheet.set(sheet);
                }
            } else {
                play_next_track();
            }
        }
    };

    // Background Async Poller (50ms Interval): Updates playback, auto-transitions, scan events, and lyrics
    use_future(move || {
        let player = player.clone();
        let scan_rx = scan_rx.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(50)).await;

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

                    // Determine active deck position and duration
                    let (pos, dur) = if snap.active_deck_str == "Deck B" {
                        (snap.deck_b.position_ms, snap.deck_b.duration_ms)
                    } else {
                        (snap.deck_a.position_ms, snap.deck_a.duration_ms)
                    };
                    playback_pos_ms.set(pos);
                    playback_dur_ms.set(dur);

                    if let Some((_, _, pct, archetype)) = &snap.transition_info {
                        active_trans_name.set(Some(format!("Blending: {archetype} ({:.0}%)", pct * 100.0)));
                    } else {
                        active_trans_name.set(None);
                    }
                }

                // 3. Check for Automatic JIT Transition Trigger
                if let Some(triggered) = p.check_auto_transition() {
                    last_verdict.set(Some(triggered.clone()));
                    let q = queue.read().clone();
                    let curr_pos = *queue_pos.read();
                    if curr_pos + 1 < q.len() {
                        queue_pos.set(curr_pos + 1);
                        current_track.set(Some(q[curr_pos + 1].clone()));
                        let sheet = resolve_companion_lyrics(&q[curr_pos + 1].path).unwrap_or_else(LyricsSheet::empty);
                        lyrics_sheet.set(sheet);
                    }
                }

                // 4. Update Pending Transition Info
                pending_trans_info.set(p.pending_transition());

                // 5. Update Synchronized Lyrics Active Line
                let pos = *playback_pos_ms.read();
                let active_idx = lyrics_sheet.read().active_line_index(pos);
                if active_idx != *active_lyric_index.read() {
                    active_lyric_index.set(active_idx);
                }
            }
        }
    });

    // Compute harmonic matches for the currently playing track
    let harmonic_matches: Vec<TrackEntry> = {
        let all = tracks.read().clone();
        if let Some(curr) = current_track.read().as_ref() {
            let curr_key = curr.key.as_deref().unwrap_or("");
            all.into_iter()
                .filter(|t| t.path != curr.path && t.key.as_deref().unwrap_or("").len() > 1 && (curr_key.is_empty() || t.key.as_deref() == Some(curr_key)))
                .take(6)
                .collect()
        } else {
            all.into_iter().take(6).collect()
        }
    };

    // Calculate time labels for scrubber
    let elapsed_str = fmt_ms(*playback_pos_ms.read());
    let rem_ms = playback_dur_ms.read().saturating_sub(*playback_pos_ms.read());
    let remaining_str = format!("-{}", fmt_ms(rem_ms));
    let (scan_curr, scan_total) = *scan_progress.read();

    let curr_track_clone = current_track.read().clone();
    let mini_pill_info = curr_track_clone.as_ref().map(|track| {
        let art_src = track.artwork_data_url.clone().unwrap_or_default();
        (track.title.clone(), format!("{} • {}", track.artist, track.album), art_src)
    });

    rsx! {
        div { class: "app-container",
            div { class: "ambient-mesh" }

            // ==========================================
            // LEFT SIDEBAR
            // ==========================================
            div { class: "sidebar",
                // Brand Header
                div { style: "display: flex; align-items: center; gap: 10px; padding: 6px 8px 12px 8px;",
                    div {
                        style: "width: 28px; height: 28px; border-radius: 7px; background: linear-gradient(135deg, var(--accent-mint), var(--accent-purple)); display: flex; align-items: center; justify-content: center; font-size: 16px; font-weight: 800; color: #0b101b;",
                        "A"
                    }
                    span { style: "font-weight: 700; font-size: 16px; letter-spacing: 0.05em;", "ATTUNE" }
                }

                // Global Search Input
                div { class: "search-bar",
                    span { "🔍" }
                    input {
                        placeholder: "Search library...",
                        value: "{search_query}",
                        oninput: move |evt| {
                            search_query.set(evt.value());
                            if *current_stage.read() != StageView::Library {
                                navigate_to(StageView::Library);
                            }
                        },
                    }
                }

                // Attune Main Navigation
                div { class: "nav-section",
                    span { class: "section-label", "ATTUNE" }
                    div {
                        class: if *current_stage.read() == StageView::Home { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Home),
                        span { "🏠" }
                        span { "Home" }
                    }
                    div {
                        class: if *current_stage.read() == StageView::ForYou { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::ForYou),
                        span { "✨" }
                        span { "For You" }
                    }
                    div {
                        class: if *current_stage.read() == StageView::Library { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Library),
                        span { "♫" }
                        span { "Library" }
                    }
                    div {
                        class: if *current_stage.read() == StageView::Albums { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Albums),
                        span { "◉" }
                        span { "Albums" }
                    }
                    div {
                        class: if *current_stage.read() == StageView::Favorites { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Favorites),
                        span { "❤️" }
                        span { "Favorite Songs" }
                    }
                }

                // Sources Section
                div { class: "nav-section", style: "margin-top: 8px;",
                    span { class: "section-label", "SOURCES" }
                    div { style: "padding: 6px 8px; font-size: 11.5px; color: var(--text-muted); word-break: break-all; background: rgba(255,255,255,0.03); border-radius: 6px; border: 1px solid var(--border-glass); margin-bottom: 8px;",
                        if let Some(dir) = music_dir.read().as_ref() {
                            "📁 {dir.display()}"
                        } else {
                            "No folder chosen"
                        }
                    }
                    div { style: "display: flex; gap: 6px;",
                        button {
                            class: "btn-pill",
                            style: "flex: 1; padding: 4px 8px; font-size: 11.5px;",
                            onclick: move |_| {
                                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                    music_dir.set(Some(path.clone()));
                                    tracks.set(Vec::new());
                                    let rx = library::spawn_scan(path);
                                    *scan_rx.write() = Arc::new(Mutex::new(Some(rx)));
                                    is_scanning.set(true);
                                }
                            },
                            "Choose"
                        }
                        button {
                            class: "btn-pill",
                            style: "flex: 1; padding: 4px 8px; font-size: 11.5px;",
                            disabled: music_dir.read().is_none() || *is_scanning.read(),
                            onclick: move |_| {
                                if let Some(dir) = music_dir.read().clone() {
                                    tracks.set(Vec::new());
                                    let rx = library::spawn_scan(dir);
                                    *scan_rx.write() = Arc::new(Mutex::new(Some(rx)));
                                    is_scanning.set(true);
                                }
                            },
                            if *is_scanning.read() { "Scanning..." } else { "Rescan" }
                        }
                    }
                }

                // Sidebar Bottom
                div { class: "sidebar-bottom",
                    div {
                        class: if *current_stage.read() == StageView::Equalizer { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::Equalizer),
                        span { "🎚️" }
                        span { "Equalizer" }
                    }
                    div {
                        class: if *current_stage.read() == StageView::About { "nav-item active" } else { "nav-item" },
                        onclick: move |_| navigate_to(StageView::About),
                        span { "ℹ" }
                        span { "About Attune" }
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
                            disabled: *history_index.read() == 0,
                            onclick: move |_| {
                                let idx = *history_index.read();
                                if idx > 0 {
                                    history_index.set(idx - 1);
                                    current_stage.set(stage_history.read()[idx - 1].clone());
                                }
                            },
                            "‹"
                        }
                        button {
                            class: "btn-icon",
                            disabled: *history_index.read() + 1 >= stage_history.read().len(),
                            onclick: move |_| {
                                let idx = *history_index.read();
                                if idx + 1 < stage_history.read().len() {
                                    history_index.set(idx + 1);
                                    current_stage.set(stage_history.read()[idx + 1].clone());
                                }
                            },
                            "›"
                        }
                    }

                    // Scan Status or Active Transition Telemetry
                    div { style: "display: flex; align-items: center; gap: 10px;",
                        if *is_scanning.read() {
                            div { class: "quality-badge quality-good",
                                "🔄 Scanning {scan_curr}/{scan_total}: {last_scanned_file.read()}"
                            }
                        } else if let Some(trans) = active_trans_name.read().as_ref() {
                            div { class: "quality-badge quality-very-good",
                                "✨ {trans}"
                            }
                        } else if let Some(pending) = pending_trans_info.read().as_ref() {
                            div {
                                class: "quality-badge quality-proprietary",
                                style: "cursor: pointer;",
                                onclick: move |_| show_verdict_modal.set(true),
                                "⚡ Primed: {pending.archetype_name} ({pending.confidence_pct}%)"
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
                            "💬 Lyrics"
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
                            "☰ Queue"
                        }
                        button {
                            class: if *current_stage.read() == StageView::Equalizer { "btn-icon active" } else { "btn-icon" },
                            title: "Master Equalizer Rack",
                            onclick: move |_| navigate_to(StageView::Equalizer),
                            "🎚️"
                        }
                    }
                }

                // Stage View Content Scroll
                div { class: "content-scroll",
                    match current_stage.read().clone() {
                        StageView::Home => rsx! {
                            Home {
                                current_track: current_track.read().clone(),
                                is_playing: *is_playing.read(),
                                playback_pos_ms: *playback_pos_ms.read(),
                                playback_dur_ms: *playback_dur_ms.read(),
                                recent_tracks: recent_tracks.read().clone(),
                                harmonic_matches: harmonic_matches.clone(),
                                on_play_track: move |t| play_track(t),
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
                                on_play_track: move |t| play_track(t),
                                on_play_tracks: move |list| play_track_list(list),
                                on_edit_album: move |group| navigate_to(StageView::AlbumEditor(group)),
                            }
                        },
                        StageView::AlbumEditor(group) => rsx! {
                            AlbumEditor {
                                group,
                                on_back: move |_| navigate_to(StageView::Albums),
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
                                on_start_mix: move |list| play_track_list(list),
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
                        StageView::About => rsx! {
                            About {}
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
                                span { style: "font-weight: 700; font-size: 15px;", "Synchronized Lyrics" }
                                div { style: "display: flex; gap: 8px; align-items: center;",
                                    button {
                                        class: if *auto_scroll_lyrics.read() { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 11px; padding: 2px 8px;",
                                        onclick: move |_| {
                                            let curr = *auto_scroll_lyrics.read();
                                            auto_scroll_lyrics.set(!curr);
                                        },
                                        if *auto_scroll_lyrics.read() { "🔒 Auto-Scroll" } else { "🔓 Free-Scroll" }
                                    }
                                    button {
                                        class: "btn-icon",
                                        style: "width: 24px; height: 24px; font-size: 12px;",
                                        onclick: move |_| dock_mode.set(DockMode::Closed),
                                        "✕"
                                    }
                                }
                            }

                            div { class: "dock-scroll",
                                if lyrics_sheet.read().lines.is_empty() {
                                    div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; text-align: center; color: var(--text-muted); gap: 12px; padding: 40px 20px;",
                                        span { style: "font-size: 32px;", "📝" }
                                        p { style: "font-size: 15px; font-weight: 600;", "No Lyrics Available" }
                                        p { style: "font-size: 12px; line-height: 1.4;", "Place a companion .lrc file alongside your song to display synchronized singing lines." }
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
                                                        class: if is_active { "lyric-line active" } else { "lyric-line" },
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
                                span { style: "font-weight: 700; font-size: 15px;", "Up Next Queue" }
                                div { style: "display: flex; gap: 8px; align-items: center;",
                                    button {
                                        class: "btn-pill",
                                        style: "font-size: 11px; padding: 2px 8px;",
                                        onclick: move |_| {
                                            let mut q = queue.read().clone();
                                            q.clear();
                                            queue.set(q);
                                        },
                                        "Clear"
                                    }
                                    button {
                                        class: "btn-icon",
                                        style: "width: 24px; height: 24px; font-size: 12px;",
                                        onclick: move |_| dock_mode.set(DockMode::Closed),
                                        "✕"
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
                                    button {
                                        class: "btn-pill active",
                                        style: "width: 100%; margin-top: 8px; font-size: 11.5px; padding: 4px;",
                                        onclick: move |_| trigger_transition_now(),
                                        "⚡ Blend Now"
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
                // Scrubber Row: 0:42 ━━━━━━━━━━━━━━━━━━●────────────────────── -2:01
                div { class: "scrubber-row",
                    span { class: "time-label", "{elapsed_str}" }
                    input {
                        r#type: "range",
                        class: "scrubber-slider",
                        min: "0",
                        max: "{playback_dur_ms.read().max(1)}",
                        value: "{playback_pos_ms.read()}",
                        oninput: move |evt| {
                            if let Ok(val) = evt.value().parse::<u64>() {
                                playback_pos_ms.set(val);
                                player.read().seek_ms(val);
                            }
                        },
                    }
                    span { class: "time-label", style: "text-align: right;", "{remaining_str}" }
                }

                // Controls Row: Left = Album info, Center = Playback controls, Right = Actions & Volume
                div { class: "controls-row",
                    // Left: Mini Now Playing Pill
                    div { class: "transport-left",
                        if let Some((title, artist_album, art_src)) = mini_pill_info {
                            div { class: "mini-pill",
                                if !art_src.is_empty() {
                                    img { class: "mini-art", src: "{art_src}", alt: "{title}" }
                                } else {
                                    div {
                                        class: "mini-art",
                                        style: "display: flex; align-items: center; justify-content: center; font-size: 20px;",
                                        "🎵"
                                    }
                                }
                                div { class: "mini-meta",
                                    span { class: "mini-title", "{title}" }
                                    span { class: "mini-artist", "{artist_album}" }
                                }
                            }
                        } else {
                            div { class: "mini-pill",
                                span { style: "color: var(--text-muted); font-size: 13px;", "No track selected" }
                            }
                        }
                    }

                    // Center: Playback controls
                    div { class: "transport-center",
                        // Shuffle
                        button {
                            class: "btn-ctrl",
                            style: if *shuffle_mode.read() { "color: var(--accent-mint);" } else { "" },
                            title: "Smart DJ Shuffle",
                            onclick: move |_| {
                                let curr = *shuffle_mode.read();
                                shuffle_mode.set(!curr);
                            },
                            "🔀"
                        }
                        // Previous
                        button {
                            class: "btn-ctrl",
                            title: "Previous Track",
                            onclick: move |_| play_prev_track(),
                            "⏮"
                        }
                        // Play / Pause
                        button {
                            class: "btn-ctrl btn-play",
                            title: if *is_playing.read() { "Pause" } else { "Play" },
                            onclick: move |_| {
                                let p = player.read();
                                if *is_playing.read() { p.pause(); } else { p.play(); }
                            },
                            if *is_playing.read() { "⏸" } else { "▶" }
                        }
                        // Next / Transition
                        button {
                            class: "btn-ctrl",
                            title: "Next Track (Seamless Blend)",
                            onclick: move |_| trigger_transition_now(),
                            "⏭"
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
                            match *repeat_mode.read() {
                                RepeatMode::Off | RepeatMode::All => "🔁",
                                RepeatMode::One => "🔂",
                            }
                        }
                    }

                    // Right: Actions (Favorites, Volume, Fullscreen)
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
                            if current_track.read().as_ref().map(|t| favorites.read().contains(&t.path)).unwrap_or(false) { "♥" } else { "♡" }
                        }

                        // Volume Slider
                        span { style: "font-size: 14px; color: var(--text-muted);", "🔊" }
                        input {
                            r#type: "range",
                            class: "vol-slider",
                            min: "0",
                            max: "100",
                            value: "{volume_pct.read()}",
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

                        // Fullscreen / Stage Toggle
                        button {
                            class: "btn-ctrl",
                            title: "Toggle Now Playing",
                            onclick: move |_| navigate_to(StageView::Home),
                            "⛶"
                        }
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
                                    "✕"
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
        }
    }
}
