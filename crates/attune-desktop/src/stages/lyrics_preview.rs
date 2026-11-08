//! Standalone Secondary Window: Lyrics Live Animation & Style Preview.
//!
//! Provides a dedicated real-time preview window allowing users to immediately inspect
//! and verify their lyrics animations, layout placements, text alignments, glowing shadows,
//! and custom CSS keyframe scripts without having to start playback or toggle full screen.
//!
//! Synchronizes live with AppConfig changes every 500ms so edits made in the Settings
//! window reflect instantly.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use dioxus::prelude::*;

use crate::config::load_config;

pub static LYRICS_PREVIEW_WINDOW_OPEN: AtomicBool = AtomicBool::new(false);

/// RAII Drop Guard ensuring the window open atomic latch is cleared upon closure.
struct PreviewWindowGuard;

impl Drop for PreviewWindowGuard {
    fn drop(&mut self) {
        LYRICS_PREVIEW_WINDOW_OPEN.store(false, Ordering::SeqCst);
    }
}

/// Sample Track representation for the live lyrics preview.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewSampleTrack {
    pub title: &'static str,
    pub artist: &'static str,
    pub album: &'static str,
    pub audio_format: &'static str,
    pub primary_rgb: (u8, u8, u8),
    pub secondary_rgb: (u8, u8, u8),
    pub dark_rgb: (u8, u8, u8),
    pub accent_hex: &'static str,
    pub lyrics: &'static [&'static str],
}

pub const SAMPLE_TRACKS: &[PreviewSampleTrack] = &[
    PreviewSampleTrack {
        title: "(Not) A Devil",
        artist: "DECO*27, PinocchioP",
        album: "(Not) A Devil",
        audio_format: "FLAC • 44.1 kHz • 149 BPM • C Major",
        primary_rgb: (0, 245, 212),
        secondary_rgb: (255, 0, 127),
        dark_rgb: (13, 17, 23),
        accent_hex: "#00f5d4",
        lyrics: &[
            "DECO",
            "PINO",
            "堕",
            "ながらの優等生 今宵も自分と密会です",
            "しんどいわ しんどいわ あーつらつら合掌",
            "ほんとは劣等生 あたしはあたしに説教です",
            "しんどいわ しんどいわ もーいやいや絶交",
            "いいから はやく ハンコ押せや",
            "デビルじゃない デビルじゃないもん 悪魔ほどそう言うもん",
            "あっけなく 散った 夢見た未来",
        ],
    },
    PreviewSampleTrack {
        title: "Ready",
        artist: "Attune",
        album: "Attune Dynamic DJ Sessions Vol. 1",
        audio_format: "FLAC • 96.0 kHz • 128 BPM • F Minor",
        primary_rgb: (45, 212, 191),
        secondary_rgb: (168, 85, 247),
        dark_rgb: (10, 14, 23),
        accent_hex: "#2dd4bf",
        lyrics: &[
            "Step onto the floor, the frequency is clear",
            "Feel the sub-bass rising in the atmosphere",
            "Lock the tempo, harmonize the keys",
            "Attune your soul to the digital breeze",
            "Transition seamless, never skip a beat",
            "From peak time energy to twilight retreat",
            "Pure resonance, dynamic flow",
            "Let the music take control",
        ],
    },
    PreviewSampleTrack {
        title: "YUMEYUME",
        artist: "DECO*27 feat. Hatsune Miku",
        album: "HATSUNE MIKU Project mirai COMPLETE",
        audio_format: "FLAC • 44.1 kHz • 80 BPM • F Major",
        primary_rgb: (57, 197, 187),
        secondary_rgb: (255, 182, 193),
        dark_rgb: (15, 20, 28),
        accent_hex: "#39c5bb",
        lyrics: &[
            "僕のことはさて置き",
            "君のことが聞きたいんだ",
            "色々あるんだホント",
            "それじゃあ今日はよろしくどうぞ",
            "いつも輝いてる 君の色はなんだったっけ？",
            "たくさんあったはずなのに",
            "今ではもう覚えてないんだ",
            "夢の続きを見に行こう",
        ],
    },
];

/// Opens the Lyrics Live Animation & Style Preview in a dedicated secondary OS window.
pub fn open_lyrics_preview_window() {
    if LYRICS_PREVIEW_WINDOW_OPEN
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let dom = VirtualDom::new(LyricsPreviewStandaloneWindow);
        let mut window = dioxus::desktop::WindowBuilder::new()
            .with_title("Attune — Lyrics Live Animation Preview")
            .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1120.0, 720.0))
            .with_min_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(820.0, 520.0));

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

/// Standalone Window Root Component for Lyrics Live Animation Preview.
#[component]
pub fn LyricsPreviewStandaloneWindow() -> Element {
    use_hook(|| Arc::new(PreviewWindowGuard));

    let mut config_sig = use_signal(load_config);
    let mut selected_track_idx = use_signal(|| 0usize);
    let mut active_line_idx = use_signal(|| 3usize);
    let mut is_playing = use_signal(|| true);
    let mut sim_speed = use_signal(|| 1.0f32);
    let mut preview_mode = use_signal(|| "fullscreen".to_string()); // "fullscreen" | "sidebar"

    // Live two-way config synchronization: re-reads settings from disk every 500ms
    // so edits made in the Settings window immediately reflect in this preview window.
    use_future(move || async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let latest = load_config();
            if latest != *config_sig.peek() {
                config_sig.set(latest);
            }
        }
    });

    // Automatic timed lyric line advancement simulating singing progression
    use_future(move || async move {
        loop {
            let speed = *sim_speed.read();
            let interval_ms = (2600.0 / speed.max(0.2)).round() as u64;
            tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;

            if *is_playing.read() {
                let track = &SAMPLE_TRACKS[*selected_track_idx.read() % SAMPLE_TRACKS.len()];
                let total = track.lyrics.len();
                if total > 0 {
                    let next = (*active_line_idx.read() + 1) % total;
                    active_line_idx.set(next);
                }
            }
        }
    });

    let curr_cfg = config_sig.read().clone();
    let strings = crate::i18n::get_strings(crate::i18n::Language::from_code(&curr_cfg.language));
    let track = &SAMPLE_TRACKS[*selected_track_idx.read() % SAMPLE_TRACKS.len()];
    let total_lines = track.lyrics.len();
    let curr_line = *active_line_idx.read() % total_lines.max(1);

    // Dynamic background style matching track artwork palette
    let (r1, g1, b1) = track.primary_rgb;
    let (r2, g2, b2) = track.secondary_rgb;
    let (dr, dg, db) = track.dark_rgb;
    let dynamic_bg_style = if curr_cfg.fullscreen_gradient_bg {
        format!(
            "background: radial-gradient(circle at 18% 28%, rgba({}, {}, {}, 0.40) 0%, transparent 55%), \
                         radial-gradient(circle at 82% 72%, rgba({}, {}, {}, 0.30) 0%, transparent 60%), \
                         linear-gradient(145deg, rgba({}, {}, {}, 0.96) 0%, #06090e 100%);",
            r1, g1, b1, r2, g2, b2, dr, dg, db
        )
    } else {
        "background: #080c14;".to_string()
    };

    rsx! {
        div {
            class: "lyrics-preview-container",
            style: "width: 100vw; height: 100vh; display: flex; flex-direction: column; background: #06090e; color: #ffffff; overflow: hidden; position: relative; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;",

            // Inject custom CSS dynamically when enabled
            if curr_cfg.enable_lyrics_custom_css && !curr_cfg.lyrics_custom_css.trim().is_empty() {
                style {
                    "{curr_cfg.lyrics_custom_css}"
                }
            }

            // Top Control & Diagnostic Toolbar
            header {
                style: "height: 64px; background: rgba(10, 14, 23, 0.88); backdrop-filter: blur(16px); border-bottom: 1px solid var(--border-glass); display: flex; justify-content: space-between; align-items: center; padding: 0 20px; z-index: 100; flex-shrink: 0;",

                // Left: Title + Realtime Status Indicator
                div { style: "display: flex; align-items: center; gap: 12px;",
                    div { class: "live-pulse-dot", style: "width: 9px; height: 9px; border-radius: 50%; background: var(--accent-mint); box-shadow: 0 0 10px var(--accent-mint);" }
                    div {
                        h1 { style: "font-size: 14.5px; font-weight: 700; margin: 0; line-height: 1.2;", "{strings.preview_lyrics_window_title}" }
                        p { style: "font-size: 11px; color: var(--text-muted); margin: 0;", "{strings.preview_notice_realtime}" }
                    }
                }

                // Center: Viewport Mode Switcher & Track Selector
                div { style: "display: flex; align-items: center; gap: 10px;",
                    // Viewport Mode Pills
                    div { style: "display: flex; background: rgba(255,255,255,0.06); padding: 3px; border-radius: 20px; border: 1px solid var(--border-glass); gap: 2px;",
                        button {
                            class: if *preview_mode.read() == "fullscreen" { "btn-pill active" } else { "btn-pill" },
                            style: "font-size: 11.5px; padding: 4px 12px; border: none;",
                            onclick: move |_| preview_mode.set("fullscreen".to_string()),
                            span { class: "material-icons md-14", "fullscreen" }
                            "{strings.preview_fullscreen_mode}"
                        }
                        button {
                            class: if *preview_mode.read() == "sidebar" { "btn-pill active" } else { "btn-pill" },
                            style: "font-size: 11.5px; padding: 4px 12px; border: none;",
                            onclick: move |_| preview_mode.set("sidebar".to_string()),
                            span { class: "material-icons md-14", "view_sidebar" }
                            "{strings.preview_sidebar_mode}"
                        }
                    }

                    // Track Picker
                    select {
                        class: "form-select",
                        style: "font-size: 11.5px; padding: 5px 12px; max-width: 170px;",
                        onchange: move |e| {
                            if let Ok(idx) = e.value().parse::<usize>() {
                                selected_track_idx.set(idx);
                                active_line_idx.set(0);
                            }
                        },
                        for (i, t) in SAMPLE_TRACKS.iter().enumerate() {
                            option {
                                value: "{i}",
                                selected: *selected_track_idx.read() == i,
                                "{t.title} — {t.artist}"
                            }
                        }
                    }
                }

                // Right: Playback & Line Advancement Simulation Controls
                div { style: "display: flex; align-items: center; gap: 8px;",
                    // Prev Line
                    button {
                        class: "btn-icon",
                        style: "width: 32px; height: 32px; color: var(--text-secondary);",
                        title: "Previous Lyric Line",
                        onclick: move |_| {
                            let curr = *active_line_idx.read();
                            active_line_idx.set((curr + total_lines - 1) % total_lines);
                        },
                        span { class: "material-icons md-18", "skip_previous" }
                    }

                    // Play/Pause Simulation
                    button {
                        class: "btn-pill-play",
                        style: "width: 34px; height: 34px; border-radius: 50%; background: var(--accent-mint); color: #0a0e17; border: none; display: flex; align-items: center; justify-content: center; cursor: pointer;",
                        title: if *is_playing.read() { "Pause Simulation" } else { "Play Simulation" },
                        onclick: move |_| {
                            let next = !*is_playing.read();
                            is_playing.set(next);
                        },
                        span {
                            class: "material-icons md-20",
                            style: "color: #0a0e17;",
                            if *is_playing.read() { "pause" } else { "play_arrow" }
                        }
                    }

                    // Next Line
                    button {
                        class: "btn-icon",
                        style: "width: 32px; height: 32px; color: var(--text-secondary);",
                        title: "Next Lyric Line",
                        onclick: move |_| {
                            let curr = *active_line_idx.read();
                            active_line_idx.set((curr + 1) % total_lines);
                        },
                        span { class: "material-icons md-18", "skip_next" }
                    }

                    // Reset / Restart Line 0
                    button {
                        class: "btn-icon",
                        style: "width: 32px; height: 32px; color: var(--text-muted);",
                        title: "Restart Simulation from Line 1",
                        onclick: move |_| active_line_idx.set(0),
                        span { class: "material-icons md-18", "restart_alt" }
                    }

                    // Line Counter Pill
                    span {
                        style: "font-family: monospace; font-size: 11px; background: rgba(255,255,255,0.06); padding: 4px 8px; border-radius: 6px; border: 1px solid var(--border-glass); color: var(--accent-mint);",
                        "{strings.preview_simulating_line} {curr_line + 1}/{total_lines}"
                    }

                    // Speed Toggle
                    button {
                        class: "btn-pill",
                        style: "font-size: 11px; padding: 3px 8px; font-family: monospace;",
                        onclick: move |_| {
                            let next = match *sim_speed.read() {
                                s if s < 0.9 => 1.0f32,
                                s if s < 1.4 => 2.0f32,
                                _ => 0.5f32,
                            };
                            sim_speed.set(next);
                        },
                        "{sim_speed.read():.1}x"
                    }
                }
            }

            // Secondary Status Information Subheader
            div {
                style: "height: 32px; background: rgba(0,0,0,0.40); border-bottom: 1px solid rgba(255,255,255,0.05); display: flex; align-items: center; justify-content: space-between; padding: 0 20px; font-size: 11.5px; z-index: 90; flex-shrink: 0;",
                div { style: "display: flex; gap: 14px; align-items: center;",
                    span { style: "color: var(--text-muted);",
                        "Layout: "
                        strong { style: "color: var(--text-primary);",
                            if curr_cfg.fullscreen_layout_side == "right" { "Controls Right, Lyrics Left" } else { "Controls Left, Lyrics Right" }
                        }
                    }
                    span { style: "color: var(--text-muted);",
                        "Alignment: "
                        strong { style: "color: var(--text-primary); text-transform: capitalize;",
                            if curr_cfg.enable_lyrics_custom_css { "Custom CSS Controlled" } else { "{curr_cfg.fullscreen_lyrics_align}" }
                        }
                    }
                    span { style: "color: var(--text-muted);",
                        "Glow: "
                        strong { style: "color: if curr_cfg.fullscreen_text_glow { \"var(--accent-mint)\" } else { \"var(--text-muted)\" };",
                            if curr_cfg.fullscreen_text_glow { "Active" } else { "Disabled" }
                        }
                    }
                }
                div {
                    if curr_cfg.enable_lyrics_custom_css {
                        span {
                            style: "font-size: 10.5px; padding: 2px 8px; border-radius: 4px; background: rgba(255, 170, 0, 0.15); color: #ffa726; border: 1px solid rgba(255, 170, 0, 0.35); font-weight: 600;",
                            "Custom CSS Active (Overriding Default Alignment)"
                        }
                    } else {
                        span {
                            style: "font-size: 10.5px; padding: 2px 8px; border-radius: 4px; background: rgba(45, 212, 191, 0.12); color: var(--accent-mint); font-weight: 600;",
                            "Standard Declarative Styling"
                        }
                    }
                }
            }

            // Main Preview Stage
            main {
                style: format!("flex: 1; width: 100%; position: relative; overflow: hidden; display: flex; {}", dynamic_bg_style),

                // Background Pulse Color Orbs (if enabled in config)
                if curr_cfg.fullscreen_visualizer_effects {
                    div {
                        style: "position: absolute; top: 15%; left: 10%; width: 420px; height: 420px; border-radius: 50%; background: radial-gradient(circle, rgba(45,212,191,0.22) 0%, transparent 70%); filter: blur(50px); pointer-events: none; animation: floatOrb1 12s infinite alternate ease-in-out;"
                    }
                    div {
                        style: "position: absolute; bottom: 10%; right: 15%; width: 480px; height: 480px; border-radius: 50%; background: radial-gradient(circle, rgba(168,85,247,0.18) 0%, transparent 70%); filter: blur(60px); pointer-events: none; animation: floatOrb2 16s infinite alternate ease-in-out;"
                    }
                }

                // VIEWPORT BRANCH: Fullscreen Mode vs Sidebar Mode
                if *preview_mode.read() == "fullscreen" {
                    // Fullscreen Split Viewport Mockup
                    div {
                        class: "fullscreen-body",
                        style: if curr_cfg.fullscreen_layout_side == "right" {
                            "display: flex; flex-direction: row-reverse; flex: 1; padding: 32px 50px; gap: 48px; align-items: center; z-index: 10; height: 100%; box-sizing: border-box;"
                        } else {
                            "display: flex; flex-direction: row; flex: 1; padding: 32px 50px; gap: 48px; align-items: center; z-index: 10; height: 100%; box-sizing: border-box;"
                        },

                        // Left Column (Artwork, Track Details, Scrubber, Controls)
                        div {
                            class: "fullscreen-left-col",
                            style: "display: flex; flex-direction: column; width: 320px; min-width: 280px; gap: 16px; flex-shrink: 0;",

                            // Album Artwork Mock Box
                            div {
                                class: "album-cover-box",
                                style: "width: 100%; aspect-ratio: 1/1; border-radius: 16px; overflow: hidden; box-shadow: 0 20px 50px rgba(0, 0, 0, 0.65); border: 1px solid rgba(255, 255, 255, 0.12); position: relative; background: #121722; display: flex; align-items: center; justify-content: center;",
                                // Geometric placeholder artwork matching primary color
                                svg {
                                    style: "width: 100%; height: 100%; display: block;",
                                    view_box: "0 0 320 320",
                                    defs {
                                        linearGradient { id: "artGrad", x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                                            stop { offset: "0%", stop_color: "{track.accent_hex}", stop_opacity: "0.85" }
                                            stop { offset: "100%", stop_color: "rgba(168,85,247,0.75)", stop_opacity: "0.85" }
                                        }
                                    }
                                    rect { width: "320", height: "320", fill: "#111827" }
                                    circle { cx: "160", cy: "160", r: "90", fill: "url(#artGrad)", opacity: "0.85" }
                                    polygon { points: "145,120 200,160 145,200", fill: "#ffffff" }
                                }
                            }

                            // Metadata
                            div { style: "display: flex; flex-direction: column; gap: 4px;",
                                h2 { style: "font-size: 22px; font-weight: 800; color: #ffffff; margin: 0; line-height: 1.2;", "{track.title}" }
                                div { style: "font-size: 15px; font-weight: 600; color: var(--accent-mint);", "{track.artist}" }
                                div { style: "font-size: 12px; color: var(--text-muted);", "{track.album}" }
                                div {
                                    style: "margin-top: 4px; display: inline-flex; align-items: center; gap: 6px; font-size: 11px; color: rgba(255, 255, 255, 0.6); background: rgba(255, 255, 255, 0.05); padding: 3px 8px; border-radius: 6px; width: fit-content; border: 1px solid rgba(255, 255, 255, 0.08);",
                                    span { class: "material-icons md-14", style: "color: var(--accent-mint);", "graphic_eq" }
                                    "{track.audio_format}"
                                }
                            }

                            // Simulated Scrubber Bar
                            div { style: "display: flex; flex-direction: column; gap: 6px; margin-top: 2px;",
                                div { style: "width: 100%; height: 5px; background: rgba(255,255,255,0.15); border-radius: 3px; position: relative; overflow: hidden;",
                                    div {
                                        style: format!(
                                            "height: 100%; width: {:.1}%; background: var(--accent-mint); border-radius: 3px; transition: width 0.3s ease;",
                                            ((curr_line as f32 + 1.0) / total_lines as f32) * 100.0
                                        )
                                    }
                                }
                                div { style: "display: flex; justify-content: space-between; font-size: 11px; font-family: monospace; color: var(--text-muted);",
                                    span { "0:{curr_line * 20 + 12:02}" }
                                    span { "-2:48" }
                                }
                            }

                            // Transport Controls Mockup
                            div { style: "display: flex; justify-content: center; align-items: center; gap: 16px; padding-top: 4px;",
                                button { class: "btn-icon", style: "color: var(--accent-mint);", span { class: "material-icons md-20", "shuffle" } }
                                button { class: "btn-icon", style: "color: var(--text-secondary);", span { class: "material-icons md-22", "skip_previous" } }
                                button {
                                    class: "btn-pill-play",
                                    style: "width: 48px; height: 48px; border-radius: 50%; background: var(--accent-mint); color: #0a0e17; border: none; display: flex; align-items: center; justify-content: center; cursor: pointer; box-shadow: 0 6px 18px rgba(100, 255, 218, 0.4);",
                                    span { class: "material-icons md-26", style: "color: #0a0e17;", "pause" }
                                }
                                button { class: "btn-icon", style: "color: var(--text-secondary);", span { class: "material-icons md-22", "skip_next" } }
                                button { class: "btn-icon", style: "color: var(--accent-mint);", span { class: "material-icons md-20", "repeat" } }
                            }
                        }

                        // Right Column (Fullscreen Lyrics Viewport)
                        div {
                            class: "fullscreen-right-col",
                            style: "flex: 1; height: 100%; display: flex; flex-direction: column; justify-content: center; overflow: hidden; position: relative;",

                            div {
                                class: "fullscreen-lyrics-scroll",
                                style: "overflow-y: auto; max-height: 80vh; padding: 25vh 24px; display: flex; flex-direction: column; gap: 24px; scroll-behavior: smooth; mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%); -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%);",

                                for (idx, line_text) in track.lyrics.iter().enumerate() {
                                    {
                                        let is_active = idx == curr_line;
                                        let align_class = if !curr_cfg.enable_lyrics_custom_css {
                                            match curr_cfg.fullscreen_lyrics_align.as_str() {
                                                "center" => " align-center",
                                                "right" => " align-right",
                                                _ => " align-left",
                                            }
                                        } else {
                                            ""
                                        };

                                        let line_class = if is_active {
                                            if curr_cfg.fullscreen_text_glow {
                                                format!("fullscreen-lyric-line fullscreen-lyric-active glow{}", align_class)
                                            } else {
                                                format!("fullscreen-lyric-line fullscreen-lyric-active{}", align_class)
                                            }
                                        } else {
                                            format!("fullscreen-lyric-line fullscreen-lyric-inactive{}", align_class)
                                        };

                                        rsx! {
                                            div {
                                                key: "{idx}",
                                                "data-line-idx": "{idx}",
                                                class: "{line_class}",
                                                style: "cursor: pointer;",
                                                title: "Click to jump simulation to this line",
                                                onclick: move |_| {
                                                    active_line_idx.set(idx);
                                                },
                                                "{line_text}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Sidebar Drawer Viewport Mockup
                    div {
                        style: "display: flex; flex: 1; height: 100%; align-items: center; justify-content: center; padding: 30px;",

                        div {
                            style: "width: 360px; height: 90%; background: rgba(14, 20, 32, 0.85); backdrop-filter: blur(24px); border-radius: 18px; border: 1px solid var(--border-glass); box-shadow: 0 24px 60px rgba(0,0,0,0.7); display: flex; flex-direction: column; overflow: hidden;",

                            // Sidebar Header
                            div {
                                style: "padding: 16px 20px; border-bottom: 1px solid var(--border-glass); display: flex; justify-content: space-between; align-items: center;",
                                span { style: "font-weight: 700; font-size: 14px;", "Synchronized Lyrics" }
                                div { style: "display: flex; gap: 6px;",
                                    span { class: "badge-pill quality-good", style: "font-size: 10px;", "Auto-Scroll" }
                                }
                            }

                            // Sidebar Scrolling Lines
                            div {
                                style: "flex: 1; overflow-y: auto; padding: 30px 20px; display: flex; flex-direction: column; gap: 18px;",

                                for (idx, line_text) in track.lyrics.iter().enumerate() {
                                    {
                                        let is_active = idx == curr_line;
                                        let align_class = match curr_cfg.dock_lyrics_align.as_str() {
                                            "center" => "align-center",
                                            "right" => "align-right",
                                            _ => "align-left",
                                        };

                                        let line_style = if is_active {
                                            if curr_cfg.sidebar_text_glow {
                                                "font-size: 16px; font-weight: 700; color: #ffffff; text-shadow: 0 0 16px rgba(255,255,255,0.7); opacity: 1; cursor: pointer; transition: all 0.25s ease;"
                                            } else {
                                                "font-size: 16px; font-weight: 700; color: #ffffff; opacity: 1; cursor: pointer; transition: all 0.25s ease;"
                                            }
                                        } else {
                                            "font-size: 13.5px; font-weight: 500; color: rgba(255,255,255,0.40); cursor: pointer; transition: all 0.25s ease;"
                                        };

                                        rsx! {
                                            div {
                                                key: "{idx}",
                                                class: "lyric-line {align_class}",
                                                style: "{line_style}",
                                                onclick: move |_| {
                                                    active_line_idx.set(idx);
                                                },
                                                "{line_text}"
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_tracks_non_empty_and_valid() {
        assert!(!SAMPLE_TRACKS.is_empty());
        for track in SAMPLE_TRACKS {
            assert!(!track.title.is_empty());
            assert!(!track.artist.is_empty());
            assert!(!track.lyrics.is_empty());
            assert!(track.lyrics.len() >= 5);
        }
    }

    #[test]
    fn test_sample_track_cycling() {
        let count = SAMPLE_TRACKS.len();
        let idx = 4usize;
        let clamped = idx % count;
        assert!(clamped < count);
    }
}
