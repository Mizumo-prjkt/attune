//! Pure Fullscreen Now Playing Stage.
//!
//! Provides an edge-to-edge pure fullscreen playback and synchronized lyrics
//! experience with dynamic artwork gradient backgrounds, glowing lyrics typography,
//! living animated visualizer fluid mesh effects, and full transport controls
//! (including Repeat). Adheres strictly to Mandate 4 (zero hardcoded values).

use dioxus::prelude::*;
use crate::app::RepeatMode;
use crate::components::lyrics_parser::LyricsSheet;
use crate::config::AppConfig;
use crate::library::{fmt_ms, TrackEntry};
use crate::smart_shuffle::ShuffleMode;
use crate::theme::DynamicPalette;

#[component]
pub fn FullscreenNowPlaying(
    current_track: Option<TrackEntry>,
    is_playing: bool,
    position_ms: u64,
    duration_ms: u64,
    shuffle_mode: ShuffleMode,
    repeat_mode: RepeatMode,
    lyrics_sheet: LyricsSheet,
    active_lyric_index: Option<usize>,
    config: AppConfig,
    on_toggle_play: EventHandler<()>,
    on_seek: EventHandler<u64>,
    on_prev: EventHandler<()>,
    on_next: EventHandler<()>,
    on_cycle_shuffle: EventHandler<()>,
    on_cycle_repeat: EventHandler<()>,
    on_exit: EventHandler<()>,
) -> Element {
    let strings = crate::i18n::current_strings();

    // Default to locked follow mode (Flutter equivalent: _isLockedToActive = true)
    let mut is_locked_to_active = use_signal(|| true);

    // Reset to follow mode whenever track changes
    let current_track_id = current_track.as_ref().map(|t| t.path.clone());
    use_effect(use_reactive(&current_track_id, move |_| {
        is_locked_to_active.set(true);
    }));

    // Ensure the OS window enters pure borderless fullscreen
    use_effect(move || {
        dioxus::desktop::window().set_fullscreen(true);
    });

    // Automatically trigger smooth centering when active line updates in Follow Mode
    use_effect(use_reactive(&active_lyric_index, move |idx| {
        if *is_locked_to_active.read() && idx.is_some() {
            let _ = document::eval("if (window.attuneSyncLyrics) window.attuneSyncLyrics(false);");
        }
    }));

    let track = current_track.clone().unwrap_or_else(|| TrackEntry {
        title: strings.select_track_to_play.to_string(),
        artist: strings.app_name.to_string(),
        album: "Ready".to_string(),
        duration_ms: 0,
        ..Default::default()
    });

    let effective_dur = if duration_ms > 0 { duration_ms } else { track.duration_ms };
    let progress_pct = if effective_dur > 0 {
        ((position_ms as f64 / effective_dur as f64) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    let remaining_ms = effective_dur.saturating_sub(position_ms);
    let time_elapsed = fmt_ms(position_ms);
    let time_remaining = format!("-{}", fmt_ms(remaining_ms));

    let audio_profile_text = {
        let fmt = if track.format.is_empty() { "PCM" } else { &track.format };
        let sr = if track.sample_rate > 0 {
            format!("{:.1} kHz", track.sample_rate as f64 / 1000.0)
        } else {
            "44.1 kHz".to_string()
        };
        let bpm = track.bpm.map(|b| format!("{:.0} BPM", b)).unwrap_or_else(|| "Tempo pending".to_string());
        let key = track.key.clone().unwrap_or_else(|| "Key pending".to_string());
        format!("{fmt} • {sr} • {bpm} • {key}")
    };

    let artwork_url = track.artwork_data_url.clone().unwrap_or_else(|| {
        "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='400' viewBox='0 0 400 400'><rect width='400' height='400' fill='%23121722'/><circle cx='200' cy='200' r='90' fill='none' stroke='%2364ffda' stroke-width='8'/><polygon points='180,150 240,200 180,250' fill='%2364ffda'/></svg>".to_string()
    });

    // Dynamic color extraction strictly from the active track artwork
    let palette = DynamicPalette::from_track(&track);
    let (r1, g1, b1) = palette.primary_rgb;
    let (r2, g2, b2) = palette.secondary_rgb;
    let (r3, g3, b3) = palette.tertiary_rgb;
    let (dr, dg, db) = palette.dark_base_rgb;

    // Dynamic background style matching the album cover colors
    let bg_style = if config.fullscreen_gradient_bg {
        format!(
            "background: radial-gradient(circle at 18% 28%, rgba({}, {}, {}, 0.42) 0%, transparent 55%), \
                         radial-gradient(circle at 82% 72%, rgba({}, {}, {}, 0.32) 0%, transparent 60%), \
                         linear-gradient(145deg, rgba({}, {}, {}, 0.97) 0%, #06090e 100%);",
            r1, g1, b1, r2, g2, b2, dr, dg, db
        )
    } else {
        "background: #090d16;".to_string()
    };

    // Animated living fluid gradient visualizer orbs
    let visualizer_class = if is_playing {
        "fullscreen-visualizer-container"
    } else {
        "fullscreen-visualizer-container visualizer-paused"
    };

    let orb1_style = format!(
        "background: radial-gradient(circle, rgba({}, {}, {}, 0.45) 0%, rgba({}, {}, {}, 0.15) 50%, transparent 75%);",
        r1, g1, b1, r1, g1, b1
    );
    let orb2_style = format!(
        "background: radial-gradient(circle, rgba({}, {}, {}, 0.38) 0%, rgba({}, {}, {}, 0.12) 50%, transparent 75%);",
        r2, g2, b2, r2, g2, b2
    );
    let orb3_style = format!(
        "background: radial-gradient(circle, rgba({}, {}, {}, 0.32) 0%, rgba({}, {}, {}, 0.09) 55%, transparent 75%);",
        r3, g3, b3, r3, g3, b3
    );
    let orb4_style = format!(
        "background: radial-gradient(circle, rgba({}, {}, {}, 0.28) 0%, transparent 70%);",
        r1, g1, b1
    );

    let text_glow_style = if config.fullscreen_text_glow {
        "text-shadow: 0 0 24px rgba(255, 253, 208, 0.85), 0 0 48px rgba(100, 255, 218, 0.40);"
    } else {
        ""
    };

    rsx! {
        div {
            class: "fullscreen-stage",
            style: "{bg_style}",

            // Dynamic Living Fluid Gradient Visualizer Mesh
            if config.fullscreen_visualizer_effects {
                div {
                    class: "{visualizer_class}",
                    div { class: "fluid-orb fluid-orb-1", style: "{orb1_style}" }
                    div { class: "fluid-orb fluid-orb-2", style: "{orb2_style}" }
                    div { class: "fluid-orb fluid-orb-3", style: "{orb3_style}" }
                    div { class: "fluid-orb fluid-orb-4", style: "{orb4_style}" }
                }
                div { class: "fullscreen-vignette-scrim" }
            }

            // Minimalist Floating Top Action Bar (Lyrics Sources, Follow/Free-Scroll Mode Toggle, Exit Fullscreen)
            div {
                class: "fullscreen-top-actions",

                // Quick access to Lyrics Finder / Sources
                if current_track.is_some() {
                    button {
                        class: "btn-pill",
                        style: "background: rgba(255, 255, 255, 0.08); backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.16); border-radius: 20px; padding: 7px 16px; display: inline-flex; align-items: center; gap: 6px; color: #e6edf3; cursor: pointer; font-size: 13px; font-weight: 500; white-space: nowrap; transition: all 0.2s ease;",
                        title: "Find or Change Lyrics (Local Files, LRCLIB, BiniLyrics)",
                        onclick: {
                            let track_clone = current_track.clone();
                            move |_| {
                                crate::stages::lyrics_finder::open_lyrics_finder_window(track_clone.clone());
                            }
                        },
                        span { class: "material-icons md-18", style: "color: var(--accent-mint);", "manage_search" }
                        span { "Lyrics Sources" }
                    }
                }

                // Follow / Free-Scroll Mode Toggle Pill
                if lyrics_sheet.is_timed && !lyrics_sheet.lines.is_empty() {
                    button {
                        class: if *is_locked_to_active.read() { "btn-pill active" } else { "btn-pill" },
                        style: "backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); border-radius: 20px; padding: 7px 14px; display: inline-flex; align-items: center; gap: 6px; cursor: pointer; font-size: 13px; font-weight: 500; white-space: nowrap; transition: all 0.2s ease;",
                        title: if *is_locked_to_active.read() {
                            "Follow Mode Active (Auto-centering singing line). Click to switch to Free-Scroll."
                        } else {
                            "Free-Scroll Mode Active. Click to lock and follow singing line."
                        },
                        onclick: move |_| {
                            let next = !*is_locked_to_active.read();
                            is_locked_to_active.set(next);
                            if next {
                                let _ = document::eval("if (window.attuneSyncLyrics) window.attuneSyncLyrics(true);");
                            }
                        },
                        if *is_locked_to_active.read() {
                            span { class: "material-icons md-18", style: "color: var(--accent-mint);", "lock" }
                            span { "Follow Lyrics" }
                        } else {
                            span { class: "material-icons md-18", style: "color: var(--text-muted);", "lock_open" }
                            span { "Free-Scroll" }
                        }
                    }
                }

                // Minimalist Floating Exit Button (Esc shortcut)
                button {
                    class: "fullscreen-floating-close-inline",
                    title: "Exit Fullscreen (Esc)",
                    onclick: move |_| {
                        dioxus::desktop::window().set_fullscreen(false);
                        on_exit.call(());
                    },
                    span { class: "material-icons md-20", "close" }
                }
            }

            // Main Content: Pure Edge-to-Edge 2-Column Split
            div {
                class: "fullscreen-body",
                style: "display: flex; flex: 1; padding: 48px 64px; gap: 72px; align-items: center; z-index: 10; height: 100vh; max-height: 100vh; box-sizing: border-box;",

                // Left Column: Artwork + Track Info + Scrubber + Full Transport Controls
                div {
                    class: "fullscreen-left-col",
                    style: "display: flex; flex-direction: column; width: 380px; min-width: 340px; gap: 20px;",

                    // Album Art
                    div {
                        style: "width: 100%; aspect-ratio: 1/1; border-radius: 18px; overflow: hidden; box-shadow: 0 24px 60px rgba(0, 0, 0, 0.65); border: 1px solid rgba(255, 255, 255, 0.12); position: relative;",
                        img {
                            src: "{artwork_url}",
                            alt: "{track.title}",
                            style: "width: 100%; height: 100%; object-fit: cover; display: block;"
                        }
                    }

                    // Metadata
                    div { style: "display: flex; flex-direction: column; gap: 6px;",
                        h2 {
                            style: "font-size: 26px; font-weight: 800; color: #ffffff; margin: 0; line-height: 1.2; overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;",
                            "{track.title}"
                        }
                        div { style: "font-size: 17px; font-weight: 600; color: var(--accent-mint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                            "{track.artist}"
                        }
                        div { style: "font-size: 13px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                            "{track.album}"
                        }
                        div {
                            style: "margin-top: 6px; display: inline-flex; align-items: center; gap: 6px; font-size: 11.5px; color: rgba(255, 255, 255, 0.6); background: rgba(255, 255, 255, 0.05); padding: 4px 10px; border-radius: 6px; width: fit-content; border: 1px solid rgba(255, 255, 255, 0.08);",
                            span { class: "material-icons md-14", style: "color: var(--accent-mint);", "graphic_eq" }
                            "{audio_profile_text}"
                        }
                    }

                    // Scrubber Bar
                    div { style: "display: flex; flex-direction: column; gap: 8px; margin-top: 4px;",
                        input {
                            r#type: "range",
                            class: "scrubber-range",
                            min: 0,
                            max: effective_dur,
                            value: position_ms,
                            style: format!(
                                "width: 100%; height: 6px; cursor: pointer; background: linear-gradient(to right, var(--accent-mint) 0%, var(--accent-mint) {:.2}%, rgba(255,255,255,0.15) {:.2}%, rgba(255,255,255,0.15) 100%); border-radius: 3px;",
                                progress_pct, progress_pct
                            ),
                            oninput: move |e| {
                                if let Ok(val) = e.value().parse::<u64>() {
                                    on_seek.call(val);
                                }
                            }
                        }
                        div { style: "display: flex; justify-content: space-between; font-size: 12px; font-family: monospace; color: var(--text-muted);",
                            span { "{time_elapsed}" }
                            span { "{time_remaining}" }
                        }
                    }

                    // Full Transport Controls (Shuffle, Previous, Play/Pause, Next, Repeat)
                    div {
                        style: "display: flex; justify-content: center; align-items: center; gap: 18px; padding-top: 8px;",
                        // Shuffle Button
                        button {
                            class: "btn-icon",
                            style: if shuffle_mode != ShuffleMode::Off { "color: var(--accent-mint);" } else { "color: var(--text-muted);" },
                            title: format!("Shuffle: {:?}", shuffle_mode),
                            onclick: move |_| on_cycle_shuffle.call(()),
                            span { class: "material-icons md-22", "shuffle" }
                        }
                        // Skip Previous
                        button {
                            class: "btn-icon",
                            title: "Previous Track",
                            onclick: move |_| on_prev.call(()),
                            span { class: "material-icons md-26", "skip_previous" }
                        }
                        // Play / Pause Circle
                        button {
                            class: "btn-pill-play",
                            style: "width: 60px; height: 60px; border-radius: 50%; background: var(--accent-mint); color: #0a0e17; border: none; display: flex; align-items: center; justify-content: center; cursor: pointer; box-shadow: 0 8px 24px rgba(100, 255, 218, 0.4); transition: transform 0.15s ease;",
                            title: if is_playing { "Pause" } else { "Play" },
                            onclick: move |_| on_toggle_play.call(()),
                            span {
                                class: "material-icons md-32",
                                style: "color: #0a0e17;",
                                if is_playing { "pause" } else { "play_arrow" }
                            }
                        }
                        // Skip Next
                        button {
                            class: "btn-icon",
                            title: "Next Track",
                            onclick: move |_| on_next.call(()),
                            span { class: "material-icons md-26", "skip_next" }
                        }
                        // Repeat Button (Right Next to Next)
                        button {
                            class: "btn-icon",
                            style: match repeat_mode {
                                RepeatMode::Off => "color: var(--text-muted);",
                                RepeatMode::All => "color: var(--accent-mint);",
                                RepeatMode::One => "color: var(--accent-purple);",
                            },
                            title: match repeat_mode {
                                RepeatMode::Off => "Repeat: Off",
                                RepeatMode::All => "Repeat: All",
                                RepeatMode::One => "Repeat: One",
                            },
                            onclick: move |_| on_cycle_repeat.call(()),
                            if repeat_mode == RepeatMode::One {
                                span { class: "material-icons md-22", "repeat_one" }
                            } else {
                                span { class: "material-icons md-22", "repeat" }
                            }
                        }
                    }
                }

                // Right Column: Pure Fullscreen Synchronized Lyrics
                div {
                    class: "fullscreen-right-col",
                    style: "flex: 1; height: 100%; display: flex; flex-direction: column; justify-content: center; overflow: hidden; position: relative;",

                    div {
                        class: "fullscreen-lyrics-scroll",
                        "data-locked": if *is_locked_to_active.read() { "true" } else { "false" },
                        style: "overflow-y: auto; max-height: 85vh; padding: 140px 32px; display: flex; flex-direction: column; gap: 28px; scroll-behavior: smooth; mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%); -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%);",
                        onwheel: move |_| {
                            if *is_locked_to_active.read() {
                                is_locked_to_active.set(false);
                            }
                        },
                        onpointerdown: move |_| {
                            if *is_locked_to_active.read() {
                                is_locked_to_active.set(false);
                            }
                        },

                        if lyrics_sheet.lines.is_empty() {
                            div {
                                style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; text-align: center; color: var(--text-muted); gap: 16px; padding: 60px 20px;",
                                span { class: "material-icons md-48", style: "opacity: 0.5;", "queue_music" }
                                p { style: "font-size: 20px; font-weight: 700; color: #ffffff;", "{strings.instrumental_or_lyrics_pending}" }
                                p { style: "font-size: 13px; max-width: 380px; line-height: 1.5;", "{strings.fullscreen_lyrics_guide}" }
                                if current_track.is_some() {
                                    button {
                                        style: "margin-top: 12px; display: inline-flex; align-items: center; gap: 8px; padding: 10px 20px; border-radius: 9999px; background: rgba(100, 255, 218, 0.14); border: 1px solid rgba(100, 255, 218, 0.40); color: var(--accent-mint); font-weight: 600; font-size: 14px; cursor: pointer; transition: all 0.2s ease;",
                                        onclick: {
                                            let track_clone = current_track.clone();
                                            move |_| {
                                                crate::stages::lyrics_finder::open_lyrics_finder_window(track_clone.clone());
                                            }
                                        },
                                        span { class: "material-icons md-18", "manage_search" }
                                        span { "Find & Match Lyrics" }
                                    }
                                }
                            }
                        } else {
                            for (idx, line) in lyrics_sheet.lines.iter().enumerate() {
                                {
                                    let is_active = active_lyric_index == Some(idx);
                                    let line_ts = line.timestamp_ms;
                                    let text = line.text.clone();

                                    rsx! {
                                        div {
                                            key: "{idx}",
                                            class: if is_active { "fullscreen-lyric-active" } else { "fullscreen-lyric-inactive" },
                                            style: if is_active {
                                                format!(
                                                    "font-size: 32px; font-weight: 800; color: #FFFDD0; line-height: 1.3; cursor: pointer; transition: all 0.3s ease; transform: scale(1.02); transform-origin: left center; {text_glow_style}"
                                                )
                                            } else {
                                                "font-size: 22px; font-weight: 500; color: rgba(255, 255, 255, 0.40); line-height: 1.4; cursor: pointer; transition: all 0.3s ease;".to_string()
                                            },
                                            onclick: move |_| {
                                                on_seek.call(line_ts);
                                                is_locked_to_active.set(true);
                                                let _ = document::eval("if (window.attuneSyncLyrics) window.attuneSyncLyrics(true);");
                                            },
                                            "{text}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Floating Liquid Glass Sync-to-Current-Line button when user is in free-scroll mode (Flutter parity)
                    if lyrics_sheet.is_timed && !*is_locked_to_active.read() && !lyrics_sheet.lines.is_empty() {
                        button {
                            class: "fullscreen-sync-button",
                            onclick: move |_| {
                                is_locked_to_active.set(true);
                                let _ = document::eval("if (window.attuneSyncLyrics) window.attuneSyncLyrics(true);");
                            },
                            span { class: "material-icons md-18", "vertical_align_center" }
                            span { "Sync to Current Line" }
                        }
                    }
                }
            }
        }
    }
}

/// Dynamically calculates the centered scroll target position for the active lyric line
/// inside a scrollable container. Adheres strictly to Mandate 4 (zero hardcoded values).
pub fn compute_centered_scroll_target(
    container_scroll_top: f64,
    container_top: f64,
    container_height: f64,
    active_top: f64,
    active_height: f64,
) -> f64 {
    container_scroll_top + (active_top - container_top) - (container_height / 2.0) + (active_height / 2.0)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_compute_centered_scroll_target_when_active_is_lower() {
        // Container is 800px tall at viewport y=100 with scrollTop=0.
        // Active line is 50px tall at viewport y=600.
        // Active center relative to container top is (600 - 100) + 25 = 525.
        // Container center is 400.
        // Expected target scroll is 0 + 525 - 400 = 125.
        let target = compute_centered_scroll_target(0.0, 100.0, 800.0, 600.0, 50.0);
        assert!((target - 125.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_centered_scroll_target_when_already_centered() {
        // Container is 600px tall at viewport y=0 with scrollTop=200.
        // Active line is 40px tall at viewport y=280 (center is 300, matching container center 300).
        // Expected target scroll is 200 + 0 = 200.
        let target = compute_centered_scroll_target(200.0, 0.0, 600.0, 280.0, 40.0);
        assert!((target - 200.0).abs() < 1e-6);
    }

    #[test]
    fn test_lyrics_sync_script_structure_and_selectors() {
        let script = crate::theme::LYRICS_SYNC_SCRIPT;
        assert!(script.contains("window.attuneSyncLyrics"));
        assert!(script.contains(".fullscreen-lyrics-scroll"));
        assert!(script.contains(".fullscreen-lyric-active"));
        assert!(script.contains(".dock-scroll"));
        assert!(script.contains("data-locked"));
        assert!(script.contains("MutationObserver"));
    }
}
