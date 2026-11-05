//! Setup Stage: Step-by-step first-run setup & deep audio profiling wizard for Attune Desktop.
//!
//! Provides:
//! - Step 1: Music Library directory selection & folder picker.
//! - Step 2: Full 5-layer audio profiling (Camelot harmonic keys via CQT, nominal BPM,
//!          5-band energy, and aggression index) with live animated progress.
//! - Step 3: Engine readiness summary and launch into Home.

use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::library::{self, IndexingProgressEvent, TrackEntry};

#[component]
pub fn SetupStage(
    initial_dir: Option<PathBuf>,
    on_complete: EventHandler<Vec<TrackEntry>>,
) -> Element {
    let mut selected_dir = use_signal(|| {
        initial_dir.unwrap_or_else(|| {
            let (dir, _) = crate::config::resolve_music_dir(None);
            dir.unwrap_or_else(|| dirs::audio_dir().unwrap_or_else(|| PathBuf::from("/home/miku/Music")))
        })
    });

    let mut current_step = use_signal(|| 1usize); // 1 = Directory, 2 = Profiling, 3 = Complete
    let mut is_indexing = use_signal(|| false);
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
                            status_note.set(format!("Discovered {total_files} audio files. Starting 5-layer DSP analysis..."));
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
                        IndexingProgressEvent::Finished { tracks, lossless_count: lc } => {
                            lossless_count.set(lc);
                            let count = tracks.len();
                            indexed_tracks.set(tracks);
                            is_indexing.set(false);
                            progress_pct.set(100.0);
                            current_step.set(3);
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

    let dir_val = selected_dir.read().clone();
    let dir_display = dir_val.display().to_string();

    rsx! {
        div { class: "setup-stage",
            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 80vh; max-width: 780px; margin: 0 auto; width: 100%; padding: 40px 24px;",

            // Main Liquid Material Glass Setup Card
            div { style: "background: var(--bg-card); border: 1px solid var(--border-glass); border-radius: 28px; padding: 40px; width: 100%; box-shadow: 0 24px 48px rgba(0,0,0,0.5); backdrop-filter: blur(40px); display: flex; flex-direction: column; align-items: center; text-align: center;",

                // Hero Logo Disc
                div { style: "width: 72px; height: 72px; border-radius: 50%; background: linear-gradient(135deg, rgba(45, 212, 191, 0.25), rgba(168, 85, 247, 0.25)); border: 2px solid var(--accent-mint); display: flex; align-items: center; justify-content: center; margin-bottom: 20px; box-shadow: 0 0 28px rgba(45, 212, 191, 0.3);",
                    span { class: "material-icons md-36", style: "color: var(--accent-mint);", "album" }
                }

                h1 { style: "font-size: 28px; font-weight: 700; letter-spacing: -0.02em; margin-bottom: 8px;",
                    "Welcome to Attune"
                }
                p { style: "color: var(--text-secondary); font-size: 14px; max-width: 540px; margin-bottom: 28px; line-height: 1.5;",
                    "Intelligent DJ-grade music player with audio-profile-driven transition & remixing engine. Set up your local music library to begin."
                }

                // Step Indicator Pills
                div { style: "display: flex; gap: 10px; margin-bottom: 32px; align-items: center;",
                    div {
                        style: if *current_step.read() == 1 {
                            "padding: 6px 14px; border-radius: 12px; font-size: 12px; font-weight: 700; background: var(--accent-mint); color: #000;"
                        } else {
                            "padding: 6px 14px; border-radius: 12px; font-size: 12px; font-weight: 600; background: rgba(255,255,255,0.06); color: var(--text-muted);"
                        },
                        "1. Music Folder"
                    }
                    span { style: "color: var(--border-glass);", "—" }
                    div {
                        style: if *current_step.read() == 2 {
                            "padding: 6px 14px; border-radius: 12px; font-size: 12px; font-weight: 700; background: var(--accent-mint); color: #000;"
                        } else {
                            "padding: 6px 14px; border-radius: 12px; font-size: 12px; font-weight: 600; background: rgba(255,255,255,0.06); color: var(--text-muted);"
                        },
                        "2. Audio Profiling"
                    }
                    span { style: "color: var(--border-glass);", "—" }
                    div {
                        style: if *current_step.read() == 3 {
                            "padding: 6px 14px; border-radius: 12px; font-size: 12px; font-weight: 700; background: var(--accent-mint); color: #000;"
                        } else {
                            "padding: 6px 14px; border-radius: 12px; font-size: 12px; font-weight: 600; background: rgba(255,255,255,0.06); color: var(--text-muted);"
                        },
                        "3. Ready"
                    }
                }

                // ==========================================
                // STEP 1: DIRECTORY SELECTION
                // ==========================================
                if *current_step.read() == 1 {
                    div { style: "width: 100%; display: flex; flex-direction: column; gap: 16px; align-items: stretch; text-align: left;",
                        label { style: "font-size: 12px; font-weight: 700; text-transform: uppercase; color: var(--text-muted);",
                            "Local Music Collection Directory"
                        }

                        div { style: "display: flex; gap: 10px;",
                            input {
                                class: "search-bar",
                                style: "flex: 1; padding: 12px 16px; font-size: 14px; font-family: monospace; background: rgba(255,255,255,0.04); border: 1px solid var(--border-glass); border-radius: 12px; color: #fff;",
                                value: "{dir_display}",
                                readonly: true,
                            }
                            button {
                                class: "btn-pill active",
                                style: "padding: 12px 20px; font-weight: 600; display: inline-flex; align-items: center; gap: 8px;",
                                onclick: move |_| {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        selected_dir.set(folder);
                                    }
                                },
                                span { class: "material-icons md-18", "folder_open" }
                                "Browse"
                            }
                        }

                        p { style: "font-size: 12.5px; color: var(--text-secondary); line-height: 1.4;",
                            "Attune scans standard audio formats (FLAC, MP3, WAV, AAC, OGG, ALAC). Songs are indexed locally with zero external uploads."
                        }

                        div { style: "margin-top: 24px; display: flex; justify-content: flex-end;",
                            button {
                                class: "btn-pill active",
                                style: "padding: 14px 28px; font-size: 14.5px; font-weight: 700; display: inline-flex; align-items: center; gap: 10px; background: var(--accent-mint); color: #000;",
                                onclick: move |_| current_step.set(2),
                                "Continue to Audio Profiling"
                                span { class: "material-icons md-18", "arrow_forward" }
                            }
                        }
                    }
                }

                // ==========================================
                // STEP 2: AUDIO PROFILING & INDEXING
                // ==========================================
                if *current_step.read() == 2 {
                    div { style: "width: 100%; display: flex; flex-direction: column; gap: 20px; align-items: stretch; text-align: left;",
                        div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 16px; padding: 20px;",
                            h3 { style: "font-size: 15px; font-weight: 600; margin-bottom: 6px;", "Five-Layer Audio Analysis" }
                            p { style: "font-size: 12.5px; color: var(--text-secondary); line-height: 1.5;",
                                "Attune extracts genuine CQT Camelot musical keys, nominal BPM, phrase grid alignments, and 5-band energy dynamics directly from track waveforms. This guarantees pitch-perfect Smart DJ Shuffle and Harmonic Flow mixes."
                            }
                        }

                        if !*is_indexing.read() && *progress_pct.read() == 0.0 {
                            div { style: "text-align: center; padding: 20px 0;",
                                button {
                                    class: "btn-pill active",
                                    style: "padding: 16px 36px; font-size: 15px; font-weight: 700; display: inline-flex; align-items: center; gap: 10px; background: var(--accent-mint); color: #000; box-shadow: 0 0 24px rgba(45,212,191,0.3);",
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

                                // Neon Progress Bar
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
                    }
                }

                // ==========================================
                // STEP 3: SUMMARY & LAUNCH
                // ==========================================
                if *current_step.read() == 3 {
                    div { style: "width: 100%; display: flex; flex-direction: column; gap: 20px; align-items: center; text-align: center;",
                        div { style: "width: 56px; height: 56px; border-radius: 50%; background: rgba(45, 212, 191, 0.2); border: 2px solid var(--accent-mint); display: flex; align-items: center; justify-content: center; margin-bottom: 8px;",
                            span { class: "material-icons md-32", style: "color: var(--accent-mint);", "check" }
                        }

                        h2 { style: "font-size: 22px; font-weight: 700;", "Library Ready!" }
                        p { style: "color: var(--text-secondary); font-size: 13.5px; max-width: 500px; line-height: 1.5;",
                            "Your music library has been indexed with genuine audio profiles. Camelot harmonic flow pairings and Smart DJ Shuffle recipes are primed."
                        }

                        // Summary Statistics Cards
                        div { style: "display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; width: 100%; margin: 12px 0;",
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
                                span { style: "font-size: 10.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; display: block; margin-bottom: 4px;", "Startup Cache" }
                                span { style: "font-size: 14px; font-weight: 700; color: var(--accent-mint);",
                                    "< 5ms Instant"
                                }
                            }
                        }

                        button {
                            class: "btn-pill active",
                            style: "padding: 16px 42px; font-size: 16px; font-weight: 700; display: inline-flex; align-items: center; gap: 10px; background: var(--accent-mint); color: #000; box-shadow: 0 0 28px rgba(45,212,191,0.35); margin-top: 10px;",
                            onclick: {
                                let tracks = indexed_tracks.read().clone();
                                let dir = selected_dir.read().clone();
                                move |_| {
                                    crate::config::set_setup_completed(true, Some(dir.clone()));
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
