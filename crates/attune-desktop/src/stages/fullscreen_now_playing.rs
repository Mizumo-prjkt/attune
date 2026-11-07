//! Fullscreen Now Playing Stage (Lucky Guy Immersive Mode).
//!
//! Provides an edge-to-edge full-window playback and synchronized lyrics
//! experience with dynamic artwork gradient backgrounds, glowing lyrics typography,
//! and background visualizer effects. Adheres strictly to Mandate 4.

use dioxus::prelude::*;
use crate::components::lyrics_parser::LyricsSheet;
use crate::config::AppConfig;
use crate::library::{fmt_ms, TrackEntry};
use crate::smart_shuffle::ShuffleMode;

#[component]
pub fn FullscreenNowPlaying(
    current_track: Option<TrackEntry>,
    is_playing: bool,
    position_ms: u64,
    duration_ms: u64,
    shuffle_mode: ShuffleMode,
    lyrics_sheet: LyricsSheet,
    active_lyric_index: Option<usize>,
    config: AppConfig,
    on_toggle_play: EventHandler<()>,
    on_seek: EventHandler<u64>,
    on_prev: EventHandler<()>,
    on_next: EventHandler<()>,
    on_cycle_shuffle: EventHandler<()>,
    on_exit: EventHandler<()>,
) -> Element {
    let track = current_track.unwrap_or_else(|| TrackEntry {
        title: "No Track Playing".to_string(),
        artist: "Attune Engine".to_string(),
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

    // Background style based on config
    let bg_style = if config.fullscreen_gradient_bg {
        "background: radial-gradient(circle at 25% 40%, rgba(20, 35, 60, 0.90) 0%, rgba(10, 14, 23, 0.98) 70%), linear-gradient(135deg, #0d131f 0%, #06090e 100%);"
    } else {
        "background: #090d16;"
    };

    let text_glow_style = if config.fullscreen_text_glow {
        "text-shadow: 0 0 24px rgba(255, 253, 208, 0.85), 0 0 48px rgba(100, 255, 218, 0.40);"
    } else {
        ""
    };

    rsx! {
        div {
            class: "fullscreen-stage",
            style: "{bg_style}",

            // Optional background visualizer pulse effects
            if config.fullscreen_visualizer_effects {
                div {
                    class: "fullscreen-visualizer-orb",
                    style: "position: absolute; top: 15%; right: 20%; width: 500px; height: 500px; border-radius: 50%; background: radial-gradient(circle, rgba(100, 255, 218, 0.08) 0%, transparent 70%); filter: blur(50px); pointer-events: none; animation: orbPulse 6s ease-in-out infinite alternate;"
                }
                div {
                    class: "fullscreen-visualizer-orb-2",
                    style: "position: absolute; bottom: 10%; left: 15%; width: 420px; height: 420px; border-radius: 50%; background: radial-gradient(circle, rgba(168, 85, 247, 0.08) 0%, transparent 70%); filter: blur(60px); pointer-events: none; animation: orbPulse 8s ease-in-out infinite alternate-reverse;"
                }
            }

            // Top Header: Title & Close Button
            div {
                class: "fullscreen-header",
                style: "display: flex; justify-content: space-between; align-items: center; padding: 24px 36px; z-index: 10;",
                div { style: "display: flex; align-items: center; gap: 10px; opacity: 0.8;",
                    span { class: "material-icons md-20", style: "color: var(--accent-mint);", "auto_awesome" }
                    span { style: "font-size: 13px; font-weight: 600; letter-spacing: 1px; text-transform: uppercase; color: var(--text-secondary);",
                        "Attune Immersive View"
                    }
                }
                button {
                    class: "btn-icon",
                    style: "background: rgba(255, 255, 255, 0.08); width: 40px; height: 40px; border-radius: 50%; border: 1px solid var(--border-glass); cursor: pointer; transition: all 0.2s;",
                    title: "Exit Fullscreen (Esc)",
                    onclick: move |_| on_exit.call(()),
                    span { class: "material-icons md-22", "close" }
                }
            }

            // Main Content: 2-Column Split
            div {
                class: "fullscreen-body",
                style: "display: flex; flex: 1; padding: 10px 48px 40px 48px; gap: 64px; align-items: center; z-index: 10; max-height: calc(100vh - 100px); box-sizing: border-box;",

                // Left Column: Artwork + Track Info + Scrubber + Transport
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

                    // Transport Controls
                    div {
                        style: "display: flex; justify-content: center; align-items: center; gap: 20px; padding-top: 8px;",
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
                    }
                }

                // Right Column: Synchronized Lyrics
                div {
                    class: "fullscreen-right-col",
                    style: "flex: 1; height: 100%; display: flex; flex-direction: column; justify-content: center; overflow: hidden; position: relative;",

                    div {
                        class: "fullscreen-lyrics-scroll",
                        style: "overflow-y: auto; max-height: 80vh; padding: 120px 24px; display: flex; flex-direction: column; gap: 28px; scroll-behavior: smooth; mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%); -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%);",

                        if lyrics_sheet.lines.is_empty() {
                            div {
                                style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; text-align: center; color: var(--text-muted); gap: 16px; padding: 60px 20px;",
                                span { class: "material-icons md-48", style: "opacity: 0.5;", "queue_music" }
                                p { style: "font-size: 20px; font-weight: 700; color: #ffffff;", "Instrumental or Lyrics Pending" }
                                p { style: "font-size: 13px; max-width: 380px; line-height: 1.5;", "Synchronized singing lines will appear here automatically when a companion .lrc file is provided or fetched via API." }
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
                                            onclick: move |_| on_seek.call(line_ts),
                                            "{text}"
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
}
