//! Standalone Secondary Window: Lyrics Live Animation & Style Preview.
//!
//! Provides a dedicated real-time preview window allowing users to immediately inspect
//! and verify their lyrics animations, layout placements, text alignments, glowing shadows,
//! custom CSS keyframe scripts, and TTML / ELRC syllable word-by-word timing without having
//! to start playback or toggle full screen.
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

/// Syllable / Timed-word definition within a lyric line for TTML / ELRC simulation.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewTimedWord {
    pub text: &'static str,
    pub start_ms: u64,
    pub duration_ms: u64,
}

/// Single lyric line containing text and syllable word timing definitions.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewLine {
    pub text: &'static str,
    pub duration_ms: u64,
    pub words: &'static [PreviewTimedWord],
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
    pub lines: &'static [PreviewLine],
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
        lines: &[
            PreviewLine {
                text: "DECO",
                duration_ms: 1200,
                words: &[PreviewTimedWord { text: "DECO", start_ms: 0, duration_ms: 1100 }],
            },
            PreviewLine {
                text: "PINO",
                duration_ms: 1200,
                words: &[PreviewTimedWord { text: "PINO", start_ms: 0, duration_ms: 1100 }],
            },
            PreviewLine {
                text: "堕",
                duration_ms: 1400,
                words: &[PreviewTimedWord { text: "堕", start_ms: 0, duration_ms: 1300 }],
            },
            PreviewLine {
                text: "ながらの優等生 今宵も自分と密会です",
                duration_ms: 2800,
                words: &[
                    PreviewTimedWord { text: "ながらの", start_ms: 0, duration_ms: 450 },
                    PreviewTimedWord { text: "優等生 ", start_ms: 450, duration_ms: 600 },
                    PreviewTimedWord { text: "今宵も", start_ms: 1050, duration_ms: 450 },
                    PreviewTimedWord { text: "自分と", start_ms: 1500, duration_ms: 500 },
                    PreviewTimedWord { text: "密会です", start_ms: 2000, duration_ms: 700 },
                ],
            },
            PreviewLine {
                text: "しんどいわ しんどいわ あーつらつら合掌",
                duration_ms: 2800,
                words: &[
                    PreviewTimedWord { text: "しんどいわ ", start_ms: 0, duration_ms: 700 },
                    PreviewTimedWord { text: "しんどいわ ", start_ms: 700, duration_ms: 700 },
                    PreviewTimedWord { text: "あー", start_ms: 1400, duration_ms: 400 },
                    PreviewTimedWord { text: "つらつら", start_ms: 1800, duration_ms: 450 },
                    PreviewTimedWord { text: "合掌", start_ms: 2250, duration_ms: 500 },
                ],
            },
            PreviewLine {
                text: "ほんとは劣等生 あたしはあたしに説教です",
                duration_ms: 2800,
                words: &[
                    PreviewTimedWord { text: "ほんとは", start_ms: 0, duration_ms: 450 },
                    PreviewTimedWord { text: "劣等生 ", start_ms: 450, duration_ms: 600 },
                    PreviewTimedWord { text: "あたしは", start_ms: 1050, duration_ms: 450 },
                    PreviewTimedWord { text: "あたしに", start_ms: 1500, duration_ms: 450 },
                    PreviewTimedWord { text: "説教です", start_ms: 1950, duration_ms: 750 },
                ],
            },
            PreviewLine {
                text: "いいから はやく ハンコ押せや",
                duration_ms: 2600,
                words: &[
                    PreviewTimedWord { text: "いいから ", start_ms: 0, duration_ms: 500 },
                    PreviewTimedWord { text: "はやく ", start_ms: 500, duration_ms: 600 },
                    PreviewTimedWord { text: "ハンコ", start_ms: 1100, duration_ms: 650 },
                    PreviewTimedWord { text: "押せや", start_ms: 1750, duration_ms: 800 },
                ],
            },
            PreviewLine {
                text: "デビルじゃない デビルじゃないもん 悪魔ほどそう言うもん",
                duration_ms: 3000,
                words: &[
                    PreviewTimedWord { text: "デビルじゃない ", start_ms: 0, duration_ms: 700 },
                    PreviewTimedWord { text: "デビルじゃないもん ", start_ms: 700, duration_ms: 900 },
                    PreviewTimedWord { text: "悪魔ほど", start_ms: 1600, duration_ms: 600 },
                    PreviewTimedWord { text: "そう言うもん", start_ms: 2200, duration_ms: 750 },
                ],
            },
            PreviewLine {
                text: "あっけなく 散った 夢見た未来",
                duration_ms: 2600,
                words: &[
                    PreviewTimedWord { text: "あっけなく ", start_ms: 0, duration_ms: 700 },
                    PreviewTimedWord { text: "散った ", start_ms: 700, duration_ms: 600 },
                    PreviewTimedWord { text: "夢見た", start_ms: 1300, duration_ms: 600 },
                    PreviewTimedWord { text: "未来", start_ms: 1900, duration_ms: 650 },
                ],
            },
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
        lines: &[
            PreviewLine {
                text: "Step onto the floor, the frequency is clear",
                duration_ms: 2700,
                words: &[
                    PreviewTimedWord { text: "Step ", start_ms: 0, duration_ms: 300 },
                    PreviewTimedWord { text: "onto ", start_ms: 300, duration_ms: 350 },
                    PreviewTimedWord { text: "the ", start_ms: 650, duration_ms: 200 },
                    PreviewTimedWord { text: "floor, ", start_ms: 850, duration_ms: 450 },
                    PreviewTimedWord { text: "the ", start_ms: 1300, duration_ms: 200 },
                    PreviewTimedWord { text: "frequency ", start_ms: 1500, duration_ms: 500 },
                    PreviewTimedWord { text: "is ", start_ms: 2000, duration_ms: 200 },
                    PreviewTimedWord { text: "clear", start_ms: 2200, duration_ms: 450 },
                ],
            },
            PreviewLine {
                text: "Feel the sub-bass rising in the atmosphere",
                duration_ms: 2800,
                words: &[
                    PreviewTimedWord { text: "Feel ", start_ms: 0, duration_ms: 300 },
                    PreviewTimedWord { text: "the ", start_ms: 300, duration_ms: 200 },
                    PreviewTimedWord { text: "sub-bass ", start_ms: 500, duration_ms: 500 },
                    PreviewTimedWord { text: "rising ", start_ms: 1000, duration_ms: 450 },
                    PreviewTimedWord { text: "in ", start_ms: 1450, duration_ms: 200 },
                    PreviewTimedWord { text: "the ", start_ms: 1650, duration_ms: 200 },
                    PreviewTimedWord { text: "atmosphere", start_ms: 1850, duration_ms: 850 },
                ],
            },
            PreviewLine {
                text: "Lock the tempo, harmonize the keys",
                duration_ms: 2500,
                words: &[
                    PreviewTimedWord { text: "Lock ", start_ms: 0, duration_ms: 300 },
                    PreviewTimedWord { text: "the ", start_ms: 300, duration_ms: 200 },
                    PreviewTimedWord { text: "tempo, ", start_ms: 500, duration_ms: 450 },
                    PreviewTimedWord { text: "harmonize ", start_ms: 950, duration_ms: 600 },
                    PreviewTimedWord { text: "the ", start_ms: 1550, duration_ms: 200 },
                    PreviewTimedWord { text: "keys", start_ms: 1750, duration_ms: 650 },
                ],
            },
            PreviewLine {
                text: "Attune your soul to the digital breeze",
                duration_ms: 2600,
                words: &[
                    PreviewTimedWord { text: "Attune ", start_ms: 0, duration_ms: 450 },
                    PreviewTimedWord { text: "your ", start_ms: 450, duration_ms: 250 },
                    PreviewTimedWord { text: "soul ", start_ms: 700, duration_ms: 400 },
                    PreviewTimedWord { text: "to ", start_ms: 1100, duration_ms: 200 },
                    PreviewTimedWord { text: "the ", start_ms: 1300, duration_ms: 200 },
                    PreviewTimedWord { text: "digital ", start_ms: 1500, duration_ms: 450 },
                    PreviewTimedWord { text: "breeze", start_ms: 1950, duration_ms: 600 },
                ],
            },
            PreviewLine {
                text: "Transition seamless, never skip a beat",
                duration_ms: 2700,
                words: &[
                    PreviewTimedWord { text: "Transition ", start_ms: 0, duration_ms: 550 },
                    PreviewTimedWord { text: "seamless, ", start_ms: 550, duration_ms: 550 },
                    PreviewTimedWord { text: "never ", start_ms: 1100, duration_ms: 350 },
                    PreviewTimedWord { text: "skip ", start_ms: 1450, duration_ms: 300 },
                    PreviewTimedWord { text: "a ", start_ms: 1750, duration_ms: 200 },
                    PreviewTimedWord { text: "beat", start_ms: 1950, duration_ms: 650 },
                ],
            },
            PreviewLine {
                text: "From peak time energy to twilight retreat",
                duration_ms: 2800,
                words: &[
                    PreviewTimedWord { text: "From ", start_ms: 0, duration_ms: 250 },
                    PreviewTimedWord { text: "peak ", start_ms: 250, duration_ms: 350 },
                    PreviewTimedWord { text: "time ", start_ms: 600, duration_ms: 350 },
                    PreviewTimedWord { text: "energy ", start_ms: 950, duration_ms: 500 },
                    PreviewTimedWord { text: "to ", start_ms: 1450, duration_ms: 200 },
                    PreviewTimedWord { text: "twilight ", start_ms: 1650, duration_ms: 500 },
                    PreviewTimedWord { text: "retreat", start_ms: 2150, duration_ms: 600 },
                ],
            },
            PreviewLine {
                text: "Pure resonance, dynamic flow",
                duration_ms: 2400,
                words: &[
                    PreviewTimedWord { text: "Pure ", start_ms: 0, duration_ms: 350 },
                    PreviewTimedWord { text: "resonance, ", start_ms: 350, duration_ms: 650 },
                    PreviewTimedWord { text: "dynamic ", start_ms: 1000, duration_ms: 500 },
                    PreviewTimedWord { text: "flow", start_ms: 1500, duration_ms: 800 },
                ],
            },
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
        lines: &[
            PreviewLine {
                text: "僕のことはさて置き",
                duration_ms: 2200,
                words: &[
                    PreviewTimedWord { text: "僕の", start_ms: 0, duration_ms: 450 },
                    PreviewTimedWord { text: "ことは", start_ms: 450, duration_ms: 500 },
                    PreviewTimedWord { text: "さて置き", start_ms: 950, duration_ms: 1100 },
                ],
            },
            PreviewLine {
                text: "君のことが聞きたいんだ",
                duration_ms: 2400,
                words: &[
                    PreviewTimedWord { text: "君の", start_ms: 0, duration_ms: 450 },
                    PreviewTimedWord { text: "ことが", start_ms: 450, duration_ms: 550 },
                    PreviewTimedWord { text: "聞きたいんだ", start_ms: 1000, duration_ms: 1250 },
                ],
            },
            PreviewLine {
                text: "色々あるんだホント",
                duration_ms: 2300,
                words: &[
                    PreviewTimedWord { text: "色々", start_ms: 0, duration_ms: 550 },
                    PreviewTimedWord { text: "あるんだ", start_ms: 550, duration_ms: 600 },
                    PreviewTimedWord { text: "ホント", start_ms: 1150, duration_ms: 1050 },
                ],
            },
            PreviewLine {
                text: "それじゃあ今日はよろしくどうぞ",
                duration_ms: 2700,
                words: &[
                    PreviewTimedWord { text: "それじゃあ", start_ms: 0, duration_ms: 600 },
                    PreviewTimedWord { text: "今日は", start_ms: 600, duration_ms: 550 },
                    PreviewTimedWord { text: "よろしく", start_ms: 1150, duration_ms: 650 },
                    PreviewTimedWord { text: "どうぞ", start_ms: 1800, duration_ms: 800 },
                ],
            },
            PreviewLine {
                text: "いつも輝いてる 君の色はなんだったっけ？",
                duration_ms: 3000,
                words: &[
                    PreviewTimedWord { text: "いつも", start_ms: 0, duration_ms: 500 },
                    PreviewTimedWord { text: "輝いてる ", start_ms: 500, duration_ms: 700 },
                    PreviewTimedWord { text: "君の", start_ms: 1200, duration_ms: 400 },
                    PreviewTimedWord { text: "色は", start_ms: 1600, duration_ms: 450 },
                    PreviewTimedWord { text: "なんだったっけ？", start_ms: 2050, duration_ms: 900 },
                ],
            },
            PreviewLine {
                text: "たくさんあったはずなのに",
                duration_ms: 2500,
                words: &[
                    PreviewTimedWord { text: "たくさん", start_ms: 0, duration_ms: 600 },
                    PreviewTimedWord { text: "あった", start_ms: 600, duration_ms: 450 },
                    PreviewTimedWord { text: "はずなのに", start_ms: 1050, duration_ms: 1300 },
                ],
            },
            PreviewLine {
                text: "今ではもう覚えてないんだ",
                duration_ms: 2600,
                words: &[
                    PreviewTimedWord { text: "今では", start_ms: 0, duration_ms: 550 },
                    PreviewTimedWord { text: "もう", start_ms: 550, duration_ms: 400 },
                    PreviewTimedWord { text: "覚えて", start_ms: 950, duration_ms: 600 },
                    PreviewTimedWord { text: "ないんだ", start_ms: 1550, duration_ms: 950 },
                ],
            },
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
            .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1140.0, 740.0))
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
                "<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n\
                <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\n\
                <link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=Roboto:wght@400;500;700&family=Sora:wght@400;600;700;800&display=swap\">\n\
                <link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/icon?family=Material+Icons\">\n\
                <style>\n{icons_css}\n{css}\n</style>"
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
    let mut line_elapsed_ms = use_signal(|| 0u64);
    let mut is_playing = use_signal(|| true);
    let mut sim_speed = use_signal(|| 1.0f32);
    let mut preview_mode = use_signal(|| "fullscreen".to_string()); // "fullscreen" | "sidebar"
    let mut sync_format = use_signal(|| "word".to_string()); // "word" | "line"

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

    // High-resolution 40ms simulation loop tracking millisecond line elapsed progress
    // and advancing word-by-word (TTML/ELRC) and line-by-line automatically.
    use_future(move || async move {
        loop {
            let speed = *sim_speed.read();
            let tick_ms = 40u64;
            tokio::time::sleep(tokio::time::Duration::from_millis(tick_ms)).await;

            if *is_playing.read() {
                let track = &SAMPLE_TRACKS[*selected_track_idx.read() % SAMPLE_TRACKS.len()];
                let total = track.lines.len();
                if total > 0 {
                    let curr_idx = *active_line_idx.read() % total;
                    let target_dur = track.lines[curr_idx].duration_ms;
                    let scaled_increment = (tick_ms as f32 * speed.max(0.1)) as u64;
                    let next_elapsed = *line_elapsed_ms.read() + scaled_increment;

                    if next_elapsed >= target_dur {
                        line_elapsed_ms.set(0);
                        active_line_idx.set((curr_idx + 1) % total);
                    } else {
                        line_elapsed_ms.set(next_elapsed);
                    }
                }
            }
        }
    });

    let curr_cfg = config_sig.read().clone();
    let preview_font_css = crate::system_fonts::resolve_font_family_css(&curr_cfg.lyric_font_family);
    let strings = crate::i18n::get_strings(crate::i18n::Language::from_code(&curr_cfg.language));
    let track = &SAMPLE_TRACKS[*selected_track_idx.read() % SAMPLE_TRACKS.len()];
    let total_lines = track.lines.len();
    let curr_line_idx = *active_line_idx.read() % total_lines.max(1);
    let curr_line = &track.lines[curr_line_idx];
    let curr_elapsed = *line_elapsed_ms.read();

    // Identify active word for progress diagnostics
    let (active_word_num, active_word_text) = {
        let mut num = 1;
        let mut txt = "";
        for (i, w) in curr_line.words.iter().enumerate() {
            if curr_elapsed >= w.start_ms {
                num = i + 1;
                txt = w.text.trim();
            }
        }
        (num, txt)
    };

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
                style: "height: 64px; background: rgba(10, 14, 23, 0.90); backdrop-filter: blur(16px); border-bottom: 1px solid var(--border-glass); display: flex; justify-content: space-between; align-items: center; padding: 0 20px; z-index: 100; flex-shrink: 0;",

                // Left: Title + Realtime Status Indicator
                div { style: "display: flex; align-items: center; gap: 12px;",
                    div { class: "live-pulse-dot", style: "width: 9px; height: 9px; border-radius: 50%; background: var(--accent-mint); box-shadow: 0 0 10px var(--accent-mint);" }
                    div {
                        h1 { style: "font-size: 14.5px; font-weight: 700; margin: 0; line-height: 1.2;", "{strings.preview_lyrics_window_title}" }
                        p { style: "font-size: 11px; color: var(--text-muted); margin: 0;", "{strings.preview_notice_realtime}" }
                    }
                }

                // Center: Viewport Mode Switcher & Sync Format Toggle & Track Selector
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

                    // Format Toggle: Line Sync (LRC) vs Syllable / Word Sync (TTML / ELRC)
                    div { style: "display: flex; background: rgba(255,255,255,0.06); padding: 3px; border-radius: 20px; border: 1px solid var(--border-glass); gap: 2px;",
                        button {
                            class: if *sync_format.read() == "word" { "btn-pill active" } else { "btn-pill" },
                            style: "font-size: 11px; padding: 4px 10px; border: none;",
                            title: "Syllable / Word-by-word synchronization simulation (TTML / ELRC)",
                            onclick: move |_| sync_format.set("word".to_string()),
                            span { class: "material-icons md-14", "timer" }
                            "{strings.preview_format_word_sync}"
                        }
                        button {
                            class: if *sync_format.read() == "line" { "btn-pill active" } else { "btn-pill" },
                            style: "font-size: 11px; padding: 4px 10px; border: none;",
                            title: "Standard whole-line synchronization (LRC)",
                            onclick: move |_| sync_format.set("line".to_string()),
                            span { class: "material-icons md-14", "format_align_left" }
                            "{strings.preview_format_line_sync}"
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
                                line_elapsed_ms.set(0);
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
                            line_elapsed_ms.set(0);
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
                            line_elapsed_ms.set(0);
                        },
                        span { class: "material-icons md-18", "skip_next" }
                    }

                    // Reset / Restart Line 0
                    button {
                        class: "btn-icon",
                        style: "width: 32px; height: 32px; color: var(--text-muted);",
                        title: "Restart Simulation from Line 1",
                        onclick: move |_| {
                            active_line_idx.set(0);
                            line_elapsed_ms.set(0);
                        },
                        span { class: "material-icons md-18", "restart_alt" }
                    }

                    // Line Counter Pill
                    span {
                        style: "font-family: monospace; font-size: 11px; background: rgba(255,255,255,0.06); padding: 4px 8px; border-radius: 6px; border: 1px solid var(--border-glass); color: var(--accent-mint);",
                        "{strings.preview_simulating_line} {curr_line_idx + 1}/{total_lines}"
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

                    // In-Preview Text Resizing Stepper (A- / A+)
                    {
                        let is_fs = *preview_mode.read() == "fullscreen";
                        let curr_sz = if is_fs { curr_cfg.fullscreen_lyrics_font_size } else { curr_cfg.dock_lyrics_font_size };
                        let min_sz = if is_fs { 18 } else { 12 };
                        let max_sz = if is_fs { 56 } else { 32 };
                        let step = 2;

                        rsx! {
                            div {
                                class: "lyrics-size-pill",
                                title: if is_fs { "{strings.fullscreen_lyrics_size}: {curr_sz}px" } else { "{strings.sidebar_lyrics_size}: {curr_sz}px" },
                                button {
                                    class: "lyrics-size-btn",
                                    title: "{strings.decrease_lyrics_size}",
                                    disabled: curr_sz <= min_sz,
                                    onclick: {
                                        let mut new_cfg = curr_cfg.clone();
                                        let mut config_sig = config_sig.clone();
                                        move |_| {
                                            if is_fs {
                                                new_cfg.fullscreen_lyrics_font_size = (new_cfg.fullscreen_lyrics_font_size.saturating_sub(step)).max(min_sz);
                                            } else {
                                                new_cfg.dock_lyrics_font_size = (new_cfg.dock_lyrics_font_size.saturating_sub(step)).max(min_sz);
                                            }
                                            let _ = crate::config::save_config(&new_cfg);
                                            config_sig.set(new_cfg.clone());
                                        }
                                    },
                                    "A-"
                                }
                                span {
                                    class: "lyrics-size-val",
                                    "{curr_sz}px"
                                }
                                button {
                                    class: "lyrics-size-btn",
                                    title: "{strings.increase_lyrics_size}",
                                    disabled: curr_sz >= max_sz,
                                    onclick: {
                                        let mut new_cfg = curr_cfg.clone();
                                        let mut config_sig = config_sig.clone();
                                        move |_| {
                                            if is_fs {
                                                new_cfg.fullscreen_lyrics_font_size = (new_cfg.fullscreen_lyrics_font_size.saturating_add(step)).min(max_sz);
                                            } else {
                                                new_cfg.dock_lyrics_font_size = (new_cfg.dock_lyrics_font_size.saturating_add(step)).min(max_sz);
                                            }
                                            let _ = crate::config::save_config(&new_cfg);
                                            config_sig.set(new_cfg.clone());
                                        }
                                    },
                                    "A+"
                                }
                            }
                        }
                    }
                }
            }

            // Secondary Status Information Subheader & Syllable Progress Bar
            div {
                style: "height: 36px; background: rgba(0,0,0,0.45); border-bottom: 1px solid rgba(255,255,255,0.06); display: flex; align-items: center; justify-content: space-between; padding: 0 20px; font-size: 11.5px; z-index: 90; flex-shrink: 0;",
                div { style: "display: flex; gap: 14px; align-items: center;",
                    span { style: "color: var(--text-muted);",
                        "Font: "
                        strong { style: "color: var(--accent-mint);",
                            "{curr_cfg.lyric_font_family}"
                        }
                    }
                    span { style: "color: var(--text-muted);",
                        "Size: "
                        strong { style: "color: var(--accent-mint);",
                            if *preview_mode.read() == "fullscreen" {
                                "{curr_cfg.fullscreen_lyrics_font_size}px (FS)"
                            } else {
                                "{curr_cfg.dock_lyrics_font_size}px (Dock)"
                            }
                        }
                    }
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
                        "Engine: "
                        strong { style: "color: var(--accent-mint);",
                            if *sync_format.read() == "word" { "TTML / ELRC Syllable Spans" } else { "Standard LRC Lines" }
                        }
                    }
                }

                // Word progress indicator and mini progress bar
                div { style: "display: flex; align-items: center; gap: 12px;",
                    if *sync_format.read() == "word" {
                        div { style: "display: flex; align-items: center; gap: 8px;",
                            span { style: "font-family: monospace; font-size: 11px; color: var(--text-secondary);",
                                "{strings.preview_syllable_progress}: "
                                span { style: "color: #ffffff; font-weight: 700;", "{active_word_num}/{curr_line.words.len()}" }
                                if !active_word_text.is_empty() {
                                    span { style: "color: var(--accent-mint); margin-left: 4px; font-weight: 600;", "('{active_word_text}')" }
                                }
                            }
                            // Mini line syllable timeline bar
                            div { style: "width: 80px; height: 5px; background: rgba(255,255,255,0.12); border-radius: 3px; overflow: hidden;",
                                div {
                                    style: format!(
                                        "height: 100%; width: {:.1}%; background: var(--accent-mint); border-radius: 3px; transition: width 0.05s linear;",
                                        ((curr_elapsed as f32) / (curr_line.duration_ms.max(1) as f32) * 100.0).clamp(0.0, 100.0)
                                    )
                                }
                            }
                        }
                    }

                    if curr_cfg.enable_lyrics_custom_css {
                        span {
                            style: "font-size: 10.5px; padding: 2px 8px; border-radius: 4px; background: rgba(255, 170, 0, 0.15); color: #ffa726; border: 1px solid rgba(255, 170, 0, 0.35); font-weight: 600;",
                            "Custom CSS Active"
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
                                            ((curr_line_idx as f32 + (curr_elapsed as f32 / curr_line.duration_ms.max(1) as f32)) / total_lines as f32) * 100.0
                                        )
                                    }
                                }
                                div { style: "display: flex; justify-content: space-between; font-size: 11px; font-family: monospace; color: var(--text-muted);",
                                    span { "0:{curr_line_idx * 15 + 10:02}" }
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
                                style: "overflow-y: auto; max-height: 80vh; padding: 25vh 24px; display: flex; flex-direction: column; gap: 24px; scroll-behavior: smooth; mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%); -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%); --lyrics-font-family: {preview_font_css}; --lyrics-fullscreen-font-size: {curr_cfg.fullscreen_lyrics_font_size}px; font-family: var(--lyrics-font-family);",

                                for (idx, line_item) in track.lines.iter().enumerate() {
                                    {
                                        let is_active = idx == curr_line_idx;
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

                                        let is_word_mode = *sync_format.read() == "word";

                                        rsx! {
                                            div {
                                                key: "{idx}",
                                                "data-line-idx": "{idx}",
                                                class: "{line_class}",
                                                style: "cursor: pointer;",
                                                title: "Click to jump simulation to this line",
                                                onclick: move |_| {
                                                    active_line_idx.set(idx);
                                                    line_elapsed_ms.set(0);
                                                },

                                                if is_word_mode {
                                                    // Syllable / Word-by-Word Synchronized Rendering (TTML / ELRC)
                                                    for (w_idx, w) in line_item.words.iter().enumerate() {
                                                        {
                                                            let word_state = if !is_active {
                                                                "word-inactive"
                                                            } else {
                                                                if curr_elapsed < w.start_ms {
                                                                    "word-upcoming"
                                                                } else if curr_elapsed < w.start_ms + w.duration_ms {
                                                                    "word-active"
                                                                } else {
                                                                    "word-sung"
                                                                }
                                                            };

                                                            rsx! {
                                                                span {
                                                                    key: "{w_idx}",
                                                                    class: "fullscreen-lyric-word {word_state}",
                                                                    title: format!("{}: {}ms - {}ms", w.text.trim(), w.start_ms, w.start_ms + w.duration_ms),
                                                                    onclick: {
                                                                        let start = w.start_ms;
                                                                        move |e: Event<MouseData>| {
                                                                            e.stop_propagation();
                                                                            active_line_idx.set(idx);
                                                                            line_elapsed_ms.set(start + 10);
                                                                        }
                                                                    },
                                                                    "{w.text}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    // Standard whole-line text rendering
                                                    "{line_item.text}"
                                                }
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
                                div { style: "display: flex; gap: 6px; align-items: center;",
                                    if *sync_format.read() == "word" {
                                        span { class: "badge-pill quality-good", style: "font-size: 10px;", "TTML/ELRC" }
                                    }
                                    span { class: "badge-pill quality-good", style: "font-size: 10px;", "Auto-Scroll" }
                                }
                            }

                            // Sidebar Scrolling Lines
                            div {
                                style: "flex: 1; overflow-y: auto; padding: 30px 20px; display: flex; flex-direction: column; gap: 18px; --lyrics-font-family: {preview_font_css}; --lyrics-font-size: {curr_cfg.dock_lyrics_font_size}px; font-family: var(--lyrics-font-family);",

                                for (idx, line_item) in track.lines.iter().enumerate() {
                                    {
                                        let is_active = idx == curr_line_idx;
                                        let align_class = match curr_cfg.dock_lyrics_align.as_str() {
                                            "center" => "align-center",
                                            "right" => "align-right",
                                            _ => "align-left",
                                        };

                                        let active_sz = curr_cfg.dock_lyrics_font_size;
                                        let inactive_sz = ((active_sz as f32) * 0.84).round() as u32;

                                        let line_style = if is_active {
                                            if curr_cfg.sidebar_text_glow {
                                                format!("font-family: var(--lyrics-font-family); font-size: {active_sz}px; font-weight: 700; color: #ffffff; text-shadow: 0 0 16px rgba(255,255,255,0.7); opacity: 1; cursor: pointer; transition: all 0.25s ease;")
                                            } else {
                                                format!("font-family: var(--lyrics-font-family); font-size: {active_sz}px; font-weight: 700; color: #ffffff; opacity: 1; cursor: pointer; transition: all 0.25s ease;")
                                            }
                                        } else {
                                            format!("font-family: var(--lyrics-font-family); font-size: {inactive_sz}px; font-weight: 500; color: rgba(255,255,255,0.40); cursor: pointer; transition: all 0.25s ease;")
                                        };

                                        let is_word_mode = *sync_format.read() == "word";

                                        rsx! {
                                            div {
                                                key: "{idx}",
                                                class: "lyric-line {align_class}",
                                                style: "{line_style}",
                                                onclick: move |_| {
                                                    active_line_idx.set(idx);
                                                    line_elapsed_ms.set(0);
                                                },

                                                if is_word_mode && is_active {
                                                    for (w_idx, w) in line_item.words.iter().enumerate() {
                                                        {
                                                            let word_state = if curr_elapsed < w.start_ms {
                                                                "word-upcoming"
                                                            } else if curr_elapsed < w.start_ms + w.duration_ms {
                                                                "word-active"
                                                            } else {
                                                                "word-sung"
                                                            };
                                                            rsx! {
                                                                span {
                                                                    key: "{w_idx}",
                                                                    class: "lyric-word {word_state}",
                                                                    "{w.text}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    "{line_item.text}"
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
            assert!(!track.lines.is_empty());
            assert!(track.lines.len() >= 5);
            for line in track.lines {
                assert!(!line.text.is_empty());
                assert!(line.duration_ms > 0);
                assert!(!line.words.is_empty());
                for w in line.words {
                    assert!(!w.text.is_empty());
                    assert!(w.duration_ms > 0);
                }
            }
        }
    }

    #[test]
    fn test_sample_track_cycling() {
        let count = SAMPLE_TRACKS.len();
        let idx = 4usize;
        let clamped = idx % count;
        assert!(clamped < count);
    }

    #[test]
    fn test_ttml_elrc_timed_word_intervals() {
        let track = &SAMPLE_TRACKS[0];
        let line = &track.lines[3];
        assert_eq!(line.words.len(), 5);
        assert_eq!(line.words[0].start_ms, 0);
        assert_eq!(line.words[1].start_ms, 450);
        assert!(line.words[4].start_ms + line.words[4].duration_ms <= line.duration_ms);
    }
}
