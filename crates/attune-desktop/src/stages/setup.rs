//! Setup Stage: Comprehensive first-run setup & configuration wizard for Attune Desktop.
//!
//! Provides:
//! - Automatic detection & warning if a previous legacy Flutter installation's incompatible
//!   data/cache is present on the system, with 1-click cleanup.
//! - Step 1: Music Library directory selection, extra folders & duration filter.
//! - Step 2: Audio Engine & Output configuration (Codec Engine, 32-bit Float PCM, Skip Silence, Live DAC Diagnostics).
//! - Step 3: Lyrics configuration (Priority chain, LRCLIB / BiniLyrics / Musixmatch provider, Alignment, Glow & Animation Presets).
//! - Step 4: API Providers configuration (YouTube Music Innertube, HD Artwork, Wikipedia/MusicBrainz, Experimental Musixmatch API Key & Test Query).
//! - Step 5: Font & Typography configuration (Application UI Font Family, Lyrics Font Family, Font Sizes & Live Previews).
//! - Step 6: Full 5-layer audio profiling (Camelot harmonic keys via CQT, nominal BPM, 5-band energy, aggression index) & Launch.

use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::config::{self, AppConfig};
use crate::library::{self, IndexingProgressEvent, TrackEntry};

/// Step metadata for the 6-stage onboarding wizard.
pub const SETUP_STEPS: &[(usize, &str, &str)] = &[
    (1, "1. Library", "folder_open"),
    (2, "2. Audio", "graphic_eq"),
    (3, "3. Lyrics", "lyrics"),
    (4, "4. APIs", "cloud_sync"),
    (5, "5. Fonts", "font_download"),
    (6, "6. Profile & Launch", "rocket_launch"),
];

/// Reorders the lyric priority vector so `preferred` becomes `#1` while preserving all 5 standard tiers.
pub fn reorder_lyric_priority(preferred: &str) -> Vec<String> {
    let mut list = vec![preferred.to_string()];
    for item in &["embedded", "lrc", "ttml", "elrc", "external"] {
        if *item != preferred && !list.iter().any(|x| x == item) {
            list.push((*item).to_string());
        }
    }
    list
}

#[component]
pub fn SetupStage(
    initial_dir: Option<PathBuf>,
    on_complete: EventHandler<Vec<TrackEntry>>,
) -> Element {
    let mut cfg_sig = use_signal(config::load_config);

    let mut selected_dir = use_signal(|| {
        initial_dir.unwrap_or_else(|| {
            let (dir, _) = config::resolve_music_dir(None);
            dir.unwrap_or_else(|| {
                dirs::audio_dir()
                    .or_else(dirs::home_dir)
                    .unwrap_or_else(|| PathBuf::from("."))
            })
        })
    });

    // Detect legacy Flutter installation artifacts on startup
    let mut legacy_flutter_paths = use_signal(config::detect_legacy_flutter_install);
    let mut legacy_flutter_dismissed = use_signal(|| false);
    let mut legacy_purge_feedback = use_signal(|| None::<(bool, String)>);

    // Wizard step state (1..=6)
    let mut current_step = use_signal(|| 1usize);
    let mut show_am_license_prompt = use_signal(|| false);
    let mut mxm_test_loading = use_signal(|| false);
    let mut mxm_test_status = use_signal(|| None::<(bool, String)>);

    // Hardware output driver diagnostics for Step 2 (Audio)
    let mut driver_diag = use_signal(crate::player::get_driver_diagnostics);

    // Indexing & 5-Layer DSP Profiling state for Step 6
    let mut is_indexing = use_signal(|| false);
    let mut indexing_done = use_signal(|| false);
    let mut progress_pct = use_signal(|| 0.0f32);
    let mut current_index = use_signal(|| 0usize);
    let mut total_tracks = use_signal(|| 0usize);
    let mut current_track_info = use_signal(|| None::<(String, String, Option<f32>, Option<String>, String)>);
    let mut indexed_tracks = use_signal(Vec::<TrackEntry>::new);
    let mut lossless_count = use_signal(|| 0usize);
    let mut status_note = use_signal(|| "Ready to profile your music collection".to_string());

    let mut index_rx = use_signal(|| Arc::new(Mutex::new(None::<std::sync::mpsc::Receiver<IndexingProgressEvent>>)));

    // Polling effect for background indexing events
    use_future(move || async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            if *is_indexing.read() {
                let maybe_msg = {
                    let guard = index_rx.read();
                    let lock = guard.lock().unwrap();
                    if let Some(ref rx) = *lock {
                        rx.try_recv().ok()
                    } else {
                        None
                    }
                };

                if let Some(ev) = maybe_msg {
                    match ev {
                        IndexingProgressEvent::Discovered { total_files } => {
                            total_tracks.set(total_files);
                            status_note.set(format!(
                                "Discovered {total_files} audio files. Starting 5-layer DSP analysis..."
                            ));
                        }
                        IndexingProgressEvent::Progress {
                            current,
                            total,
                            track_title,
                            track_artist,
                            bpm,
                            key,
                            format,
                        } => {
                            current_index.set(current);
                            total_tracks.set(total);
                            let pct = if total > 0 {
                                ((current as f32 / total as f32) * 100.0).clamp(0.0, 100.0)
                            } else {
                                0.0
                            };
                            progress_pct.set(pct);
                            current_track_info.set(Some((track_title, track_artist, bpm, key, format)));
                            status_note.set(format!("Profiling track {current} of {total}"));
                        }
                        IndexingProgressEvent::Finished {
                            tracks,
                            lossless_count: lc,
                        } => {
                            lossless_count.set(lc);
                            let count = tracks.len();
                            indexed_tracks.set(tracks);
                            is_indexing.set(false);
                            indexing_done.set(true);
                            progress_pct.set(100.0);
                            status_note.set(format!("✓ All {count} tracks successfully indexed!"));
                        }
                        IndexingProgressEvent::Error(err) => {
                            is_indexing.set(false);
                            status_note.set(format!("Error during profiling: {err}"));
                        }
                    }
                }
            }
        }
    });

    // Helper to mutate and persist AppConfig immediately
    let mut update_cfg = move |mut modifier: Box<dyn FnMut(&mut AppConfig)>| {
        let mut c = cfg_sig.read().clone();
        modifier(&mut c);
        config::save_config(&c);
        cfg_sig.set(c);
    };

    let curr_cfg = cfg_sig.read().clone();
    let app_font_css = crate::system_fonts::resolve_font_family_css(&curr_cfg.app_font_family);
    let current_app_font = curr_cfg.app_font_family.clone();
    let preview_app_font_css = crate::system_fonts::resolve_font_family_css(&current_app_font);
    let current_lyric_font = if curr_cfg.lyric_font_family.is_empty() || curr_cfg.lyric_font_family == "System Default" {
        "system".to_string()
    } else {
        curr_cfg.lyric_font_family.clone()
    };
    let preview_lyric_font_css = crate::system_fonts::resolve_font_family_css(&curr_cfg.lyric_font_family);
    let system_fonts = crate::system_fonts::get_available_system_fonts();
    let default_font_group = crate::system_fonts::pango_escape("Attune Curated Fonts");
    let system_font_group = format!(
        "{} ({})",
        crate::system_fonts::pango_escape("System Installed Fonts"),
        system_fonts.len()
    );
    let dir_val = selected_dir.read().clone();
    let dir_display = dir_val.display().to_string();
    let step_now = *current_step.read();
    let diag = driver_diag.read().clone();
    let detected_flutter = legacy_flutter_paths.read().clone();

    rsx! {
        div {
            class: "setup-stage",
            style: "display: flex; flex-direction: column; align-items: center; justify-content: flex-start; min-height: 85vh; max-width: 880px; margin: 0 auto; width: 100%; padding: 28px 24px 48px 24px; --font-family: {app_font_css}; font-family: var(--font-family);",

            // =====================================================================
            // LEGACY FLUTTER INSTALLATION INCOMPATIBILITY WARNING BANNER
            // =====================================================================
            if !detected_flutter.is_empty() && !*legacy_flutter_dismissed.read() {
                div {
                    style: "width: 100%; margin-bottom: 20px; padding: 18px 22px; border-radius: 18px; background: rgba(245, 158, 11, 0.12); border: 1px solid rgba(245, 158, 11, 0.45); box-shadow: 0 12px 32px rgba(0,0,0,0.35); display: flex; flex-direction: column; gap: 10px;",
                    div { style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 12px;",
                        div { style: "display: flex; align-items: center; gap: 10px;",
                            span { class: "material-icons md-24", style: "color: #fbbf24;", "warning_amber" }
                            div {
                                h3 { style: "margin: 0; font-size: 15px; font-weight: 700; color: #fde68a;",
                                    "Previous Flutter Installation Detected — Data Incompatibility Notice"
                                }
                                p { style: "margin: 4px 0 0 0; font-size: 12.5px; color: rgba(255,255,255,0.85); line-height: 1.45;",
                                    "Attune detected application data or runtime files from a legacy Flutter version. Preferences, caches, and state from the Flutter version are NOT compatible with the Native Rust engine (attune-desktop)."
                                }
                            }
                        }
                        button {
                            class: "btn-pill",
                            style: "font-size: 11px; padding: 4px 10px; flex-shrink: 0;",
                            onclick: move |_| legacy_flutter_dismissed.set(true),
                            "Dismiss"
                        }
                    }

                    div {
                        style: "background: rgba(0,0,0,0.30); border-radius: 10px; padding: 8px 12px; font-family: monospace; font-size: 11.5px; color: #fcd34d; max-height: 90px; overflow-y: auto;",
                        for p in detected_flutter.iter() {
                            div { key: "{p.display()}", "• {p.display()}" }
                        }
                    }

                    div { style: "display: flex; align-items: center; gap: 12px; flex-wrap: wrap;",
                        button {
                            class: "btn-pill active",
                            style: "background: #f59e0b; color: #000; font-weight: 700; font-size: 12px; padding: 7px 16px; display: inline-flex; align-items: center; gap: 6px; border: none;",
                            onclick: move |_| {
                                let paths = legacy_flutter_paths.read().clone();
                                match config::purge_legacy_flutter_paths(&paths) {
                                    Ok(n) => {
                                        legacy_purge_feedback.set(Some((true, format!("✓ Cleaned up {n} legacy Flutter item(s)."))));
                                        legacy_flutter_paths.set(config::detect_legacy_flutter_install());
                                    }
                                    Err(err) => {
                                        legacy_purge_feedback.set(Some((false, err)));
                                    }
                                }
                            },
                            span { class: "material-icons md-16", "delete_sweep" }
                            "Purge Incompatible Legacy Flutter Data"
                        }
                        if let Some((ok, ref msg)) = *legacy_purge_feedback.read() {
                            span {
                                style: if ok { "font-size: 12px; color: var(--accent-mint); font-weight: 600;" } else { "font-size: 12px; color: #fb7185; font-weight: 600;" },
                                "{msg}"
                            }
                        }
                    }
                }
            }

            // =====================================================================
            // MAIN LIQUID MATERIAL GLASS SETUP CARD
            // =====================================================================
            div {
                style: "background: var(--bg-card); border: 1px solid var(--border-glass); border-radius: 28px; padding: 34px 38px; width: 100%; box-shadow: 0 24px 48px rgba(0,0,0,0.5); backdrop-filter: blur(40px); display: flex; flex-direction: column; align-items: center; text-align: center; box-sizing: border-box;",

                // Hero Logo Disc + Header
                div { style: "display: flex; align-items: center; gap: 16px; margin-bottom: 18px;",
                    div { style: "width: 56px; height: 56px; border-radius: 50%; background: linear-gradient(135deg, rgba(45, 212, 191, 0.25), rgba(168, 85, 247, 0.25)); border: 2px solid var(--accent-mint); display: flex; align-items: center; justify-content: center; box-shadow: 0 0 24px rgba(45, 212, 191, 0.3); flex-shrink: 0; overflow: hidden;",
                        img {
                            src: "{crate::app_icon::app_icon_data_url()}",
                            style: "width: 38px; height: 38px; object-fit: contain; filter: drop-shadow(0 2px 8px rgba(45, 212, 191, 0.45));",
                            alt: "Attune Logo",
                        }
                    }
                    div { style: "text-align: left;",
                        h1 { style: "font-size: 24px; font-weight: 700; letter-spacing: -0.02em; margin: 0 0 4px 0;",
                            "Welcome to Attune"
                        }
                        p { style: "color: var(--text-secondary); font-size: 13px; margin: 0; line-height: 1.4;",
                            "Configure your music library, native DSP audio engine, synchronized lyrics, online APIs, and typography."
                        }
                    }
                }

                // Interactive Step Navigation Bar (Steps 1..=6)
                div { style: "display: flex; gap: 8px; margin-bottom: 28px; align-items: center; flex-wrap: wrap; justify-content: center; width: 100%; padding-bottom: 16px; border-bottom: 1px solid var(--border-glass);",
                    for (step_num, label, icon_name) in SETUP_STEPS.iter() {
                        {
                            let step_style = if step_now == *step_num {
                                "padding: 7px 14px; border-radius: 12px; font-size: 12px; font-weight: 700; background: var(--accent-mint); color: #000; display: inline-flex; align-items: center; gap: 6px; border: none; cursor: pointer;"
                            } else if *step_num < step_now {
                                "padding: 7px 14px; border-radius: 12px; font-size: 12px; font-weight: 600; background: rgba(45, 212, 191, 0.12); color: var(--accent-mint); border: 1px solid rgba(45, 212, 191, 0.3); display: inline-flex; align-items: center; gap: 6px; cursor: pointer;"
                            } else {
                                "padding: 7px 14px; border-radius: 12px; font-size: 12px; font-weight: 600; background: rgba(255,255,255,0.05); color: var(--text-muted); border: 1px solid transparent; display: inline-flex; align-items: center; gap: 6px; cursor: pointer;"
                            };
                            rsx! {
                                button {
                                    key: "{step_num}",
                                    class: if step_now == *step_num { "btn-pill active" } else { "btn-pill" },
                                    style: "{step_style}",
                                    disabled: *is_indexing.read(),
                                    onclick: {
                                        let target_s = *step_num;
                                        move |_| current_step.set(target_s)
                                    },
                                    span { class: "material-icons md-16", "{icon_name}" }
                                    "{label}"
                                }
                            }
                        }
                    }
                }

                // =================================================================
                // STEP 1: MUSIC LIBRARY & DIRECTORY CONFIGURATION
                // =================================================================
                if step_now == 1 {
                    div { style: "width: 100%; display: flex; flex-direction: column; gap: 18px; align-items: stretch; text-align: left;",
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px;",
                            label { style: "font-size: 12px; font-weight: 700; text-transform: uppercase; color: var(--accent-mint); display: block; margin-bottom: 8px;",
                                "Primary Local Music Collection Directory"
                            }

                            div { style: "display: flex; gap: 10px;",
                                input {
                                    class: "search-bar",
                                    style: "flex: 1; padding: 11px 14px; font-size: 13.5px; font-family: monospace; background: rgba(255,255,255,0.05); border: 1px solid var(--border-glass); border-radius: 10px; color: #fff;",
                                    value: "{dir_display}",
                                    readonly: true,
                                }
                                button {
                                    class: "btn-pill active",
                                    style: "padding: 10px 18px; font-weight: 600; display: inline-flex; align-items: center; gap: 6px;",
                                    onclick: move |_| {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            selected_dir.set(folder.clone());
                                            update_cfg(Box::new(move |c| c.music_dir = Some(folder.clone())));
                                        }
                                    },
                                    span { class: "material-icons md-18", "folder_open" }
                                    "Browse"
                                }
                            }

                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 10px 0 0 0; line-height: 1.4;",
                                "Supports FLAC, MP3, WAV, AAC, M4A, OGG, and Opus. All audio analysis and indexing happens locally on your machine."
                            }
                        }

                        // Extra Music Directories
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px;",
                            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",
                                div {
                                    div { style: "font-size: 13.5px; font-weight: 600;", "Additional Music Folders (Optional)" }
                                    div { style: "font-size: 12px; color: var(--text-muted);", "Include secondary drives or external music folders in your library." }
                                }
                                button {
                                    class: "btn-pill",
                                    style: "font-size: 12px; display: inline-flex; align-items: center; gap: 6px;",
                                    onclick: move |_| {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            config::add_music_dir(folder);
                                            cfg_sig.set(config::load_config());
                                        }
                                    },
                                    span { class: "material-icons md-16", "create_new_folder" }
                                    "Add Folder"
                                }
                            }

                            if !curr_cfg.extra_music_dirs.is_empty() {
                                div { style: "display: flex; flex-direction: column; gap: 6px; margin-top: 10px;",
                                    for extra in curr_cfg.extra_music_dirs.iter() {
                                        div {
                                            key: "{extra.display()}",
                                            style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: rgba(0,0,0,0.25); border-radius: 8px; border: 1px solid rgba(255,255,255,0.06);",
                                            span { style: "font-family: monospace; font-size: 12.5px; color: var(--text-primary);", "{extra.display()}" }
                                            button {
                                                class: "btn-pill",
                                                style: "font-size: 11px; padding: 3px 8px; color: #fb7185;",
                                                onclick: {
                                                    let p = extra.clone();
                                                    move |_| {
                                                        config::remove_music_dir(&p);
                                                        cfg_sig.set(config::load_config());
                                                    }
                                                },
                                                "Remove"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Minimum Track Duration Filter
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 16px 18px; display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap;",
                            div {
                                div { style: "font-size: 13.5px; font-weight: 600;", "Minimum Song Duration Filter" }
                                div { style: "font-size: 12px; color: var(--text-muted);", "Skip short sound effects, voice clips, or ringtones under this duration." }
                            }
                            div { style: "display: flex; gap: 8px; align-items: center;",
                                for sec in [0u64, 30, 60, 90] {
                                    button {
                                        key: "{sec}",
                                        class: if curr_cfg.min_song_duration_sec == sec { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 12px; padding: 5px 12px;",
                                        onclick: move |_| update_cfg(Box::new(move |c| c.min_song_duration_sec = sec)),
                                        if sec == 0 { "All Files (0s)" } else { "{sec}s+" }
                                    }
                                }
                            }
                        }

                        div { style: "margin-top: 8px; display: flex; justify-content: flex-end;",
                            button {
                                class: "btn-pill active",
                                style: "padding: 12px 26px; font-size: 14px; font-weight: 700; display: inline-flex; align-items: center; gap: 8px; background: var(--accent-mint); color: #000;",
                                onclick: move |_| current_step.set(2),
                                "Next: Audio Configuration"
                                span { class: "material-icons md-18", "arrow_forward" }
                            }
                        }
                    }
                }

                // =================================================================
                // STEP 2: AUDIO ENGINE & OUTPUT CONFIGURATION
                // =================================================================
                if step_now == 2 {
                    div { style: "width: 100%; display: flex; flex-direction: column; gap: 16px; align-items: stretch; text-align: left;",
                        // Playback Codec Engine
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px;",
                            h3 { style: "font-size: 14.5px; font-weight: 700; margin: 0 0 4px 0;", "Playback Codec & DSP Engine" }
                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 0 0 12px 0;",
                                "Select between Attune's native Rust DSP hybrid engine (required for Smart DJ Shuffle transitions and 10-Band Studio EQ) or the OS default codec."
                            }
                            div { style: "display: flex; gap: 10px; flex-wrap: wrap;",
                                button {
                                    class: if curr_cfg.engine_choice == "attune_codec" { "btn-pill active" } else { "btn-pill" },
                                    onclick: move |_| update_cfg(Box::new(|c| c.engine_choice = "attune_codec".to_string())),
                                    span { class: "material-icons md-16", "bolt" }
                                    "Attune Hybrid Codec Engine (Recommended)"
                                }
                                button {
                                    class: if curr_cfg.engine_choice == "system_default" { "btn-pill active" } else { "btn-pill" },
                                    onclick: move |_| update_cfg(Box::new(|c| c.engine_choice = "system_default".to_string())),
                                    span { class: "material-icons md-16", "settings_system_daydream" }
                                    "System Default Codec"
                                }
                            }
                            if curr_cfg.engine_choice == "system_default" {
                                div { style: "margin-top: 10px; background: rgba(251, 113, 133, 0.12); border: 1px solid rgba(251, 113, 133, 0.35); padding: 8px 12px; border-radius: 8px; color: #fb7185; font-size: 12px; display: flex; align-items: center; gap: 8px;",
                                    span { class: "material-icons md-16", "warning" }
                                    "Warning: System default codec mode disables Smart Shuffle DJ transitions and Master FX DSP."
                                }
                            }
                        }

                        // Force 32-Bit Float PCM & Skip Silence
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px; display: flex; flex-direction: column; gap: 14px;",
                            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 16px;",
                                div {
                                    div { style: "display: flex; align-items: center; gap: 8px;",
                                        span { style: "font-size: 13.5px; font-weight: 600;", "[EXPERIMENTAL] Force 32-bit PCM Float Output" }
                                        span { class: "badge-pill", style: "background: rgba(234, 179, 8, 0.2); color: #eab308; border-color: rgba(234, 179, 8, 0.4); font-size: 10px;", "Experimental" }
                                    }
                                    div { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px; line-height: 1.4;",
                                        "Stream unclipped 32-bit floating point PCM directly to hardware DAC. Disable if audio sounds distorted or slow on older sound cards."
                                    }
                                }
                                button {
                                    class: if curr_cfg.force_32bit_float { "btn-pill active" } else { "btn-pill" },
                                    style: "min-width: 90px; justify-content: center;",
                                    onclick: move |_| {
                                        let new_val = !curr_cfg.force_32bit_float;
                                        let _ = crate::player::reconfigure_active_player(new_val);
                                        update_cfg(Box::new(move |c| c.force_32bit_float = new_val));
                                        driver_diag.set(crate::player::get_driver_diagnostics());
                                    },
                                    if curr_cfg.force_32bit_float { "Enabled" } else { "Disabled" }
                                }
                            }

                            div { style: "border-top: 1px solid rgba(255,255,255,0.06); padding-top: 12px; display: flex; justify-content: space-between; align-items: center; gap: 16px;",
                                div {
                                    div { style: "font-size: 13.5px; font-weight: 600;", "Skip Leading & Trailing Silence" }
                                    div { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;",
                                        "Automatically trim digital silence at the start and end of tracks during standard playback."
                                    }
                                }
                                button {
                                    class: if curr_cfg.skip_silence { "btn-pill active" } else { "btn-pill" },
                                    style: "min-width: 90px; justify-content: center;",
                                    onclick: move |_| {
                                        let new_val = !curr_cfg.skip_silence;
                                        update_cfg(Box::new(move |c| c.skip_silence = new_val));
                                    },
                                    if curr_cfg.skip_silence { "Enabled" } else { "Disabled" }
                                }
                            }

                            div { style: "border-top: 1px solid rgba(255,255,255,0.06); padding-top: 12px; display: flex; justify-content: space-between; align-items: center; gap: 16px;",
                                div {
                                    div { style: "display: flex; align-items: center; gap: 8px;",
                                        span { style: "font-size: 13.5px; font-weight: 600;", "[EXPERIMENTAL] GPU Audio Processing" }
                                        span { class: "badge-pill", style: "background: rgba(234, 179, 8, 0.2); color: #eab308; border-color: rgba(234, 179, 8, 0.4); font-size: 10px;", "Experimental" }
                                    }
                                    div { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px; line-height: 1.4;",
                                        "Offload real-time audio FFT spectral profiling and JIT DJ transition planning to GPU compute (Vulkan / Metal / Direct3D)."
                                    }
                                }
                                button {
                                    class: if curr_cfg.gpu_audio_processing { "btn-pill active" } else { "btn-pill" },
                                    style: "min-width: 90px; justify-content: center;",
                                    onclick: move |_| {
                                        let new_val = !curr_cfg.gpu_audio_processing;
                                        update_cfg(Box::new(move |c| c.gpu_audio_processing = new_val));
                                    },
                                    if curr_cfg.gpu_audio_processing { "Enabled" } else { "Disabled" }
                                }
                            }
                        }

                        // Live Hardware Output Diagnostics
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 16px 18px;",
                            div { style: "font-size: 12px; font-weight: 700; text-transform: uppercase; color: var(--text-muted); margin-bottom: 10px;",
                                "Detected Hardware Output Driver"
                            }
                            div { style: "display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px;",
                                div { style: "background: rgba(0,0,0,0.25); padding: 10px 12px; border-radius: 8px;",
                                    span { style: "font-size: 11px; color: var(--text-muted); display: block;", "Device" }
                                    span { style: "font-size: 12.5px; font-weight: 600; color: #fff;", "{diag.device_name}" }
                                }
                                div { style: "background: rgba(0,0,0,0.25); padding: 10px 12px; border-radius: 8px;",
                                    span { style: "font-size: 11px; color: var(--text-muted); display: block;", "Sample Rate" }
                                    span { style: "font-size: 12.5px; font-weight: 600; color: var(--accent-mint);", "{diag.sample_rate} Hz" }
                                }
                                div { style: "background: rgba(0,0,0,0.25); padding: 10px 12px; border-radius: 8px;",
                                    span { style: "font-size: 11px; color: var(--text-muted); display: block;", "Stream Precision" }
                                    span { style: "font-size: 12.5px; font-weight: 600; color: var(--accent-mint);", "{diag.precision}" }
                                }
                            }
                        }

                        div { style: "margin-top: 6px; display: flex; justify-content: space-between;",
                            button {
                                class: "btn-pill",
                                style: "padding: 10px 20px; display: inline-flex; align-items: center; gap: 6px;",
                                onclick: move |_| current_step.set(1),
                                span { class: "material-icons md-16", "arrow_back" }
                                "Back"
                            }
                            button {
                                class: "btn-pill active",
                                style: "padding: 12px 26px; font-size: 14px; font-weight: 700; display: inline-flex; align-items: center; gap: 8px; background: var(--accent-mint); color: #000;",
                                onclick: move |_| current_step.set(3),
                                "Next: Lyrics Configuration"
                                span { class: "material-icons md-18", "arrow_forward" }
                            }
                        }
                    }
                }

                // =================================================================
                // STEP 3: LYRICS CONFIGURATION
                // =================================================================
                if step_now == 3 {
                    div { style: "width: 100%; display: flex; flex-direction: column; gap: 16px; align-items: stretch; text-align: left;",
                        // Default Lyrics Priority
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px;",
                            h3 { style: "font-size: 14.5px; font-weight: 700; margin: 0 0 4px 0;", "Default Lyrics Priority & Fallback Chain" }
                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 0 0 10px 0;",
                                "Choose which lyric source Attune queries first when loading a song."
                            }
                            select {
                                class: "form-select",
                                style: "width: 100%; max-width: 460px;",
                                value: "{curr_cfg.lyric_priority.first().map(|s| s.as_str()).unwrap_or(\"embedded\")}",
                                onchange: move |e| {
                                    let choice = e.value();
                                    update_cfg(Box::new(move |c| {
                                        c.lyric_priority = reorder_lyric_priority(&choice);
                                    }));
                                },
                                option { value: "embedded", selected: curr_cfg.lyric_priority.first().map(|s| s == "embedded").unwrap_or(true), "Embedded Metadata (FLAC Vorbis / MP3 ID3)" }
                                option { value: "lrc", selected: curr_cfg.lyric_priority.first().map(|s| s == "lrc").unwrap_or(false), "Companion .lrc File (Local File Storage)" }
                                option { value: "ttml", selected: curr_cfg.lyric_priority.first().map(|s| s == "ttml").unwrap_or(false), "TTML Formatted Lyrics (Apple Music Syllable Sync)" }
                                option { value: "elrc", selected: curr_cfg.lyric_priority.first().map(|s| s == "elrc").unwrap_or(false), "ELRC (Enhanced Word-by-Word LRC)" }
                                option { value: "external", selected: curr_cfg.lyric_priority.first().map(|s| s == "external").unwrap_or(false), "External Online Provider (LRCLIB / BiniLyrics / Musixmatch)" }
                            }

                            div { style: "display: flex; gap: 6px; flex-wrap: wrap; margin-top: 10px; align-items: center;",
                                span { style: "font-size: 11px; color: var(--text-muted); margin-right: 4px;", "Active Chain:" }
                                for (idx, p) in curr_cfg.lyric_priority.iter().enumerate() {
                                    span {
                                        key: "{idx}",
                                        class: "badge-pill",
                                        style: if idx == 0 {
                                            "background: rgba(45,212,191,0.15); color: var(--accent-mint); border-color: rgba(45,212,191,0.35); font-size: 11px;"
                                        } else {
                                            "background: rgba(255,255,255,0.05); color: var(--text-secondary); font-size: 11px;"
                                        },
                                        "#{idx + 1} {p}"
                                    }
                                }
                            }
                        }

                        // Online Lyrics Provider
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px;",
                            h3 { style: "font-size: 14.5px; font-weight: 700; margin: 0 0 4px 0;", "Online Synchronized Lyrics Provider" }
                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 0 0 10px 0;",
                                "Select your primary upstream catalog for fetching time-synced lyrics."
                            }
                            div { style: "display: flex; gap: 10px; flex-wrap: wrap;",
                                button {
                                    class: if curr_cfg.lyric_provider == "lrclib" { "btn-pill active" } else { "btn-pill" },
                                    onclick: move |_| update_cfg(Box::new(|c| c.lyric_provider = "lrclib".to_string())),
                                    "LRCLIB (Open-Source Default)"
                                }
                                button {
                                    class: if curr_cfg.lyric_provider == "am_lyrics" { "btn-pill active" } else { "btn-pill" },
                                    onclick: move |_| {
                                        if !curr_cfg.am_lyrics_agreed {
                                            show_am_license_prompt.set(true);
                                        } else {
                                            update_cfg(Box::new(|c| c.lyric_provider = "am_lyrics".to_string()));
                                        }
                                    },
                                    "BiniLyrics by Binimum (<am-lyrics>)"
                                }
                                if curr_cfg.api_musixmatch_enabled {
                                    button {
                                        class: if curr_cfg.lyric_provider == "musixmatch" { "btn-pill active" } else { "btn-pill" },
                                        onclick: move |_| update_cfg(Box::new(|c| c.lyric_provider = "musixmatch".to_string())),
                                        "Musixmatch [Exp]"
                                    }
                                }
                            }

                            if *show_am_license_prompt.read() && !curr_cfg.am_lyrics_agreed {
                                div { style: "margin-top: 12px; padding: 12px 14px; border-radius: 10px; background: rgba(45, 212, 191, 0.08); border: 1px solid rgba(45, 212, 191, 0.3);",
                                    div { style: "font-size: 12.5px; font-weight: 700; color: var(--accent-mint); margin-bottom: 4px;",
                                        "BiniLyrics (<am-lyrics>) MPL-2.0 Attribution Agreement"
                                    }
                                    p { style: "font-size: 12px; color: var(--text-secondary); margin: 0 0 10px 0; line-height: 1.4;",
                                        "BiniLyrics is powered by Binimum's open-source am-lyrics service under the Mozilla Public License 2.0. Accept to enable syllable-synced TTML/ELRC fetching."
                                    }
                                    div { style: "display: flex; gap: 8px;",
                                        button {
                                            class: "btn-pill active",
                                            style: "font-size: 12px; padding: 5px 14px;",
                                            onclick: move |_| {
                                                update_cfg(Box::new(|c| {
                                                    c.am_lyrics_agreed = true;
                                                    c.lyric_provider = "am_lyrics".to_string();
                                                }));
                                                show_am_license_prompt.set(false);
                                            },
                                            "Accept & Enable BiniLyrics"
                                        }
                                        button {
                                            class: "btn-pill",
                                            style: "font-size: 12px; padding: 5px 14px;",
                                            onclick: move |_| show_am_license_prompt.set(false),
                                            "Cancel"
                                        }
                                    }
                                }
                            }
                        }

                        // Lyrics Alignment, Glow & Fullscreen Animation Presets
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px; display: flex; flex-direction: column; gap: 12px;",
                            div { style: "display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 10px;",
                                div {
                                    div { style: "font-size: 13.5px; font-weight: 600;", "Lyrics Alignment & Active Text Glow" }
                                    div { style: "font-size: 12px; color: var(--text-muted);", "Configure text alignment and cream-white luminescence on active singing lines." }
                                }
                                div { style: "display: flex; gap: 6px; align-items: center;",
                                    for align in ["left", "center", "right"] {
                                        button {
                                            key: "{align}",
                                            class: if curr_cfg.fullscreen_lyrics_align == align { "btn-pill active" } else { "btn-pill" },
                                            style: "font-size: 12px; padding: 5px 12px; text-transform: capitalize;",
                                            onclick: move |_| update_cfg(Box::new(move |c| {
                                                c.fullscreen_lyrics_align = align.to_string();
                                                c.dock_lyrics_align = align.to_string();
                                            })),
                                            "{align}"
                                        }
                                    }
                                    button {
                                        class: if curr_cfg.fullscreen_text_glow { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 12px; padding: 5px 12px;",
                                        onclick: move |_| {
                                            let glow = !curr_cfg.fullscreen_text_glow;
                                            update_cfg(Box::new(move |c| {
                                                c.fullscreen_text_glow = glow;
                                                c.sidebar_text_glow = glow;
                                            }));
                                        },
                                        if curr_cfg.fullscreen_text_glow { "Glow: ON" } else { "Glow: OFF" }
                                    }
                                }
                            }

                            div { style: "border-top: 1px solid rgba(255,255,255,0.06); padding-top: 12px;",
                                div { style: "font-size: 13px; font-weight: 600; margin-bottom: 8px;", "Fullscreen Lyrics Animation Style" }
                                div { style: "display: flex; gap: 8px; flex-wrap: wrap;",
                                    button {
                                        class: if !curr_cfg.enable_lyrics_custom_css { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 12px; padding: 5px 12px;",
                                        onclick: move |_| update_cfg(Box::new(|c| c.enable_lyrics_custom_css = false)),
                                        "Standard Smooth"
                                    }
                                    button {
                                        class: if curr_cfg.enable_lyrics_custom_css && curr_cfg.lyrics_custom_css.starts_with("/* Spotlight") { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 12px; padding: 5px 12px;",
                                        onclick: move |_| update_cfg(Box::new(|c| {
                                            c.enable_lyrics_custom_css = true;
                                            c.lyrics_custom_css = config::PRESET_SPOTLIGHT_BLUR.to_string();
                                        })),
                                        "Spotlight Depth Blur"
                                    }
                                    button {
                                        class: if curr_cfg.enable_lyrics_custom_css && curr_cfg.lyrics_custom_css.starts_with("/* Apple Music") { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 12px; padding: 5px 12px;",
                                        onclick: move |_| update_cfg(Box::new(|c| {
                                            c.enable_lyrics_custom_css = true;
                                            c.lyrics_custom_css = config::PRESET_SPRING_POP.to_string();
                                        })),
                                        "Spring Pop"
                                    }
                                    button {
                                        class: if curr_cfg.enable_lyrics_custom_css && curr_cfg.lyrics_custom_css.starts_with("/* Cyberpunk") { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 12px; padding: 5px 12px;",
                                        onclick: move |_| update_cfg(Box::new(|c| {
                                            c.enable_lyrics_custom_css = true;
                                            c.lyrics_custom_css = config::PRESET_CYBERPUNK_NEON.to_string();
                                        })),
                                        "Cyberpunk Neon"
                                    }
                                    button {
                                        class: if curr_cfg.enable_lyrics_custom_css && curr_cfg.lyrics_custom_css.starts_with("/* Karaoke") { "btn-pill active" } else { "btn-pill" },
                                        style: "font-size: 12px; padding: 5px 12px;",
                                        onclick: move |_| update_cfg(Box::new(|c| {
                                            c.enable_lyrics_custom_css = true;
                                            c.lyrics_custom_css = config::PRESET_KARAOKE_BOUNCE.to_string();
                                        })),
                                        "Karaoke Golden Bounce"
                                    }
                                }
                            }
                        }

                        div { style: "margin-top: 6px; display: flex; justify-content: space-between;",
                            button {
                                class: "btn-pill",
                                style: "padding: 10px 20px; display: inline-flex; align-items: center; gap: 6px;",
                                onclick: move |_| current_step.set(2),
                                span { class: "material-icons md-16", "arrow_back" }
                                "Back"
                            }
                            button {
                                class: "btn-pill active",
                                style: "padding: 12px 26px; font-size: 14px; font-weight: 700; display: inline-flex; align-items: center; gap: 8px; background: var(--accent-mint); color: #000;",
                                onclick: move |_| current_step.set(4),
                                "Next: API Configuration"
                                span { class: "material-icons md-18", "arrow_forward" }
                            }
                        }
                    }
                }

                // =================================================================
                // STEP 4: API PROVIDERS CONFIGURATION
                // =================================================================
                if step_now == 4 {
                    div { style: "width: 100%; display: flex; flex-direction: column; gap: 16px; align-items: stretch; text-align: left;",
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px; display: flex; flex-direction: column; gap: 14px;",
                            // YouTube Music Innertube
                            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 16px; border-bottom: 1px solid rgba(255,255,255,0.06); padding-bottom: 12px;",
                                div {
                                    div { style: "font-size: 13.5px; font-weight: 600;", "YouTube Music Innertube API" }
                                    div { style: "font-size: 12px; color: var(--text-muted);", "Fetch verified artist portraits, 16:9 widescreen banners, subscriber counts, and bios (no API key required)." }
                                }
                                button {
                                    class: if curr_cfg.api_youtube_enabled { "btn-pill active" } else { "btn-pill" },
                                    style: "min-width: 90px; justify-content: center;",
                                    onclick: move |_| {
                                        let v = !curr_cfg.api_youtube_enabled;
                                        update_cfg(Box::new(move |c| c.api_youtube_enabled = v));
                                    },
                                    if curr_cfg.api_youtube_enabled { "Enabled" } else { "Disabled" }
                                }
                            }

                            // YouTube Music HD Artwork
                            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 16px; border-bottom: 1px solid rgba(255,255,255,0.06); padding-bottom: 12px;",
                                div {
                                    div { style: "font-size: 13.5px; font-weight: 600;", "YouTube Music High-Resolution Artwork" }
                                    div { style: "font-size: 12px; color: var(--text-muted);", "Download crisp 1024x1024 square album covers and HD thumbnails for tracks missing cover art." }
                                }
                                button {
                                    class: if curr_cfg.fetch_youtube_thumbnails { "btn-pill active" } else { "btn-pill" },
                                    style: "min-width: 90px; justify-content: center;",
                                    onclick: move |_| {
                                        let v = !curr_cfg.fetch_youtube_thumbnails;
                                        update_cfg(Box::new(move |c| c.fetch_youtube_thumbnails = v));
                                    },
                                    if curr_cfg.fetch_youtube_thumbnails { "Enabled" } else { "Disabled" }
                                }
                            }

                            // Wikipedia & MusicBrainz
                            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 16px; border-bottom: 1px solid rgba(255,255,255,0.06); padding-bottom: 12px;",
                                div {
                                    div { style: "font-size: 13.5px; font-weight: 600;", "Wikipedia & MusicBrainz REST API" }
                                    div { style: "font-size: 12px; color: var(--text-muted);", "Open community discography, release years, and encyclopedia artist lore." }
                                }
                                button {
                                    class: if curr_cfg.api_wikipedia_enabled { "btn-pill active" } else { "btn-pill" },
                                    style: "min-width: 90px; justify-content: center;",
                                    onclick: move |_| {
                                        let v = !curr_cfg.api_wikipedia_enabled;
                                        update_cfg(Box::new(move |c| {
                                            c.api_wikipedia_enabled = v;
                                            c.api_musicbrainz_enabled = v;
                                        }));
                                    },
                                    if curr_cfg.api_wikipedia_enabled { "Enabled" } else { "Disabled" }
                                }
                            }

                            // Experimental Musixmatch API
                            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 16px;",
                                div {
                                    div { style: "display: flex; align-items: center; gap: 8px;",
                                        span { style: "font-size: 13.5px; font-weight: 600;", "[EXPERIMENTAL] Musixmatch API" }
                                        span { class: "badge-pill", style: "background: rgba(234, 179, 8, 0.2); color: #eab308; border-color: rgba(234, 179, 8, 0.4); font-size: 10px;", "Experimental" }
                                    }
                                    div { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;", "Commercial synchronized lyrics catalog (requires a personal Musixmatch API key)." }
                                }
                                button {
                                    class: if curr_cfg.api_musixmatch_enabled { "btn-pill active" } else { "btn-pill" },
                                    style: "min-width: 90px; justify-content: center;",
                                    onclick: move |_| {
                                        let v = !curr_cfg.api_musixmatch_enabled;
                                        update_cfg(Box::new(move |c| c.api_musixmatch_enabled = v));
                                    },
                                    if curr_cfg.api_musixmatch_enabled { "Enabled" } else { "Disabled" }
                                }
                            }

                            if curr_cfg.api_musixmatch_enabled {
                                div { style: "padding: 12px 14px; background: rgba(0,0,0,0.25); border-radius: 10px; border: 1px solid rgba(255,255,255,0.08);",
                                    label { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; display: block; margin-bottom: 6px;",
                                        "Musixmatch User API Key"
                                    }
                                    div { style: "display: flex; gap: 10px; align-items: center; flex-wrap: wrap;",
                                        input {
                                            r#type: "password",
                                            class: "search-bar",
                                            style: "flex: 1; min-width: 220px; padding: 8px 12px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff; font-size: 13px;",
                                            placeholder: "Enter Musixmatch API Key...",
                                            value: "{curr_cfg.musixmatch_api_key}",
                                            oninput: move |e| {
                                                let key = e.value();
                                                update_cfg(Box::new(move |c| c.musixmatch_api_key = key.clone()));
                                            }
                                        }
                                        button {
                                            class: "btn-pill",
                                            disabled: *mxm_test_loading.read(),
                                            onclick: {
                                                let key = curr_cfg.musixmatch_api_key.clone();
                                                move |_| {
                                                    let key_to_test = key.clone();
                                                    mxm_test_loading.set(true);
                                                    mxm_test_status.set(None);
                                                    dioxus::prelude::spawn(async move {
                                                        let res = tokio::task::spawn_blocking(move || {
                                                            crate::stages::lyrics_finder::test_musixmatch_api(&key_to_test)
                                                        })
                                                        .await
                                                        .unwrap_or_else(|e| Err(format!("Task execution error: {e}")));
                                                        mxm_test_loading.set(false);
                                                        match res {
                                                            Ok(msg) => mxm_test_status.set(Some((true, msg))),
                                                            Err(err) => mxm_test_status.set(Some((false, err))),
                                                        }
                                                    });
                                                }
                                            },
                                            if *mxm_test_loading.read() {
                                                span { class: "material-icons md-14 spinning", "sync" }
                                                span { "Testing..." }
                                            } else {
                                                span { class: "material-icons md-14", "speed" }
                                                span { "Test Query" }
                                            }
                                        }
                                    }
                                    if let Some((is_ok, ref msg)) = *mxm_test_status.read() {
                                        div {
                                            style: if is_ok {
                                                "margin-top: 8px; padding: 7px 10px; border-radius: 8px; font-size: 12px; background: rgba(45,212,191,0.12); color: var(--accent-mint); border: 1px solid rgba(45,212,191,0.3);"
                                            } else {
                                                "margin-top: 8px; padding: 7px 10px; border-radius: 8px; font-size: 12px; background: rgba(244,63,94,0.12); color: #fb7185; border: 1px solid rgba(244,63,94,0.3);"
                                            },
                                            "{msg}"
                                        }
                                    }
                                }
                            }
                        }

                        div { style: "margin-top: 6px; display: flex; justify-content: space-between;",
                            button {
                                class: "btn-pill",
                                style: "padding: 10px 20px; display: inline-flex; align-items: center; gap: 6px;",
                                onclick: move |_| current_step.set(3),
                                span { class: "material-icons md-16", "arrow_back" }
                                "Back"
                            }
                            button {
                                class: "btn-pill active",
                                style: "padding: 12px 26px; font-size: 14px; font-weight: 700; display: inline-flex; align-items: center; gap: 8px; background: var(--accent-mint); color: #000;",
                                onclick: move |_| current_step.set(5),
                                "Next: Font Configuration"
                                span { class: "material-icons md-18", "arrow_forward" }
                            }
                        }
                    }
                }

                // =================================================================
                // STEP 5: FONT & TYPOGRAPHY CONFIGURATION
                // =================================================================
                if step_now == 5 {
                    div { style: "width: 100%; display: flex; flex-direction: column; gap: 16px; align-items: stretch; text-align: left;",
                        // Application UI Font Family
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px;",
                            h3 { style: "font-size: 14.5px; font-weight: 700; margin: 0 0 4px 0;", "Application UI Font Family" }
                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 0 0 10px 0;",
                                "Select the primary typeface used across navigation, library tables, and headers."
                            }
                            select {
                                class: "form-select",
                                style: "width: 100%; max-width: 480px;",
                                value: "{current_app_font}",
                                onchange: move |e| {
                                    let val = e.value();
                                    update_cfg(Box::new(move |c| c.app_font_family = val.clone()));
                                },
                                optgroup {
                                    label: "{default_font_group}",
                                    for opt in crate::system_fonts::ATTUNE_DEFAULT_FONTS {
                                        option {
                                            key: "{opt.id}",
                                            value: "{opt.id}",
                                            selected: current_app_font == opt.id,
                                            "{crate::system_fonts::pango_escape(opt.label)}"
                                        }
                                    }
                                }
                                optgroup {
                                    label: "{system_font_group}",
                                    for sys_font in system_fonts.iter() {
                                        option {
                                            key: "{sys_font}",
                                            value: "{sys_font}",
                                            selected: current_app_font == sys_font.as_str(),
                                            "{crate::system_fonts::pango_escape(sys_font)}"
                                        }
                                    }
                                }
                            }

                            div { style: "margin-top: 10px; padding: 10px 14px; border-radius: 10px; background: rgba(0,0,0,0.28); border: 1px solid var(--border-glass);",
                                div { style: "font-family: {preview_app_font_css}; font-size: 15px; font-weight: 700; color: #fff;",
                                    "ATTUNE • Intelligent Music Player & DJ Transition Engine"
                                }
                                div { style: "font-family: {preview_app_font_css}; font-size: 12.5px; color: var(--text-secondary); margin-top: 2px;",
                                    "Attune • Ready (Attune Dynamic DJ Sessions Vol. 1) • 128.0 BPM • Camelot 4A"
                                }
                            }
                        }

                        // Lyrics Font Family & Sizes
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 18px;",
                            h3 { style: "font-size: 14.5px; font-weight: 700; margin: 0 0 4px 0;", "Synchronized Lyrics Font Family & Size" }
                            p { style: "font-size: 12px; color: var(--text-secondary); margin: 0 0 10px 0;",
                                "Choose the typeface and text scaling for docked sidebar and fullscreen singing lines."
                            }
                            div { style: "display: flex; gap: 14px; align-items: center; flex-wrap: wrap;",
                                select {
                                    class: "form-select",
                                    style: "flex: 1; min-width: 240px; max-width: 420px;",
                                    value: "{current_lyric_font}",
                                    onchange: move |e| {
                                        let val = e.value();
                                        update_cfg(Box::new(move |c| c.lyric_font_family = val.clone()));
                                    },
                                    optgroup {
                                        label: "{default_font_group}",
                                        for opt in crate::system_fonts::ATTUNE_DEFAULT_FONTS {
                                            option {
                                                key: "{opt.id}",
                                                value: "{opt.id}",
                                                selected: current_lyric_font == opt.id,
                                                "{crate::system_fonts::pango_escape(opt.label)}"
                                            }
                                        }
                                    }
                                    optgroup {
                                        label: "{system_font_group}",
                                        for sys_font in system_fonts.iter() {
                                            option {
                                                key: "{sys_font}",
                                                value: "{sys_font}",
                                                selected: current_lyric_font == sys_font.as_str(),
                                                "{crate::system_fonts::pango_escape(sys_font)}"
                                            }
                                        }
                                    }
                                }

                                div { style: "display: flex; align-items: center; gap: 8px;",
                                    span { style: "font-size: 12px; color: var(--text-muted);", "Fullscreen Size:" }
                                    input {
                                        r#type: "range",
                                        min: "18",
                                        max: "48",
                                        step: "1",
                                        value: "{curr_cfg.fullscreen_lyrics_font_size}",
                                        style: "width: 90px; accent-color: var(--accent-mint);",
                                        oninput: move |e| {
                                            if let Ok(sz) = e.value().parse::<u32>() {
                                                let clamped = sz.clamp(18, 48);
                                                update_cfg(Box::new(move |c| c.fullscreen_lyrics_font_size = clamped));
                                            }
                                        }
                                    }
                                    span { style: "font-family: monospace; font-size: 12px; color: var(--accent-mint); min-width: 34px;", "{curr_cfg.fullscreen_lyrics_font_size}px" }
                                }
                            }

                            div { style: "margin-top: 10px; padding: 10px 14px; border-radius: 10px; background: rgba(0,0,0,0.28); border: 1px solid var(--border-glass);",
                                div { style: "font-family: {preview_lyric_font_css}; font-size: 18px; font-weight: 700; color: #FFFDD0;",
                                    "デジタルの風に 心をチューニングして • Attune your soul to the digital breeze"
                                }
                            }
                        }

                        // Marquee & High Quality Thumbnails
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 14px 18px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px;",
                            div { style: "display: flex; gap: 18px; align-items: center; flex-wrap: wrap;",
                                button {
                                    class: if curr_cfg.auto_scroll_long_text { "btn-pill active" } else { "btn-pill" },
                                    style: "font-size: 12px;",
                                    onclick: move |_| {
                                        let v = !curr_cfg.auto_scroll_long_text;
                                        update_cfg(Box::new(move |c| c.auto_scroll_long_text = v));
                                    },
                                    if curr_cfg.auto_scroll_long_text { "✓ Long Text Marquee: ON" } else { "Long Text Marquee: OFF" }
                                }
                                button {
                                    class: if curr_cfg.high_quality_thumbnails { "btn-pill active" } else { "btn-pill" },
                                    style: "font-size: 12px;",
                                    onclick: move |_| {
                                        let v = !curr_cfg.high_quality_thumbnails;
                                        update_cfg(Box::new(move |c| c.high_quality_thumbnails = v));
                                    },
                                    if curr_cfg.high_quality_thumbnails { "✓ 1024px HD Artwork: ON" } else { "1024px HD Artwork: OFF" }
                                }
                            }
                        }

                        div { style: "margin-top: 6px; display: flex; justify-content: space-between;",
                            button {
                                class: "btn-pill",
                                style: "padding: 10px 20px; display: inline-flex; align-items: center; gap: 6px;",
                                onclick: move |_| current_step.set(4),
                                span { class: "material-icons md-16", "arrow_back" }
                                "Back"
                            }
                            button {
                                class: "btn-pill active",
                                style: "padding: 12px 26px; font-size: 14px; font-weight: 700; display: inline-flex; align-items: center; gap: 8px; background: var(--accent-mint); color: #000;",
                                onclick: move |_| current_step.set(6),
                                "Next: Audio Profiling & Launch"
                                span { class: "material-icons md-18", "arrow_forward" }
                            }
                        }
                    }
                }

                // =================================================================
                // STEP 6: 5-LAYER AUDIO PROFILING & LAUNCH
                // =================================================================
                if step_now == 6 {
                    div { style: "width: 100%; display: flex; flex-direction: column; gap: 18px; align-items: stretch; text-align: left;",
                        if !*indexing_done.read() {
                            div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 20px;",
                                h3 { style: "font-size: 15px; font-weight: 700; margin: 0 0 6px 0;", "Five-Layer Audio Analysis & Library Indexing" }
                                p { style: "font-size: 12.5px; color: var(--text-secondary); margin: 0; line-height: 1.5;",
                                    "Attune extracts CQT Camelot musical keys, nominal BPM, phrase grid alignments, and 5-band energy dynamics from your tracks in "
                                    code { style: "color: var(--accent-mint); font-family: monospace;", "{dir_display}" }
                                    " to power pitch-perfect Smart DJ Shuffle transitions."
                                }
                            }

                            if !*is_indexing.read() && *progress_pct.read() == 0.0 {
                                div { style: "display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 16px 0;",
                                    button {
                                        class: "btn-pill active",
                                        style: "padding: 15px 34px; font-size: 15px; font-weight: 700; display: inline-flex; align-items: center; gap: 10px; background: var(--accent-mint); color: #000; box-shadow: 0 0 24px rgba(45,212,191,0.3);",
                                        onclick: {
                                            let root = selected_dir.read().clone();
                                            move |_| {
                                                is_indexing.set(true);
                                                status_note.set("Scanning directory...".to_string());
                                                let rx = library::spawn_deep_indexing(root.clone(), false);
                                                *index_rx.write() = Arc::new(Mutex::new(Some(rx)));
                                            }
                                        },
                                        span { class: "material-icons md-20", "auto_awesome" }
                                        "Start Library Indexing & Audio Profiling"
                                    }

                                    button {
                                        class: "btn-pill",
                                        style: "font-size: 12px; padding: 8px 18px;",
                                        onclick: {
                                            let dir = selected_dir.read().clone();
                                            move |_| {
                                                let quick_tracks = library::load_library_cache(&dir).unwrap_or_default();
                                                config::set_setup_completed(true, Some(dir.clone()));
                                                on_complete.call(quick_tracks);
                                            }
                                        },
                                        "Skip Deep Profiling & Launch Immediately (On-Demand JIT)"
                                    }
                                }
                            } else {
                                // Active Indexing Progress
                                div { style: "display: flex; flex-direction: column; gap: 12px;",
                                    div { style: "display: flex; justify-content: space-between; align-items: center; font-size: 13px;",
                                        span { style: "font-weight: 600; color: var(--text-primary);", "{status_note.read()}" }
                                        span { style: "font-family: monospace; font-weight: 700; color: var(--accent-mint); font-size: 15px;",
                                            "{progress_pct.read():.1}%"
                                        }
                                    }

                                    div { style: "width: 100%; height: 10px; background: rgba(255,255,255,0.08); border-radius: 5px; overflow: hidden; border: 1px solid var(--border-glass);",
                                        div {
                                            style: "height: 100%; width: {progress_pct.read()}%; background: linear-gradient(90deg, var(--accent-mint), #a855f7); border-radius: 5px; transition: width 0.15s ease-out; box-shadow: 0 0 12px rgba(45,212,191,0.5);"
                                        }
                                    }

                                    if let Some((title, artist, bpm, key, format)) = current_track_info.read().as_ref() {
                                        div { style: "background: rgba(45, 212, 191, 0.06); border: 1px solid rgba(45, 212, 191, 0.2); border-radius: 12px; padding: 12px 16px; margin-top: 6px; display: flex; justify-content: space-between; align-items: center;",
                                            div { style: "overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 65%;",
                                                div { style: "font-size: 13.5px; font-weight: 600;", "{title}" }
                                                div { style: "font-size: 12px; color: var(--text-muted);", "{artist}" }
                                            }
                                            div { style: "display: flex; gap: 6px; align-items: center;",
                                                if let Some(k) = key {
                                                    span { style: "padding: 3px 8px; border-radius: 6px; background: rgba(255,255,255,0.08); font-size: 11px; font-weight: 700; color: var(--accent-mint); font-family: monospace;",
                                                        "Key: {k}"
                                                    }
                                                }
                                                if let Some(b) = bpm {
                                                    span { style: "padding: 3px 8px; border-radius: 6px; background: rgba(255,255,255,0.08); font-size: 11px; font-weight: 700; color: var(--accent-mint); font-family: monospace;",
                                                        "{b:.1} BPM"
                                                    }
                                                }
                                                span { style: "padding: 3px 8px; border-radius: 6px; background: rgba(255,255,255,0.04); font-size: 11px; color: var(--text-muted); font-family: monospace;",
                                                    "{format}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !*is_indexing.read() {
                                div { style: "display: flex; justify-content: flex-start;",
                                    button {
                                        class: "btn-pill",
                                        style: "padding: 10px 20px; display: inline-flex; align-items: center; gap: 6px;",
                                        onclick: move |_| current_step.set(5),
                                        span { class: "material-icons md-16", "arrow_back" }
                                        "Back"
                                    }
                                }
                            }
                        } else {
                            // Summary & Launch
                            div { style: "width: 100%; display: flex; flex-direction: column; gap: 18px; align-items: center; text-align: center;",
                                div { style: "width: 56px; height: 56px; border-radius: 50%; background: rgba(45, 212, 191, 0.2); border: 2px solid var(--accent-mint); display: flex; align-items: center; justify-content: center;",
                                    span { class: "material-icons md-32", style: "color: var(--accent-mint);", "check" }
                                }

                                h2 { style: "font-size: 22px; font-weight: 700; margin: 0;", "Attune Is Configured & Ready!" }
                                p { style: "color: var(--text-secondary); font-size: 13.5px; max-width: 520px; margin: 0; line-height: 1.5;",
                                    "Your music library, audio engine, lyrics providers, APIs, and typography preferences are saved."
                                }

                                div { style: "display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; width: 100%; margin: 6px 0;",
                                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 12px; padding: 14px;",
                                        span { style: "font-size: 10.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; display: block; margin-bottom: 4px;", "Indexed Tracks" }
                                        span { style: "font-size: 20px; font-weight: 700; font-family: monospace; color: var(--accent-mint);",
                                            "{indexed_tracks.read().len()}"
                                        }
                                    }
                                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 12px; padding: 14px;",
                                        span { style: "font-size: 10.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; display: block; margin-bottom: 4px;", "Lossless Audio" }
                                        span { style: "font-size: 20px; font-weight: 700; font-family: monospace; color: var(--text-primary);",
                                            "{lossless_count.read()}"
                                        }
                                    }
                                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 12px; padding: 14px;",
                                        span { style: "font-size: 10.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; display: block; margin-bottom: 4px;", "Lyrics Provider" }
                                        span { style: "font-size: 14px; font-weight: 700; color: var(--accent-mint); text-transform: uppercase;",
                                            "{curr_cfg.lyric_provider}"
                                        }
                                    }
                                }

                                button {
                                    class: "btn-pill active",
                                    style: "padding: 16px 42px; font-size: 16px; font-weight: 700; display: inline-flex; align-items: center; gap: 10px; background: var(--accent-mint); color: #000; box-shadow: 0 0 28px rgba(45,212,191,0.35); margin-top: 6px;",
                                    onclick: {
                                        let tracks = indexed_tracks.read().clone();
                                        let dir = selected_dir.read().clone();
                                        move |_| {
                                            config::set_setup_completed(true, Some(dir.clone()));
                                            on_complete.call(tracks.clone());
                                        }
                                    },
                                    "Launch Attune"
                                    span { class: "material-icons md-20", "play_arrow" }
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
    fn test_setup_steps_include_all_configuration_domains() {
        assert_eq!(SETUP_STEPS.len(), 6);
        let labels: Vec<&str> = SETUP_STEPS.iter().map(|(_, l, _)| *l).collect();
        assert!(labels.iter().any(|l| l.contains("Library")));
        assert!(labels.iter().any(|l| l.contains("Audio")));
        assert!(labels.iter().any(|l| l.contains("Lyrics")));
        assert!(labels.iter().any(|l| l.contains("APIs")));
        assert!(labels.iter().any(|l| l.contains("Fonts")));
        assert!(labels.iter().any(|l| l.contains("Profile")));
    }

    #[test]
    fn test_reorder_lyric_priority_places_choice_first() {
        let chain = reorder_lyric_priority("ttml");
        assert_eq!(chain.len(), 5);
        assert_eq!(chain[0], "ttml");
        assert!(chain.contains(&"embedded".to_string()));
        assert!(chain.contains(&"lrc".to_string()));
        assert!(chain.contains(&"elrc".to_string()));
        assert!(chain.contains(&"external".to_string()));
    }

    #[test]
    fn test_setup_hero_logo_url_validity() {
        let logo_url = crate::app_icon::app_icon_data_url();
        assert!(logo_url.starts_with("data:image/png;base64,"));
        assert!(logo_url.len() > 100);
    }
}
