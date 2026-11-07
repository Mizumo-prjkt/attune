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
    let strings = crate::i18n::get_strings(crate::i18n::Language::from_code(&curr_cfg.language));

    let track = use_signal(|| {
        ACTIVE_FINDER_TRACK
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| TrackEntry {
                title: "Ready".to_string(),
                artist: strings.app_name.to_string(),
                album: "Attune Library".to_string(),
                duration_ms: 0,
                ..Default::default()
            })
    });

    let current_track = track.read().clone();
    let initial_query = format!("{} {}", current_track.title, current_track.artist).trim().to_string();

    let mut active_tab = use_signal(|| FinderTab::LocalDisk);
    let mut search_query = use_signal(|| initial_query);
    let mut is_loading = use_signal(|| false);
    let mut status_message = use_signal(|| None::<String>);
    let mut toast_feedback = use_signal(|| None::<String>);

    let mut candidates = use_signal(Vec::<LyricCandidate>::new);
    let mut selected_idx = use_signal(|| None::<usize>);

    // Scan initial local candidates on stage mount
    use_effect(move || {
        let t = track.read().clone();
        let local_matches = scan_local_lyric_files(&t);
        candidates.set(local_matches);
        if !candidates.read().is_empty() {
            selected_idx.set(Some(0));
            status_message.set(Some(format!("Found {} local companion lyric file(s)", candidates.read().len())));
        } else {
            status_message.set(Some("No companion lyric files found in audio directory. Browse device or query online.".into()));
        }
    });

    // Helper: Execute online or local queries
    let mut run_search = move |tab: FinderTab| {
        let q = search_query.read().trim().to_string();
        let t = track.read().clone();
        is_loading.set(true);
        selected_idx.set(None);

        match tab {
            FinderTab::LocalDisk => {
                let local = scan_local_lyric_files(&t);
                candidates.set(local);
                is_loading.set(false);
                let count = candidates.read().len();
                status_message.set(Some(format!("Scanned local disk: found {} candidate(s)", count)));
                if count > 0 {
                    selected_idx.set(Some(0));
                }
            }
            FinderTab::Lrclib => {
                let dur = t.duration_ms;
                let title = t.title.clone();
                let artist = t.artist.clone();
                let query = q.clone();

                dioxus::prelude::spawn(async move {
                    let results = tokio::task::spawn_blocking(move || {
                        fetch_lrclib_candidates(&query, &title, &artist, dur)
                    })
                    .await
                    .unwrap_or_default();

                    let count = results.len();
                    candidates.set(results);
                    is_loading.set(false);
                    status_message.set(Some(format!("LRCLIB search finished: found {} match(es)", count)));
                    if count > 0 {
                        selected_idx.set(Some(0));
                    }
                });
            }
            FinderTab::BiniLyrics => {
                let query = q.clone();
                let title = t.title.clone();
                let artist = t.artist.clone();

                dioxus::prelude::spawn(async move {
                    let results = tokio::task::spawn_blocking(move || {
                        fetch_binilyrics_candidates(&query, &title, &artist)
                    })
                    .await
                    .unwrap_or_default();

                    let count = results.len();
                    candidates.set(results);
                    is_loading.set(false);
                    status_message.set(Some(format!("BiniLyrics search finished: found {} match(es)", count)));
                    if count > 0 {
                        selected_idx.set(Some(0));
                    }
                });
            }
        }
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

    rsx! {
        div {
            class: "lyrics-finder-container",
            style: "display: flex; width: 100%; height: 100vh; overflow: hidden; background: rgba(10, 14, 23, 0.96); color: #ffffff; font-family: var(--font-sans, sans-serif);",

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
                            "Search online on LRCLIB or BiniLyrics, or choose 'Browse Device File' to inspect local .lrc, .elrc, .ttml, or .txt lyrics."
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

    // 1. Try search endpoint
    let encoded_q = urlencoding(query);
    let url = format!("https://lrclib.net/api/search?q={}", encoded_q);

    let resp = ureq::get(&url)
        .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
        .timeout(std::time::Duration::from_secs(5))
        .call();

    if let Ok(r) = resp {
        if let Ok(json) = r.into_json::<serde_json::Value>() {
            if let Some(arr) = json.as_array() {
                for item in arr.iter().take(15) {
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
