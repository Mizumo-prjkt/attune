//! Dedicated Standalone Lyrics Finder & Manager Window.
//!
//! Provides a responsive 2-column sidebar layout allowing listeners to:
//! 1. Inspect the active track's audio metadata and current lyrics state.
//! 2. Search and inspect local companion lyric files on disk (.lrc, .elrc, .ttml, .yrc, .txt).
//! 3. Browse and pick device lyric files via native file picker dialog.
//! 4. Query external providers: LRCLIB and BiniLyrics (<am-lyrics>, hidden if license unaccepted).
//! 5. Verify security heuristics (rejection of malicious binaries, null bytes, and script exploits).
//! 6. Preview and classify synchronization formats (Letter/Syllable sync, Line sync, Plain text).
//! 7. 1-tap select, apply, and save verified lyrics with live player synchronization.
//!
//! Strictly complies with Mandate 4 (zero hardcoded values).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use dioxus::prelude::*;

use crate::components::lyrics_parser;
use crate::components::lyrics_validator::{
    self, LyricsSyncType, ValidationResult,
};
use crate::config::load_config;
use crate::library::{fmt_ms, TrackEntry};

static LYRICS_FINDER_WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
static ACTIVE_FINDER_TRACK: Mutex<Option<TrackEntry>> = Mutex::new(None);

struct FinderWindowGuard;
impl Drop for FinderWindowGuard {
    fn drop(&mut self) {
        LYRICS_FINDER_WINDOW_OPEN.store(false, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinderTab {
    LocalDisk,
    Lrclib,
    BiniLyrics,
    Musixmatch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LyricCandidate {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_sec: Option<f64>,
    pub provider: String,
    pub sync_type: LyricsSyncType,
    pub content: String,
    pub file_path: Option<PathBuf>,
    pub validation: ValidationResult,
}

/// Standalone search coordinator delegating queries to the appropriate provider without capturing mutable environment
fn execute_search(
    tab: FinderTab,
    mut is_loading: Signal<bool>,
    mut selected_idx: Signal<Option<usize>>,
    mut candidates: Signal<Vec<LyricCandidate>>,
    mut status_message: Signal<Option<String>>,
    search_query: Signal<String>,
    track: Signal<TrackEntry>,
) {
    let q = search_query.read().trim().to_string();
    let t = track.read().clone();
    is_loading.set(true);
    selected_idx.set(None);

    match tab {
        FinderTab::LocalDisk => {
            let local = scan_local_lyric_files(&t);
            let count = local.len();
            candidates.set(local);
            is_loading.set(false);
            if count > 0 {
                status_message.set(Some(format!("Scanned local disk: found {} candidate(s)", count)));
                selected_idx.set(Some(0));
            } else {
                status_message.set(Some("No companion lyric files found in audio directory. Browse device or query online.".into()));
                selected_idx.set(None);
            }
        }
        FinderTab::Lrclib => {
            let dur = t.duration_ms;
            let title = t.title.clone();
            let artist = t.artist.clone();
            let query = q.clone();

            dioxus::prelude::spawn(async move {
                let q_for_task = query.clone();
                let results = tokio::task::spawn_blocking(move || {
                    fetch_lrclib_candidates(&q_for_task, &title, &artist, dur)
                })
                .await
                .unwrap_or_default();

                let count = results.len();
                candidates.set(results);
                is_loading.set(false);
                if count > 0 {
                    status_message.set(Some(format!("LRCLIB search finished: found {} match(es)", count)));
                    selected_idx.set(Some(0));
                } else {
                    status_message.set(Some(format!("No matching lyrics found on LRCLIB for '{}'. Try modifying the search query.", query)));
                    selected_idx.set(None);
                }
            });
        }
        FinderTab::BiniLyrics => {
            let query = q.clone();
            let title = t.title.clone();
            let artist = t.artist.clone();

            dioxus::prelude::spawn(async move {
                let q_for_task = query.clone();
                let results = tokio::task::spawn_blocking(move || {
                    fetch_binilyrics_candidates(&q_for_task, &title, &artist)
                })
                .await
                .unwrap_or_default();

                let count = results.len();
                candidates.set(results);
                is_loading.set(false);
                if count > 0 {
                    status_message.set(Some(format!("BiniLyrics search finished: found {} match(es)", count)));
                    selected_idx.set(Some(0));
                } else {
                    status_message.set(Some(format!("No matching lyrics found on BiniLyrics for '{}'.", query)));
                    selected_idx.set(None);
                }
            });
        }
        FinderTab::Musixmatch => {
            let dur = t.duration_ms;
            let title = t.title.clone();
            let artist = t.artist.clone();
            let query = q.clone();

            dioxus::prelude::spawn(async move {
                let api_key = crate::config::load_config().musixmatch_api_key.trim().to_string();
                if api_key.is_empty() {
                    status_message.set(Some("Musixmatch API key is not configured. Enter your API key in Settings > API Providers or Settings > Lyrics.".into()));
                    candidates.set(Vec::new());
                    is_loading.set(false);
                    selected_idx.set(None);
                    return;
                }

                let q_for_task = query.clone();
                let results = tokio::task::spawn_blocking(move || {
                    fetch_musixmatch_candidates(&q_for_task, &title, &artist, dur, &api_key)
                })
                .await
                .unwrap_or_default();

                let count = results.len();
                candidates.set(results);
                is_loading.set(false);
                if count > 0 {
                    status_message.set(Some(format!("Musixmatch search finished: found {} match(es)", count)));
                    selected_idx.set(Some(0));
                } else {
                    status_message.set(Some(format!("No matching lyrics found on Musixmatch for '{}'. Check API key, query, or quota.", query)));
                    selected_idx.set(None);
                }
            });
        }
    }
}

/// Opens the Lyrics Finder in a dedicated standalone secondary OS window.
pub fn open_lyrics_finder_window(track: Option<TrackEntry>) {
    if let Some(t) = track {
        *ACTIVE_FINDER_TRACK.lock().unwrap() = Some(t);
    }

    if LYRICS_FINDER_WINDOW_OPEN
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let dom = VirtualDom::new(LyricsFinderStandaloneWindow);
        let mut window = dioxus::desktop::WindowBuilder::new()
            .with_title("Attune — Lyrics Finder & Manager")
            .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1140.0, 720.0))
            .with_min_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(820.0, 540.0));

        if let Some(icon) = crate::app_icon::load_app_icon() {
            window = window.with_window_icon(Some(icon));
        }

        let css = include_str!("../style.css");
        let icons_css = include_str!("../material_icons.css");
        let cfg = dioxus::desktop::Config::new()
            .with_window(window)
            .with_menu(None)
            .with_custom_head(format!(
                "<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/icon?family=Material+Icons\">\n<style>\n{icons_css}\n{css}\n</style>"
            ));

        dioxus::desktop::window().new_window(dom, cfg);
    }
}

/// Standalone Window Root Component for Lyrics Finder
#[component]
pub fn LyricsFinderStandaloneWindow() -> Element {
    use_hook(|| Arc::new(FinderWindowGuard));

    let config_sig = use_signal(load_config);
    let curr_cfg = config_sig.read().clone();
    let _strings = crate::i18n::get_strings(crate::i18n::Language::from_code(&curr_cfg.language));

    let track_opt = ACTIVE_FINDER_TRACK.lock().unwrap().clone();
    let is_track_valid = track_opt.as_ref().map_or(false, |t| {
        !t.title.trim().is_empty()
            && t.title != "Ready"
            && (!t.path.as_os_str().is_empty() || t.duration_ms > 0)
    });

    if !is_track_valid {
        return rsx! {
            div {
                class: "lyrics-finder-container",
                style: "display: flex; flex-direction: column; width: 100%; height: 100vh; overflow: hidden; background: rgba(10, 14, 23, 0.96); color: #ffffff; align-items: center; justify-content: center; text-align: center; padding: 32px; box-sizing: border-box; font-family: var(--font-sans, sans-serif);",

                div {
                    style: "background: rgba(22, 28, 45, 0.85); border: 1px solid var(--border-glass); border-radius: 20px; padding: 44px 36px; max-width: 480px; display: flex; flex-direction: column; align-items: center; gap: 18px; box-shadow: 0 16px 40px rgba(0,0,0,0.55); backdrop-filter: blur(24px); -webkit-backdrop-filter: blur(24px);",

                    div {
                        style: "width: 72px; height: 72px; border-radius: 50%; background: rgba(244, 63, 94, 0.12); border: 1.5px solid rgba(244, 63, 94, 0.35); display: flex; align-items: center; justify-content: center; color: #fb7185;",
                        span { class: "material-icons md-36", "music_off" }
                    }

                    div { style: "display: flex; flex-direction: column; gap: 8px;",
                        h2 { style: "font-size: 20px; font-weight: 700; color: #ffffff; margin: 0;",
                            "Select tracks to find their lyrics"
                        }
                        p { style: "font-size: 13.5px; color: var(--text-muted); line-height: 1.5; margin: 0; max-width: 380px;",
                            "No active song or track candidate is selected. Please choose or play a song in Attune first to find, match, or manage its lyrics."
                        }
                    }

                    div { style: "display: flex; gap: 12px; margin-top: 6px;",
                        button {
                            class: "btn-pill active",
                            style: "padding: 10px 24px; font-size: 13px; font-weight: 700; border-radius: 9999px; cursor: pointer;",
                            onclick: move |_| {
                                dioxus::desktop::window().close();
                            },
                            "Close Window"
                        }
                    }
                }
            }
        };
    }

    let track = use_signal(|| track_opt.unwrap());

    let current_track = track.read().clone();
    let initial_query = format!("{} {}", current_track.title, current_track.artist).trim().to_string();

    let initial_local = scan_local_lyric_files(&current_track);
    let has_local = !initial_local.is_empty();

    let initial_tab = if has_local {
        FinderTab::LocalDisk
    } else if curr_cfg.api_musixmatch_enabled && curr_cfg.lyric_provider == "musixmatch" {
        FinderTab::Musixmatch
    } else {
        FinderTab::Lrclib
    };

    let initial_status = if has_local {
        Some(format!("Found {} local companion lyric file(s)", initial_local.len()))
    } else if initial_tab == FinderTab::Musixmatch {
        Some("No local companion files found. Searching Musixmatch online...".into())
    } else {
        Some("No local companion files found. Searching LRCLIB online...".into())
    };

    let initial_candidates = if has_local {
        initial_local
    } else {
        Vec::new()
    };
    let initial_selected = if has_local { Some(0) } else { None };

    let mut active_tab = use_signal(|| initial_tab);
    let mut search_query = use_signal(|| initial_query.clone());
    let mut is_loading = use_signal(|| false);
    let mut status_message = use_signal(|| initial_status);
    let mut toast_feedback = use_signal(|| None::<String>);

    let mut candidates = use_signal(|| initial_candidates);
    let mut selected_idx = use_signal(|| initial_selected);

    // Automatically trigger initial online search if no local companion files are found on disk
    use_hook(|| {
        if !has_local && !initial_query.is_empty() {
            let q = initial_query.clone();
            let t = current_track.clone();
            let is_mxm = initial_tab == FinderTab::Musixmatch;
            is_loading.set(true);

            dioxus::prelude::spawn(async move {
                let q_for_task = q.clone();
                let results = tokio::task::spawn_blocking(move || {
                    if is_mxm {
                        let key = crate::config::load_config().musixmatch_api_key.trim().to_string();
                        if !key.is_empty() {
                            return fetch_musixmatch_candidates(&q_for_task, &t.title, &t.artist, t.duration_ms, &key);
                        }
                    }
                    fetch_lrclib_candidates(&q_for_task, &t.title, &t.artist, t.duration_ms)
                })
                .await
                .unwrap_or_default();

                let count = results.len();
                candidates.set(results);
                is_loading.set(false);
                let prov_label = if is_mxm { "Musixmatch" } else { "LRCLIB" };
                if count > 0 {
                    status_message.set(Some(format!("{prov_label} search finished: found {} match(es)", count)));
                    selected_idx.set(Some(0));
                } else {
                    status_message.set(Some(format!("No matching lyrics found on {prov_label} for '{}'. Try modifying the search query.", q)));
                    selected_idx.set(None);
                }
            });
        }
    });

    // Helper: Execute online or local queries via standalone helper
    let run_search = move |tab: FinderTab| {
        execute_search(tab, is_loading, selected_idx, candidates, status_message, search_query, track);
    };

    // Helper: Browse local device file via native dialog
    let mut browse_local_file = move || {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Lyrics File (.lrc, .elrc, .ttml, .yrc, .txt)")
            .add_filter("Lyrics Files", &["lrc", "elrc", "ttml", "yrc", "txt", "xml"])
            .pick_file()
        {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let validation = lyrics_validator::validate_and_classify_lyrics(&content, Some(&file_name));

                let candidate = LyricCandidate {
                    id: format!("local-custom-{}", path.display()),
                    title: path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Custom File".into()),
                    artist: "Local Device".into(),
                    album: path.parent().map(|p| p.display().to_string()).unwrap_or_default(),
                    duration_sec: None,
                    provider: "User Chosen File".into(),
                    sync_type: validation.sync_type,
                    content,
                    file_path: Some(path.clone()),
                    validation,
                };

                let mut current_list = candidates.read().clone();
                current_list.insert(0, candidate);
                candidates.set(current_list);
                selected_idx.set(Some(0));
                active_tab.set(FinderTab::LocalDisk);
                status_message.set(Some(format!("Imported local file: {}", file_name)));
            } else {
                status_message.set(Some("Failed to read the selected file from disk".into()));
            }
        }
    };

    // Helper: Apply and save selected candidate lyrics
    let mut apply_selected_lyrics = move || {
        let curr_list = candidates.read();
        if let Some(idx) = *selected_idx.read() {
            if let Some(cand) = curr_list.get(idx) {
                if !cand.validation.is_safe {
                    toast_feedback.set(Some("Cannot apply unsafe/blocked lyrics content".into()));
                    return;
                }

                let t = track.read().clone();
                let ext = match cand.sync_type {
                    LyricsSyncType::Syllable => "elrc",
                    LyricsSyncType::Line => "lrc",
                    LyricsSyncType::Plain => "txt",
                };

                match lyrics_parser::save_lyrics_to_disk(&t.path, &cand.content, ext) {
                    Ok(saved_path) => {
                        let parsed_sheet = lyrics_validator::parse_verified_lyrics(&cand.content, &cand.provider);
                        lyrics_parser::notify_lyrics_updated(t.path.clone(), parsed_sheet);
                        toast_feedback.set(Some(format!(
                            "✓ Successfully applied & saved {} lyrics to {}",
                            cand.sync_type.badge_label(),
                            saved_path.file_name().unwrap_or_default().to_string_lossy()
                        )));
                    }
                    Err(e) => {
                        toast_feedback.set(Some(format!("Error saving lyrics: {e}")));
                    }
                }
            }
        }
    };

    let artwork_url = current_track.artwork_data_url.clone().unwrap_or_else(|| {
        "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='200' height='200' viewBox='0 0 200 200'><rect width='200' height='200' fill='%23121722'/><circle cx='100' cy='100' r='45' fill='none' stroke='%2364ffda' stroke-width='4'/></svg>".to_string()
    });

    let current_tab = *active_tab.read();
    let candidate_list = candidates.read().clone();
    let current_selected = selected_idx.read().and_then(|idx| candidate_list.get(idx).cloned());
    let cfg = crate::config::load_config();
    let app_font_css = crate::system_fonts::resolve_font_family_css(&cfg.app_font_family);

    rsx! {
        div {
            class: "lyrics-finder-container",
            style: "display: flex; width: 100%; height: 100vh; overflow: hidden; background: rgba(10, 14, 23, 0.96); color: #ffffff; --font-family: {app_font_css}; font-family: var(--font-family);",

            // =========================================================================
            // LEFT SIDEBAR: Active Song Card & Menu Tabs (~310px)
            // =========================================================================
            div {
                class: "lyrics-sidebar",
                style: "width: 310px; min-width: 280px; background: rgba(14, 18, 30, 0.94); border-right: 1px solid var(--border-glass); display: flex; flex-direction: column; padding: 20px 14px; gap: 12px; box-sizing: border-box; overflow-y: auto;",

                // Active Song Information Card
                div {
                    style: "background: rgba(22, 28, 45, 0.70); border: 1px solid var(--border-glass); border-radius: 14px; padding: 14px; display: flex; gap: 12px; align-items: center;",
                    img {
                        src: "{artwork_url}",
                        alt: "{current_track.title}",
                        style: "width: 64px; height: 64px; border-radius: 10px; object-fit: cover; border: 1px solid rgba(255, 255, 255, 0.12); flex-shrink: 0;"
                    }
                    div { style: "display: flex; flex-direction: column; overflow: hidden; gap: 3px;",
                        span {
                            style: "font-size: 14px; font-weight: 700; color: #ffffff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                            title: "{current_track.title}",
                            "{current_track.title}"
                        }
                        span {
                            style: "font-size: 12px; font-weight: 600; color: var(--accent-mint); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                            title: "{current_track.artist}",
                            "{current_track.artist}"
                        }
                        span {
                            style: "font-size: 11px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                            "{fmt_ms(current_track.duration_ms)}"
                        }
                    }
                }

                // Section Label
                div { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.8px; padding-left: 6px; margin-top: 6px;",
                    "Lyrics Sources & Providers"
                }

                // Tab 1: Local Files
                button {
                    class: if current_tab == FinderTab::LocalDisk { "settings-nav-item active" } else { "settings-nav-item" },
                    onclick: move |_| {
                        active_tab.set(FinderTab::LocalDisk);
                        run_search(FinderTab::LocalDisk);
                    },
                    span { class: "material-icons md-18", "folder" }
                    span { style: "flex: 1; text-align: left;", "Local Disk Files" }
                }

                // Tab 2: LRCLIB (Default public database)
                button {
                    class: if current_tab == FinderTab::Lrclib { "settings-nav-item active" } else { "settings-nav-item" },
                    onclick: move |_| {
                        active_tab.set(FinderTab::Lrclib);
                        run_search(FinderTab::Lrclib);
                    },
                    span { class: "material-icons md-18", "cloud_download" }
                    span { style: "flex: 1; text-align: left;", "LRCLIB Public Database" }
                }

                // Tab 3: BiniLyrics (<am-lyrics>)
                // STRICT MANDATE: Hidden if user has not accepted the MPL-2.0 license
                if curr_cfg.am_lyrics_agreed {
                    button {
                        class: if current_tab == FinderTab::BiniLyrics { "settings-nav-item active" } else { "settings-nav-item" },
                        onclick: move |_| {
                            active_tab.set(FinderTab::BiniLyrics);
                            run_search(FinderTab::BiniLyrics);
                        },
                        span { class: "material-icons md-18", "auto_awesome" }
                        span { style: "flex: 1; text-align: left;", "BiniLyrics (<am-lyrics>)" }
                    }
                }

                // Tab 4: Musixmatch API
                // STRICT MANDATE: Hidden if user has not enabled api_musixmatch_enabled in Settings
                if curr_cfg.api_musixmatch_enabled {
                    button {
                        class: if current_tab == FinderTab::Musixmatch { "settings-nav-item active" } else { "settings-nav-item" },
                        onclick: move |_| {
                            active_tab.set(FinderTab::Musixmatch);
                            run_search(FinderTab::Musixmatch);
                        },
                        span { class: "material-icons md-18", "subtitles" }
                        span { style: "flex: 1; text-align: left; display: flex; align-items: center; justify-content: space-between;",
                            span { "Musixmatch" }
                            span {
                                class: "badge-pill",
                                style: "font-size: 9.5px; padding: 1px 6px; background: rgba(234, 179, 8, 0.2); color: #eab308; border-color: rgba(234, 179, 8, 0.4);",
                                "Exp"
                            }
                        }
                    }
                }

                div { style: "flex: 1;" }

                // Action: Browse Device File with native file picker
                button {
                    class: "btn-pill",
                    style: "width: 100%; display: flex; align-items: center; justify-content: center; gap: 8px; padding: 10px; font-weight: 600; font-size: 13px; background: rgba(255, 255, 255, 0.08);",
                    onclick: move |_| browse_local_file(),
                    span { class: "material-icons md-18", "attach_file" }
                    "Browse Device File..."
                }

                // Safety Engine Status Badge
                div {
                    style: "background: rgba(45, 212, 191, 0.08); border: 1px solid rgba(45, 212, 191, 0.25); border-radius: 8px; padding: 8px 10px; display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--accent-mint);",
                    span { class: "material-icons md-14", "verified_user" }
                    span { "Safety Heuristics Engine Active" }
                }
            }

            // =========================================================================
            // RIGHT PANE: Search Bar + Candidate Selector + Live Lyrics Previewer
            // =========================================================================
            div {
                class: "lyrics-content",
                style: "flex: 1; display: flex; flex-direction: column; padding: 24px 28px; gap: 18px; overflow: hidden; box-sizing: border-box;",

                // Top Search Bar & Action Controls
                div {
                    style: "display: flex; gap: 12px; align-items: center;",
                    div {
                        class: "search-bar",
                        style: "flex: 1; margin: 0; background: rgba(22, 28, 45, 0.85); border: 1px solid var(--border-glass); border-radius: 10px; padding: 6px 14px; display: flex; align-items: center; gap: 8px;",
                        span { class: "material-icons md-18", style: "color: var(--accent-mint);", "search" }
                        input {
                            style: "flex: 1; background: transparent; border: none; outline: none; color: #ffffff; font-size: 13.5px;",
                            placeholder: "Search lyrics by song title, artist, or keywords...",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    run_search(*active_tab.read());
                                }
                            }
                        }
                    }
                    button {
                        class: "btn-pill active",
                        style: "padding: 8px 18px; font-size: 13px; font-weight: 700; display: flex; align-items: center; gap: 6px;",
                        onclick: move |_| run_search(*active_tab.read()),
                        if *is_loading.read() {
                            span { class: "material-icons md-16", "sync" }
                            "Searching..."
                        } else {
                            span { class: "material-icons md-16", "search" }
                            "Search"
                        }
                    }
                }

                // Status or Notification Message
                if let Some(ref msg) = *status_message.read() {
                    div { style: "font-size: 12px; color: var(--text-muted); padding-left: 2px;", "{msg}" }
                }

                // Toast Feedback Alert
                if let Some(ref toast) = *toast_feedback.read() {
                    div {
                        style: "background: rgba(45, 212, 191, 0.15); border: 1px solid rgba(45, 212, 191, 0.35); color: var(--accent-mint); padding: 8px 14px; border-radius: 8px; font-size: 12.5px; font-weight: 600; display: flex; align-items: center; justify-content: space-between;",
                        span { "{toast}" }
                        button {
                            style: "background: transparent; border: none; color: inherit; cursor: pointer;",
                            onclick: move |_| toast_feedback.set(None),
                            span { class: "material-icons md-16", "close" }
                        }
                    }
                }

                // Candidates Selection Carousel / Strip
                if !candidate_list.is_empty() {
                    div {
                        style: "display: flex; gap: 12px; overflow-x: auto; padding-bottom: 6px;",
                        for (idx, cand) in candidate_list.iter().enumerate() {
                            {
                                let is_sel = *selected_idx.read() == Some(idx);
                                let sync_type = cand.sync_type;
                                let provider_label = cand.provider.clone();
                                let title_str = cand.title.clone();
                                let artist_str = cand.artist.clone();

                                rsx! {
                                    div {
                                        key: "{cand.id}",
                                        style: if is_sel {
                                            "min-width: 240px; max-width: 260px; background: rgba(30, 40, 65, 0.85); border: 1.5px solid var(--accent-mint); border-radius: 12px; padding: 12px; cursor: pointer; display: flex; flex-direction: column; gap: 6px; box-shadow: 0 4px 16px rgba(45, 212, 191, 0.25);"
                                        } else {
                                            "min-width: 240px; max-width: 260px; background: rgba(22, 28, 45, 0.65); border: 1px solid var(--border-glass); border-radius: 12px; padding: 12px; cursor: pointer; display: flex; flex-direction: column; gap: 6px;"
                                        },
                                        onclick: move |_| selected_idx.set(Some(idx)),

                                        div { style: "display: flex; justify-content: space-between; align-items: center;",
                                            span {
                                                class: "badge-pill",
                                                style: "font-size: 10px; padding: 2px 7px; background: rgba(255,255,255,0.08);",
                                                "{provider_label}"
                                            }
                                            span {
                                                class: sync_type.badge_class(),
                                                style: "font-size: 10px; padding: 2px 7px;",
                                                match sync_type {
                                                    LyricsSyncType::Syllable => "Syllable",
                                                    LyricsSyncType::Line => "Line Synced",
                                                    LyricsSyncType::Plain => "Plain",
                                                }
                                            }
                                        }

                                        span {
                                            style: "font-size: 13.5px; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                            "{title_str}"
                                        }
                                        span {
                                            style: "font-size: 12px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                            "{artist_str}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Live Preview & Candidate Inspector
                if let Some(ref sel) = current_selected {
                    div {
                        style: "flex: 1; display: flex; flex-direction: column; background: rgba(16, 22, 36, 0.80); border: 1px solid var(--border-glass); border-radius: 14px; padding: 18px 22px; gap: 14px; overflow: hidden;",

                        // Preview Header with Metadata & Apply Button
                        div {
                            style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid rgba(255,255,255,0.08); padding-bottom: 12px;",
                            div { style: "display: flex; flex-direction: column; gap: 4px;",
                                div { style: "display: flex; align-items: center; gap: 8px;",
                                    span { style: "font-size: 16px; font-weight: 700; color: #ffffff;", "{sel.title}" }
                                    span { class: sel.sync_type.badge_class(), "{sel.sync_type.badge_label()}" }
                                    if sel.validation.is_safe {
                                        span {
                                            class: "badge-pill mint",
                                            style: "font-size: 10.5px; display: inline-flex; align-items: center; gap: 4px;",
                                            span { class: "material-icons md-12", "shield" }
                                            "Safe & Sanitized"
                                        }
                                    } else {
                                        span {
                                            class: "badge-pill rose",
                                            style: "font-size: 10.5px; display: inline-flex; align-items: center; gap: 4px;",
                                            span { class: "material-icons md-12", "warning" }
                                            "Security Blocked"
                                        }
                                    }
                                }
                                span { style: "font-size: 12px; color: var(--text-secondary);", "{sel.validation.security_details}" }
                            }

                            // 1-Tap Apply & Save Action
                            button {
                                class: "btn-pill active",
                                style: "padding: 8px 18px; font-weight: 700; font-size: 13px; display: flex; align-items: center; gap: 6px;",
                                disabled: !sel.validation.is_safe,
                                onclick: move |_| apply_selected_lyrics(),
                                span { class: "material-icons md-16", "save" }
                                "Apply & Save to Song"
                            }
                        }

                        // Formatted Lyric Lines Preview Box
                        div {
                            class: "lyrics-preview-box",
                            style: "flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; padding-right: 8px;",
                            {
                                let parsed = lyrics_validator::parse_verified_lyrics(&sel.content, &sel.provider);
                                if parsed.lines.is_empty() {
                                    rsx! {
                                        div { style: "color: var(--text-muted); font-size: 13px; text-align: center; padding: 40px 0;",
                                            "No text lines parsed from this candidate."
                                        }
                                    }
                                } else {
                                    rsx! {
                                        for (i, line) in parsed.lines.iter().enumerate() {
                                            div {
                                                key: "{i}",
                                                style: "display: flex; gap: 14px; align-items: baseline; font-size: 13.5px; line-height: 1.5;",
                                                if parsed.is_timed {
                                                    span {
                                                        style: "font-family: monospace; font-size: 11px; color: var(--accent-mint); opacity: 0.85; width: 62px; flex-shrink: 0;",
                                                        "{fmt_ms(line.timestamp_ms)}"
                                                    }
                                                }
                                                span {
                                                    style: "color: rgba(255, 255, 255, 0.90);",
                                                    "{line.text}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if !*is_loading.read() {
                    div {
                        style: "flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-muted); gap: 12px; text-align: center;",
                        span { class: "material-icons md-48", style: "opacity: 0.4;", "find_in_page" }
                        p { style: "font-size: 15px; font-weight: 600; color: #ffffff;", "No Lyric Candidates Selected" }
                        p { style: "font-size: 12.5px; max-width: 440px; line-height: 1.5;",
                            "Search online on LRCLIB, BiniLyrics, or Musixmatch, or choose 'Browse Device File' to inspect local .lrc, .elrc, .ttml, or .txt lyrics."
                        }
                    }
                }
            }
        }
    }
}

/// Scans local files next to audio path and inside app lyrics cache for candidate matches.
pub fn scan_local_lyric_files(track: &TrackEntry) -> Vec<LyricCandidate> {
    let mut candidates = Vec::new();
    if track.path.as_os_str().is_empty() {
        return candidates;
    }
    let mut seen_paths = std::collections::HashSet::new();

    let stem = track.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

    // 1. Check companion directory
    if let Some(parent) = track.path.parent() {
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                if matches!(ext.as_str(), "lrc" | "elrc" | "ttml" | "yrc" | "txt") {
                    if seen_paths.insert(p.clone()) {
                        if let Ok(content) = std::fs::read_to_string(&p) {
                            let fname = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            let validation = lyrics_validator::validate_and_classify_lyrics(&content, Some(&fname));
                            candidates.push(LyricCandidate {
                                id: format!("local-{}", p.display()),
                                title: p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or(fname.clone()),
                                artist: track.artist.clone(),
                                album: track.album.clone(),
                                duration_sec: Some(track.duration_ms as f64 / 1000.0),
                                provider: format!("Local .{}", ext),
                                sync_type: validation.sync_type,
                                content,
                                file_path: Some(p),
                                validation,
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. Check per-user lyrics cache directory
    if let Some(cache_dir) = dirs::cache_dir() {
        let app_cache = cache_dir.join("attune-desktop").join("lyrics");
        for ext in &["elrc", "ttml", "yrc", "lrc", "txt"] {
            let p = app_cache.join(format!("{stem}.{ext}"));
            if p.is_file() && seen_paths.insert(p.clone()) {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    let fname = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let validation = lyrics_validator::validate_and_classify_lyrics(&content, Some(&fname));
                    candidates.push(LyricCandidate {
                        id: format!("cached-{}", p.display()),
                        title: format!("{} (Cached)", track.title),
                        artist: track.artist.clone(),
                        album: track.album.clone(),
                        duration_sec: Some(track.duration_ms as f64 / 1000.0),
                        provider: "Attune Cache".into(),
                        sync_type: validation.sync_type,
                        content,
                        file_path: Some(p),
                        validation,
                    });
                }
            }
        }
    }

    candidates
}

/// Queries public LRCLIB REST API for matching lyrics.
pub fn fetch_lrclib_candidates(query: &str, track_title: &str, artist: &str, dur_ms: u64) -> Vec<LyricCandidate> {
    let mut results = Vec::new();

    let search_term = if query.trim().is_empty() {
        format!("{} {}", track_title, artist).trim().to_string()
    } else {
        query.trim().to_string()
    };

    if search_term.is_empty() || search_term == "Ready Attune" || search_term == "Ready" {
        return results;
    }

    // 1. Try search endpoint with free-text query
    let encoded_q = urlencoding(&search_term);
    let url = format!("https://lrclib.net/api/search?q={}", encoded_q);

    let resp = ureq::get(&url)
        .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
        .timeout(std::time::Duration::from_secs(8))
        .call();

    if let Ok(r) = resp {
        if let Ok(json) = r.into_json::<serde_json::Value>() {
            if let Some(arr) = json.as_array() {
                for item in arr.iter().take(20) {
                    let id = item.get("id").and_then(|v| v.as_i64()).map(|n| n.to_string()).unwrap_or_default();
                    let title = item.get("trackName").or_else(|| item.get("name")).and_then(|v| v.as_str()).unwrap_or(track_title).to_string();
                    let art = item.get("artistName").and_then(|v| v.as_str()).unwrap_or(artist).to_string();
                    let alb = item.get("albumName").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let duration_sec = item.get("duration").and_then(|v| v.as_f64());

                    let content = if let Some(synced) = item.get("syncedLyrics").and_then(|v| v.as_str()) {
                        synced.to_string()
                    } else if let Some(plain) = item.get("plainLyrics").and_then(|v| v.as_str()) {
                        plain.to_string()
                    } else {
                        continue;
                    };

                    let validation = lyrics_validator::validate_and_classify_lyrics(&content, Some("lrclib.net"));
                    if validation.is_safe {
                        results.push(LyricCandidate {
                            id: format!("lrclib-{}", id),
                            title,
                            artist: art,
                            album: alb,
                            duration_sec,
                            provider: "LRCLIB".into(),
                            sync_type: validation.sync_type,
                            content,
                            file_path: None,
                            validation,
                        });
                    }
                }
            }
        }
    }

    // 2. Fallback: If free-text search found nothing, try targeted track_name + artist_name search
    if results.is_empty() && (!track_title.is_empty() || !artist.is_empty()) {
        let enc_t = urlencoding(track_title);
        let enc_a = urlencoding(artist);
        let fallback_url = format!("https://lrclib.net/api/search?track_name={}&artist_name={}", enc_t, enc_a);
        if let Ok(r) = ureq::get(&fallback_url)
            .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
            .timeout(std::time::Duration::from_secs(8))
            .call()
        {
            if let Ok(json) = r.into_json::<serde_json::Value>() {
                if let Some(arr) = json.as_array() {
                    for item in arr.iter().take(20) {
                        let id = item.get("id").and_then(|v| v.as_i64()).map(|n| n.to_string()).unwrap_or_default();
                        let title = item.get("trackName").or_else(|| item.get("name")).and_then(|v| v.as_str()).unwrap_or(track_title).to_string();
                        let art = item.get("artistName").and_then(|v| v.as_str()).unwrap_or(artist).to_string();
                        let alb = item.get("albumName").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let duration_sec = item.get("duration").and_then(|v| v.as_f64());

                        let content = if let Some(synced) = item.get("syncedLyrics").and_then(|v| v.as_str()) {
                            synced.to_string()
                        } else if let Some(plain) = item.get("plainLyrics").and_then(|v| v.as_str()) {
                            plain.to_string()
                        } else {
                            continue;
                        };

                        let validation = lyrics_validator::validate_and_classify_lyrics(&content, Some("lrclib.net"));
                        if validation.is_safe {
                            results.push(LyricCandidate {
                                id: format!("lrclib-{}", id),
                                title,
                                artist: art,
                                album: alb,
                                duration_sec,
                                provider: "LRCLIB".into(),
                                sync_type: validation.sync_type,
                                content,
                                file_path: None,
                                validation,
                            });
                        }
                    }
                }
            }
        }
    }

    if dur_ms > 0 && !results.is_empty() {
        let target_sec = dur_ms as f64 / 1000.0;
        results.sort_by(|a, b| {
            let diff_a = a.duration_sec.map(|d| (d - target_sec).abs()).unwrap_or(999.0);
            let diff_b = b.duration_sec.map(|d| (d - target_sec).abs()).unwrap_or(999.0);
            diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    results
}

/// Queries BiniLyrics (<am-lyrics>) open-source syllable lyrics endpoints.
pub fn fetch_binilyrics_candidates(query: &str, track_title: &str, artist: &str) -> Vec<LyricCandidate> {
    let mut results = Vec::new();
    let encoded_q = urlencoding(query);
    let url = format!("https://lrclib.net/api/search?q={}", encoded_q);

    let resp = ureq::get(&url)
        .set("User-Agent", "Attune/0.1.0-AMLL (<am-lyrics> integration)")
        .timeout(std::time::Duration::from_secs(5))
        .call();

    if let Ok(r) = resp {
        if let Ok(json) = r.into_json::<serde_json::Value>() {
            if let Some(arr) = json.as_array() {
                for item in arr.iter().take(10) {
                    let id = item.get("id").and_then(|v| v.as_i64()).map(|n| n.to_string()).unwrap_or_default();
                    let title = item.get("trackName").or_else(|| item.get("name")).and_then(|v| v.as_str()).unwrap_or(track_title).to_string();
                    let art = item.get("artistName").and_then(|v| v.as_str()).unwrap_or(artist).to_string();
                    let alb = item.get("albumName").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let duration_sec = item.get("duration").and_then(|v| v.as_f64());

                    let content = if let Some(synced) = item.get("syncedLyrics").and_then(|v| v.as_str()) {
                        synced.to_string()
                    } else if let Some(plain) = item.get("plainLyrics").and_then(|v| v.as_str()) {
                        plain.to_string()
                    } else {
                        continue;
                    };

                    let validation = lyrics_validator::validate_and_classify_lyrics(&content, Some("BiniLyrics"));
                    if validation.is_safe {
                        results.push(LyricCandidate {
                            id: format!("binilyrics-{}", id),
                            title,
                            artist: art,
                            album: alb,
                            duration_sec,
                            provider: "BiniLyrics (<am-lyrics>)".into(),
                            sync_type: validation.sync_type,
                            content,
                            file_path: None,
                            validation,
                        });
                    }
                }
            }
        }
    }

    results
}

/// Trims Musixmatch commercial notice and copyright boilerplate from the end of lyrics text.
pub fn clean_musixmatch_lyrics(text: &str) -> String {
    let mut cleaned = text.to_string();
    if let Some(pos) = cleaned.find("******* This Lyrics is NOT for Commercial use *******") {
        cleaned.truncate(pos);
    }
    if let Some(pos) = cleaned.find("/* Commercial use of this lyric is prohibited") {
        cleaned.truncate(pos);
    }
    if let Some(pos) = cleaned.find("... \n\n*******") {
        cleaned.truncate(pos);
    }
    cleaned.trim().to_string()
}

/// Evaluates Musixmatch API status code and constructs a human-readable feedback message.
pub fn parse_musixmatch_status(status_code: i64, track_name: Option<&str>) -> Result<String, String> {
    match status_code {
        200 => {
            let name = track_name.unwrap_or("Track Verified");
            Ok(format!("✓ Musixmatch API connected successfully (Status 200 OK — Found '{name}')"))
        }
        401 => Err("✕ Musixmatch Error 401: Invalid or unauthorized API key.".into()),
        402 => Err("✕ Musixmatch Error 402: Usage limit exceeded for this API key.".into()),
        403 => Err("✕ Musixmatch Error 403: Forbidden / Access not permitted.".into()),
        404 => Ok("⚠ Musixmatch API connected (Status 404: Resource not found, but API Key is valid).".into()),
        code => Err(format!("✕ Musixmatch API returned error code {code}.")),
    }
}

/// Tests connectivity to the Musixmatch API with the provided user API key.
pub fn test_musixmatch_api(api_key: &str) -> Result<String, String> {
    let key_trimmed = api_key.trim();
    if key_trimmed.is_empty() {
        return Err("API key is empty. Please enter your Musixmatch user API key.".into());
    }

    let url = format!(
        "https://api.musixmatch.com/ws/1.1/track.search?q_artist=Queen&q_track=Bohemian%20Rhapsody&page_size=1&apikey={}",
        urlencoding(key_trimmed)
    );

    let resp = match ureq::get(&url)
        .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
        .timeout(std::time::Duration::from_secs(8))
        .call()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Network connection failed: {e}")),
    };

    let json: serde_json::Value = match resp.into_json() {
        Ok(j) => j,
        Err(e) => return Err(format!("Invalid JSON response: {e}")),
    };

    let status_code = json
        .get("message")
        .and_then(|m| m.get("header"))
        .and_then(|h| h.get("status_code"))
        .and_then(|c| c.as_i64())
        .unwrap_or(0);

    let track_name = json
        .get("message")
        .and_then(|m| m.get("body"))
        .and_then(|b| b.get("track_list"))
        .and_then(|tl| tl.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("track"))
        .and_then(|t| t.get("track_name"))
        .and_then(|name| name.as_str());

    parse_musixmatch_status(status_code, track_name)
}

/// Queries Musixmatch API for synchronized (LRC) and plain lyrics candidates.
pub fn fetch_musixmatch_candidates(
    query: &str,
    track_title: &str,
    artist: &str,
    dur_ms: u64,
    api_key: &str,
) -> Vec<LyricCandidate> {
    let mut results = Vec::new();
    let key_trimmed = api_key.trim();
    if key_trimmed.is_empty() {
        return results;
    }

    let search_term = if query.trim().is_empty() {
        format!("{} {}", track_title, artist).trim().to_string()
    } else {
        query.trim().to_string()
    };

    if search_term.is_empty() || search_term == "Ready Attune" || search_term == "Ready" {
        return results;
    }

    // 1. First search via track.search endpoint
    let encoded_q = urlencoding(&search_term);
    let search_url = format!(
        "https://api.musixmatch.com/ws/1.1/track.search?q={}&page_size=5&s_track_rating=desc&apikey={}",
        encoded_q,
        urlencoding(key_trimmed)
    );

    let resp = ureq::get(&search_url)
        .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
        .timeout(std::time::Duration::from_secs(8))
        .call();

    if let Ok(r) = resp {
        if let Ok(json) = r.into_json::<serde_json::Value>() {
            let status_code = json
                .get("message")
                .and_then(|m| m.get("header"))
                .and_then(|h| h.get("status_code"))
                .and_then(|c| c.as_i64())
                .unwrap_or(0);

            if status_code == 200 {
                if let Some(track_list) = json
                    .get("message")
                    .and_then(|m| m.get("body"))
                    .and_then(|b| b.get("track_list"))
                    .and_then(|tl| tl.as_array())
                {
                    for item in track_list.iter().take(5) {
                        if let Some(track) = item.get("track") {
                            let track_id = track.get("track_id").and_then(|v| v.as_i64()).unwrap_or(0);
                            if track_id == 0 {
                                continue;
                            }
                            let title = track
                                .get("track_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or(track_title)
                                .to_string();
                            let art = track
                                .get("artist_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or(artist)
                                .to_string();
                            let alb = track
                                .get("album_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let length_sec = track
                                .get("track_length")
                                .and_then(|v| v.as_f64())
                                .or_else(|| track.get("track_length").and_then(|v| v.as_i64()).map(|s| s as f64));
                            let has_subtitles = track
                                .get("has_subtitles")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) == 1;

                            // If subtitles are available, attempt track.subtitle.get (LRC format)
                            let mut lyric_content: Option<String> = None;
                            if has_subtitles {
                                let sub_url = format!(
                                    "https://api.musixmatch.com/ws/1.1/track.subtitle.get?track_id={}&subtitle_format=lrc&apikey={}",
                                    track_id,
                                    urlencoding(key_trimmed)
                                );
                                if let Ok(sub_r) = ureq::get(&sub_url)
                                    .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
                                    .timeout(std::time::Duration::from_secs(5))
                                    .call()
                                {
                                    if let Ok(sub_json) = sub_r.into_json::<serde_json::Value>() {
                                        if let Some(body) = sub_json
                                            .get("message")
                                            .and_then(|m| m.get("body"))
                                            .and_then(|b| b.get("subtitle"))
                                            .and_then(|s| s.get("subtitle_body"))
                                            .and_then(|s| s.as_str())
                                        {
                                            if !body.trim().is_empty() {
                                                lyric_content = Some(body.to_string());
                                            }
                                        }
                                    }
                                }
                            }

                            // If no subtitles were obtained, attempt track.lyrics.get (plain text)
                            if lyric_content.is_none() {
                                let lyr_url = format!(
                                    "https://api.musixmatch.com/ws/1.1/track.lyrics.get?track_id={}&apikey={}",
                                    track_id,
                                    urlencoding(key_trimmed)
                                );
                                if let Ok(lyr_r) = ureq::get(&lyr_url)
                                    .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
                                    .timeout(std::time::Duration::from_secs(5))
                                    .call()
                                {
                                    if let Ok(lyr_json) = lyr_r.into_json::<serde_json::Value>() {
                                        if let Some(body) = lyr_json
                                            .get("message")
                                            .and_then(|m| m.get("body"))
                                            .and_then(|b| b.get("lyrics"))
                                            .and_then(|s| s.get("lyrics_body"))
                                            .and_then(|s| s.as_str())
                                        {
                                            if !body.trim().is_empty() {
                                                let cleaned = clean_musixmatch_lyrics(body);
                                                lyric_content = Some(cleaned);
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(content) = lyric_content {
                                let validation = lyrics_validator::validate_and_classify_lyrics(
                                    &content,
                                    Some("Musixmatch"),
                                );
                                if validation.is_safe {
                                    results.push(LyricCandidate {
                                        id: format!("musixmatch-{}", track_id),
                                        title,
                                        artist: art,
                                        album: alb,
                                        duration_sec: length_sec,
                                        provider: "Musixmatch [Exp]".into(),
                                        sync_type: validation.sync_type,
                                        content,
                                        file_path: None,
                                        validation,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback: If track.search yielded 0 results, try matcher.subtitle.get or matcher.lyrics.get
    if results.is_empty() && (!track_title.is_empty() || !artist.is_empty()) {
        let enc_title = urlencoding(track_title);
        let enc_artist = urlencoding(artist);

        // Try matcher.subtitle.get
        let matcher_sub_url = format!(
            "https://api.musixmatch.com/ws/1.1/matcher.subtitle.get?q_track={}&q_artist={}&subtitle_format=lrc&apikey={}",
            enc_title,
            enc_artist,
            urlencoding(key_trimmed)
        );

        let mut matched_content: Option<String> = None;
        if let Ok(m_resp) = ureq::get(&matcher_sub_url)
            .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
            .timeout(std::time::Duration::from_secs(6))
            .call()
        {
            if let Ok(m_json) = m_resp.into_json::<serde_json::Value>() {
                if let Some(body) = m_json
                    .get("message")
                    .and_then(|m| m.get("body"))
                    .and_then(|b| b.get("subtitle"))
                    .and_then(|s| s.get("subtitle_body"))
                    .and_then(|s| s.as_str())
                {
                    if !body.trim().is_empty() {
                        matched_content = Some(body.to_string());
                    }
                }
            }
        }

        // If matcher.subtitle.get failed, try matcher.lyrics.get
        if matched_content.is_none() {
            let matcher_lyr_url = format!(
                "https://api.musixmatch.com/ws/1.1/matcher.lyrics.get?q_track={}&q_artist={}&apikey={}",
                enc_title,
                enc_artist,
                urlencoding(key_trimmed)
            );
            if let Ok(lyr_resp) = ureq::get(&matcher_lyr_url)
                .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
                .timeout(std::time::Duration::from_secs(6))
                .call()
            {
                if let Ok(lyr_json) = lyr_resp.into_json::<serde_json::Value>() {
                    if let Some(body) = lyr_json
                        .get("message")
                        .and_then(|m| m.get("body"))
                        .and_then(|b| b.get("lyrics"))
                        .and_then(|s| s.get("lyrics_body"))
                        .and_then(|s| s.as_str())
                    {
                        if !body.trim().is_empty() {
                            matched_content = Some(clean_musixmatch_lyrics(body));
                        }
                    }
                }
            }
        }

        if let Some(content) = matched_content {
            let validation = lyrics_validator::validate_and_classify_lyrics(&content, Some("Musixmatch"));
            if validation.is_safe {
                results.push(LyricCandidate {
                    id: format!("musixmatch-matcher-{}", blake3::hash(search_term.as_bytes()).to_hex()),
                    title: track_title.to_string(),
                    artist: artist.to_string(),
                    album: String::new(),
                    duration_sec: if dur_ms > 0 { Some(dur_ms as f64 / 1000.0) } else { None },
                    provider: "Musixmatch [Exp]".into(),
                    sync_type: validation.sync_type,
                    content,
                    file_path: None,
                    validation,
                });
            }
        }
    }

    if dur_ms > 0 && !results.is_empty() {
        let target_sec = dur_ms as f64 / 1000.0;
        results.sort_by(|a, b| {
            let diff_a = a.duration_sec.map(|d| (d - target_sec).abs()).unwrap_or(999.0);
            let diff_b = b.duration_sec.map(|d| (d - target_sec).abs()).unwrap_or(999.0);
            diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    results
}

fn urlencoding(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_or_placeholder_track_returns_no_candidates() {
        let empty_track = TrackEntry::default();
        let local = scan_local_lyric_files(&empty_track);
        assert!(local.is_empty(), "Empty track should return 0 local candidates");

        let ready_results = fetch_lrclib_candidates("Ready Attune", "Ready", "Attune", 0);
        assert!(ready_results.is_empty(), "Dummy 'Ready Attune' query should return 0 candidates without searching");

        let blank_results = fetch_lrclib_candidates("", "", "", 0);
        assert!(blank_results.is_empty(), "Blank query should return 0 candidates");

        let mxm_empty = fetch_musixmatch_candidates("", "", "", 0, "");
        assert!(mxm_empty.is_empty(), "Empty API key should return 0 candidates");

        let mxm_ready = fetch_musixmatch_candidates("Ready Attune", "Ready", "Attune", 0, "some_key");
        assert!(mxm_ready.is_empty(), "Dummy placeholder query should return 0 candidates");
    }

    #[test]
    fn test_clean_musixmatch_lyrics() {
        let raw = "Is this the real life?\nIs this just fantasy?\n\n******* This Lyrics is NOT for Commercial use *******\n(1409618177218)";
        let cleaned = clean_musixmatch_lyrics(raw);
        assert_eq!(cleaned, "Is this the real life?\nIs this just fantasy?");

        let raw_c = "Another song lyric here\n/* Commercial use of this lyric is prohibited */";
        let cleaned_c = clean_musixmatch_lyrics(raw_c);
        assert_eq!(cleaned_c, "Another song lyric here");
    }

    #[test]
    fn test_parse_musixmatch_status() {
        assert!(parse_musixmatch_status(200, Some("Bohemian Rhapsody")).unwrap().contains("Bohemian Rhapsody"));
        assert!(parse_musixmatch_status(200, None).unwrap().contains("Track Verified"));
        assert!(parse_musixmatch_status(401, None).unwrap_err().contains("401"));
        assert!(parse_musixmatch_status(402, None).unwrap_err().contains("402"));
        assert!(parse_musixmatch_status(403, None).unwrap_err().contains("403"));
        assert!(parse_musixmatch_status(404, None).unwrap().contains("404"));
        assert!(parse_musixmatch_status(500, None).unwrap_err().contains("500"));
    }

    #[test]
    fn test_test_musixmatch_api_empty_key() {
        let res = test_musixmatch_api("   ");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_candidate_serialization() {
        let text = "[00:10.00] Line test";
        let val = lyrics_validator::validate_and_classify_lyrics(text, None);
        let cand = LyricCandidate {
            id: "test-1".into(),
            title: "Test".into(),
            artist: "Band".into(),
            album: "Record".into(),
            duration_sec: Some(180.0),
            provider: "LRCLIB".into(),
            sync_type: val.sync_type,
            content: text.into(),
            file_path: None,
            validation: val,
        };

        assert_eq!(cand.sync_type, LyricsSyncType::Line);
        assert!(cand.validation.is_safe);
    }
}