use dioxus::prelude::*;
use std::path::PathBuf;
use crate::library::clear_thumbnail_cache;
use crate::process_manager::{sample_metrics, trim_memory};
use crate::smart_shuffle::ShuffleMode;

#[component]
pub fn Settings(
    music_dir: Option<PathBuf>,
    music_dir_source: String,
    has_hardware_output: bool,
    driver_error: Option<String>,
    mixer_sample_rate: u32,
    shuffle_mode: ShuffleMode,
    on_change_shuffle_mode: EventHandler<ShuffleMode>,
    on_choose_folder: EventHandler<()>,
    on_rescan_library: EventHandler<()>,
    on_open_equalizer: EventHandler<()>,
    on_open_process_manager: EventHandler<()>,
    on_open_setup: EventHandler<()>,
) -> Element {
    let mut username = use_signal(|| {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "Listener".to_string())
    });
    let mut camelot_lock = use_signal(|| true);
    let mut trim_feedback = use_signal(|| None::<(bool, String)>);
    let mut metrics_res = use_signal(sample_metrics);

    let curr_metrics = metrics_res.read().clone();
    let dir_display = music_dir
        .as_ref()
        .map(|d| d.display().to_string())
        .unwrap_or_else(|| "No directory selected".to_string());

    let output_driver_label = if has_hardware_output {
        if cfg!(target_os = "windows") {
            "WASAPI Hardware Stream (cpal)".to_string()
        } else {
            "ALSA / PipeWire Hardware Stream (cpal)".to_string()
        }
    } else if let Some(ref err) = driver_error {
        format!("Headless Fallback ({err})")
    } else {
        "Headless (No Hardware Audio Output)".to_string()
    };

    let sample_rate_label = if mixer_sample_rate > 0 {
        format!("{mixer_sample_rate} Hz • 32-bit Float")
    } else {
        "Unprobed".to_string()
    };

    rsx! {
        div { class: "settings-stage",
            style: "display: flex; flex-direction: column; gap: 24px; max-width: 860px; margin: 0 auto; width: 100%; padding-bottom: 50px;",

            // Header
            div { style: "border-bottom: 1px solid var(--border-glass); padding-bottom: 16px;",
                h1 { style: "font-size: 26px; font-weight: 700;", "Settings & Preferences" }
                p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                    "Attune Desktop Configuration, Audio Output, Library Management & System Diagnostics"
                }
            }

            // Section 1: General & Profile
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                h3 { style: "font-size: 16px; font-weight: 600;", "General & Listener Profile" }

                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    label { style: "font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;", "Listener Display Name" }
                    input {
                        class: "search-bar",
                        style: "width: 100%; max-width: 400px; font-size: 14px; padding: 10px 14px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                        value: "{username}",
                        oninput: move |e| username.set(e.value()),
                    }
                    span { style: "font-size: 11.5px; color: var(--text-muted);", "Resolved from OS environment ($USER / %USERNAME%) for personalized greetings." }
                }

                div { style: "display: flex; justify-content: space-between; align-items: center; padding-top: 10px; border-top: 1px solid var(--border-glass);",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Interface Aesthetic" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "Hardware-accelerated CSS3 liquid glass with backdrop blur" }
                    }
                    span { class: "btn-pill active", style: "font-size: 12px;", "Liquid Material Glass (Active)" }
                }
            }

            // Section 2: Library & Storage
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                h3 { style: "font-size: 16px; font-weight: 600;", "Music Library & Storage" }

                div { style: "display: flex; flex-direction: column; gap: 8px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        span { style: "font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;", "Primary Music Directory" }
                        span { style: "font-size: 11px; color: var(--text-secondary); font-family: monospace;", "Source: {music_dir_source}" }
                    }
                    div { style: "display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: rgba(255,255,255,0.03); border-radius: 8px; border: 1px solid var(--border-glass); font-family: monospace; font-size: 13px; color: var(--accent-mint); word-break: break-all;",
                        span { class: "material-icons md-16", "folder" }
                        "{dir_display}"
                    }
                    div { style: "display: flex; gap: 10px; margin-top: 6px; flex-wrap: wrap;",
                        button {
                            class: "btn-pill active",
                            style: "display: inline-flex; align-items: center; gap: 6px;",
                            onclick: move |_| on_choose_folder.call(()),
                            span { class: "material-icons md-16", "folder_open" }
                            "Choose Music Folder..."
                        }
                        button {
                            class: "btn-pill",
                            style: "display: inline-flex; align-items: center; gap: 6px;",
                            onclick: move |_| on_rescan_library.call(()),
                            span { class: "material-icons md-16", "sync" }
                            "Rescan Library"
                        }
                        button {
                            class: "btn-pill",
                            style: "display: inline-flex; align-items: center; gap: 6px; border: 1px solid rgba(45,212,191,0.4); background: rgba(45,212,191,0.08); color: var(--accent-mint); font-weight: 600;",
                            onclick: move |_| on_open_setup.call(()),
                            span { class: "material-icons md-16", "auto_awesome" }
                            "Re-run Setup & Audio Profiling Wizard"
                        }
                    }
                }

                div { style: "display: flex; justify-content: space-between; align-items: center; padding-top: 12px; border-top: 1px solid var(--border-glass);",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Artwork Thumbnail Cache" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "Stores downsampled 160x160 JPEG thumbnails to keep memory footprint under 150 MB" }
                    }
                    button {
                        class: "btn-pill",
                        style: "font-size: 12px; display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| {
                            clear_thumbnail_cache();
                            trim_feedback.set(Some((true, "✓ Thumbnail cache cleared!".to_string())));
                        },
                        span { class: "material-icons md-16", "delete_sweep" }
                        "Clear Thumbnails"
                    }
                }
            }

            // Section 3: Audio Output & DSP
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                h3 { style: "font-size: 16px; font-weight: 600;", "Audio Engine & Studio Master FX" }

                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 14px;",
                    div { style: "background: rgba(255,255,255,0.03); padding: 12px; border-radius: 10px; border: 1px solid var(--border-glass);",
                        span { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; display: block; margin-bottom: 4px;", "Output Driver" }
                        span {
                            style: if has_hardware_output { "font-weight: 600; font-size: 13.5px; color: var(--accent-mint); font-family: monospace;" } else { "font-weight: 600; font-size: 13.5px; color: #fb7185; font-family: monospace;" },
                            "{output_driver_label}"
                        }
                    }
                    div { style: "background: rgba(255,255,255,0.03); padding: 12px; border-radius: 10px; border: 1px solid var(--border-glass);",
                        span { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; display: block; margin-bottom: 4px;", "Mixer Sample Rate" }
                        span { style: "font-weight: 600; font-size: 13.5px; color: var(--accent-mint); font-family: monospace;", "{sample_rate_label}" }
                    }
                }

                div { style: "display: flex; justify-content: space-between; align-items: center; padding-top: 10px; border-top: 1px solid var(--border-glass);",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Master Equalizer & FX Rack" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "10-band IIR graphic EQ, Pure Bass+, Clarity exciter, Stereo Width, and Limiter" }
                    }
                    button {
                        class: "btn-pill active",
                        style: "display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| on_open_equalizer.call(()),
                        span { class: "material-icons md-16", "tune" }
                        "Open Equalizer Rack"
                    }
                }
            }

            // Section 4: Transitions & Smart Shuffle
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                h3 { style: "font-size: 16px; font-weight: 600;", "Smart DJ Transitions & Mixer Logic" }

                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Dynamic Smart DJ Shuffle" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "Calculates Camelot harmonic key affinity, tempo alignment, and recency penalties" }
                    }
                    button {
                        class: if shuffle_mode == ShuffleMode::Smart { "btn-pill active" } else { "btn-pill" },
                        onclick: move |_| {
                            let next = if shuffle_mode == ShuffleMode::Smart {
                                ShuffleMode::Off
                            } else {
                                ShuffleMode::Smart
                            };
                            on_change_shuffle_mode.call(next);
                        },
                        if shuffle_mode == ShuffleMode::Smart { "ENABLED (SMART DJ)" } else if shuffle_mode == ShuffleMode::Linear { "LINEAR SHUFFLE" } else { "DISABLED (SEQUENTIAL)" }
                    }
                }

                div { style: "display: flex; justify-content: space-between; align-items: center; padding-top: 12px; border-top: 1px solid var(--border-glass);",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Camelot Circle Harmonic Key Locking" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "Prioritizes adjacent keys (e.g. 8A → 8B, 9A) to prevent dissonance during blending" }
                    }
                    button {
                        class: if *camelot_lock.read() { "btn-pill active" } else { "btn-pill" },
                        onclick: move |_| {
                            let curr = *camelot_lock.read();
                            camelot_lock.set(!curr);
                        },
                        if *camelot_lock.read() { "LOCKED" } else { "UNLOCKED" }
                    }
                }
            }

            // Section 5: Hardware & Process Resource Monitor
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    div {
                        h3 { style: "font-size: 16px; font-weight: 600;", "Process & Hardware Diagnostics" }
                        p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;", "Inspect kernel CPU load, GPU clock, memory RSS, and audio IRQs via libattune_procmgr" }
                    }
                    div { style: "display: flex; gap: 10px; align-items: center;",
                        if let Some((ok, msg)) = trim_feedback.read().as_ref() {
                            span {
                                style: if *ok { "font-size: 11.5px; color: var(--accent-mint); font-weight: 600;" } else { "font-size: 11.5px; color: #fb7185; font-weight: 600;" },
                                "{msg}"
                            }
                        }
                        button {
                            class: "btn-pill",
                            style: "font-size: 12px; display: inline-flex; align-items: center; gap: 6px;",
                            onclick: move |_| {
                                clear_thumbnail_cache();
                                match trim_memory() {
                                    Ok((before, after)) => {
                                        metrics_res.set(sample_metrics());
                                        let freed_mb = before.saturating_sub(after) as f64 / (1024.0 * 1024.0);
                                        trim_feedback.set(Some((true, format!("✓ Freed {:.1} MB", freed_mb))));
                                    }
                                    Err(err) => {
                                        trim_feedback.set(Some((false, format!("✗ Trim failed: {err}"))));
                                    }
                                }
                            },
                            span { class: "material-icons md-16", "cleaning_services" }
                            "Trim Memory"
                        }
                        button {
                            class: "btn-pill active",
                            style: "display: inline-flex; align-items: center; gap: 6px;",
                            onclick: move |_| on_open_process_manager.call(()),
                            span { class: "material-icons md-16", "analytics" }
                            "Open Process Manager"
                        }
                    }
                }

                match curr_metrics {
                    Ok(m) => {
                        let rss_mb = m.rss_bytes as f64 / (1024.0 * 1024.0);
                        rsx! {
                            div { style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; padding-top: 8px;",
                                div { style: "background: rgba(255,255,255,0.03); padding: 10px; border-radius: 8px; text-align: center;",
                                    span { style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; display: block;", "Current RSS" }
                                    span { style: "font-family: monospace; font-size: 14px; font-weight: 600; color: var(--text-primary);", "{rss_mb:.1} MB" }
                                }
                                div { style: "background: rgba(255,255,255,0.03); padding: 10px; border-radius: 8px; text-align: center;",
                                    span { style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; display: block;", "CPU Usage" }
                                    span { style: "font-family: monospace; font-size: 14px; font-weight: 600; color: var(--accent-mint);", "{m.cpu_usage_pct:.1}%" }
                                }
                                div { style: "background: rgba(255,255,255,0.03); padding: 10px; border-radius: 8px; text-align: center;",
                                    span { style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; display: block;", "Threads" }
                                    span { style: "font-family: monospace; font-size: 14px; font-weight: 600; color: var(--accent-purple);", "{m.thread_count}" }
                                }
                                div { style: "background: rgba(255,255,255,0.03); padding: 10px; border-radius: 8px; text-align: center;",
                                    span { style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; display: block;", "Audio IRQs" }
                                    span { style: "font-family: monospace; font-size: 14px; font-weight: 600; color: var(--accent-mint);", "{m.audio_irq_rate_per_sec:.0}/s" }
                                }
                            }
                        }
                    }
                    Err(err_msg) => rsx! {
                        div { style: "background: rgba(251,113,133,0.10); border: 1px solid rgba(251,113,133,0.45); padding: 12px; border-radius: 8px; color: #fb7185; font-family: monospace; font-size: 12px;",
                            "Process Manager Telemetry Error: {err_msg}"
                        }
                    }
                }
            }
        }
    }
}
