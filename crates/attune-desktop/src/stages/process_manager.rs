use dioxus::prelude::*;
use crate::dynamic_components::watchguard;
use crate::library::clear_thumbnail_cache;
use crate::process_manager::{clean_process_memory, sample_metrics};

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[component]
pub fn ProcessManager() -> Element {
    let mut metrics_res = use_signal(sample_metrics);
    let mut watchguard_res = use_signal(watchguard::get_status);
    let mut trim_status = use_signal(|| None::<(bool, String)>);

    // Auto-refresh metrics and watchguard status every 1 second
    use_future(move || async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            metrics_res.set(sample_metrics());
            watchguard_res.set(watchguard::get_status());
        }
    });

    let current_metrics = metrics_res.read().clone();
    let current_wg = watchguard_res.read().clone();
    let strings = crate::i18n::current_strings();

    rsx! {
        div { class: "process-manager-stage",
            style: "display: flex; flex-direction: column; gap: 24px; max-width: 960px; margin: 0 auto; width: 100%; padding-bottom: 50px;",

            // Header
            div { style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-glass); padding-bottom: 16px;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700;", "{strings.process_and_hardware_manager}" }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                        "Live kernel telemetry via libattune_procmgr & real-time cadence guardrails via libattune_watchguard"
                    }
                }
                div { style: "display: flex; gap: 12px; align-items: center;",
                    if let Some((ok, status)) = trim_status.read().as_ref() {
                        span {
                            style: if *ok { "font-size: 12px; color: var(--accent-mint); font-weight: 600;" } else { "font-size: 12px; color: #fb7185; font-weight: 600;" },
                            "{status}"
                        }
                    }
                    button {
                        class: "btn-pill active",
                        style: "font-weight: 600; padding: 8px 18px; display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| {
                            clear_thumbnail_cache();
                            match clean_process_memory() {
                                Ok((before, after)) => {
                                    metrics_res.set(sample_metrics());
                                    let freed = before.saturating_sub(after);
                                    trim_status.set(Some((true, format!("✓ Memory Cleaned (Freed {})", format_bytes(freed)))));
                                }
                                Err(err) => {
                                    trim_status.set(Some((false, format!("✗ Memory Cleanup Failed: {err}"))));
                                }
                            }
                        },
                        span { class: "material-icons md-16", "cleaning_services" }
                        "Clean up Process Memory"
                    }
                }
            }

            match current_metrics {
                Ok(m) => {
                    let rss_mb = m.rss_bytes as f64 / (1024.0 * 1024.0);
                    let (mem_status_label, mem_status_color) = if rss_mb < 250.0 {
                        ("OPTIMAL (<250 MB)", "var(--accent-mint)")
                    } else if rss_mb < 800.0 {
                        ("MODERATE (250-800 MB)", "#facc15")
                    } else {
                        ("HIGH MEMORY BLOAT (>800 MB)", "#fb7185")
                    };

                    rsx! {
                        // Top Telemetry Grid (4 Stat Cards)
                        div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); gap: 16px;",
                            // Card 1: Resident Memory (RSS)
                            div { style: "background: var(--bg-card); padding: 18px; border-radius: 16px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; justify-content: space-between;",
                                div {
                                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",
                                        span { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;", "Process Memory (RSS)" }
                                        span { style: "font-size: 10px; font-weight: 700; color: {mem_status_color};", "{mem_status_label}" }
                                    }
                                    h2 { style: "font-size: 28px; font-weight: 700; font-family: monospace; color: var(--text-primary);",
                                        "{format_bytes(m.rss_bytes)}"
                                    }
                                }
                                div { style: "margin-top: 12px; font-size: 11.5px; color: var(--text-secondary); line-height: 1.4;",
                                    div { "Peak RSS: " span { style: "font-family: monospace; color: var(--text-primary);", "{format_bytes(m.rss_peak_bytes)}" } }
                                    div { "Virtual (VMem): " span { style: "font-family: monospace; color: var(--text-muted);", "{format_bytes(m.vmem_bytes)}" } }
                                }
                            }

                            // Card 2: CPU Utilization & Threads
                            div { style: "background: var(--bg-card); padding: 18px; border-radius: 16px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; justify-content: space-between;",
                                div {
                                    span { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 8px; display: block;", "Process CPU Load" }
                                    h2 { style: "font-size: 28px; font-weight: 700; font-family: monospace; color: var(--accent-mint);",
                                        "{m.cpu_usage_pct:.1}%"
                                    }
                                }
                                div { style: "margin-top: 12px; font-size: 11.5px; color: var(--text-secondary); line-height: 1.4;",
                                    div { "Active OS Threads: " span { style: "font-family: monospace; color: var(--text-primary);", "{m.thread_count}" } }
                                    div { "Context Switches: " span { style: "font-family: monospace; color: var(--text-muted);", "{m.voluntary_ctxt_switches + m.nonvoluntary_ctxt_switches}" } }
                                }
                            }

                            // Card 3: GPU & Compositor
                            div { style: "background: var(--bg-card); padding: 18px; border-radius: 16px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; justify-content: space-between;",
                                div {
                                    span { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 8px; display: block;", "GPU Core Clock" }
                                    if let Some(cur) = m.gpu_cur_freq_mhz {
                                        h2 { style: "font-size: 28px; font-weight: 700; font-family: monospace; color: var(--accent-purple);",
                                            "{cur} MHz"
                                        }
                                    } else {
                                        h2 { style: "font-size: 18px; font-weight: 600; color: var(--text-secondary);",
                                            "{m.gpu_device_name}"
                                        }
                                    }
                                }
                                div { style: "margin-top: 12px; font-size: 11.5px; color: var(--text-secondary); line-height: 1.4;",
                                    if let Some(max) = m.gpu_max_freq_mhz {
                                        div { "Max Clock: " span { style: "font-family: monospace; color: var(--text-primary);", "{max} MHz" } }
                                    }
                                    div { "Driver: " span { style: "color: var(--accent-mint); font-weight: 600;", "{m.gpu_driver}" } }
                                    div { style: "font-size: 10.5px; color: var(--text-muted); margin-top: 2px;", "{m.gpu_hardware_accel}" }
                                }
                            }

                            // Card 4: Audio IRQ Telemetry
                            div { style: "background: var(--bg-card); padding: 18px; border-radius: 16px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; justify-content: space-between;",
                                div {
                                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",
                                        span { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;", "Sound Card IRQ Rate" }
                                        if let Some(num) = m.audio_irq_number {
                                            span { style: "font-size: 10px; background: rgba(45,212,191,0.15); color: var(--accent-mint); padding: 2px 6px; border-radius: 4px; font-family: monospace;", "IRQ {num}" }
                                        }
                                    }
                                    h2 { style: "font-size: 28px; font-weight: 700; font-family: monospace; color: var(--accent-mint);",
                                        "{m.audio_irq_rate_per_sec:.0} /s"
                                    }
                                }
                                div { style: "margin-top: 12px; font-size: 11.5px; color: var(--text-secondary); line-height: 1.4;",
                                    div { "Device: " span { style: "font-family: monospace; color: var(--text-primary); font-size: 11px;", "{m.audio_irq_device}" } }
                                    div { "Total IRQs: " span { style: "font-family: monospace; color: var(--text-muted);", "{m.audio_irq_count}" } }
                                }
                            }
                        }

                        // Audio DAC Hardware Interface Details
                        div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                            h3 { style: "font-size: 16px; font-weight: 600;", "{strings.kernel_dac_status}" }

                            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 14px;",
                                div { style: "background: rgba(255,255,255,0.03); padding: 12px 14px; border-radius: 10px; border: 1px solid var(--border-glass);",
                                    span { style: "display: block; font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 4px;", "PCM Stream State" }
                                    span { style: "font-family: monospace; font-size: 14px; font-weight: 600; color: if m.alsa_stream_state.contains(\"RUNNING\") { \"var(--accent-mint)\" } else { \"var(--text-secondary)\" };",
                                        "{m.alsa_stream_state}"
                                    }
                                }
                                div { style: "background: rgba(255,255,255,0.03); padding: 12px 14px; border-radius: 10px; border: 1px solid var(--border-glass);",
                                    span { style: "display: block; font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 4px;", "Hardware DAC Parameters" }
                                    span { style: "font-family: monospace; font-size: 12px; color: var(--text-secondary);",
                                        "{m.alsa_hw_params}"
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err_msg) => rsx! {
                    div { style: "background: rgba(251,113,133,0.10); border: 1px solid rgba(251,113,133,0.45); padding: 18px; border-radius: 16px; color: #fb7185;",
                        div { style: "display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 15px; margin-bottom: 6px;",
                            span { class: "material-icons md-18", "error_outline" }
                            "Process Manager Telemetry Failure (libattune_procmgr)"
                        }
                        p { style: "font-family: monospace; font-size: 12.5px; color: var(--text-primary);", "{err_msg}" }
                    }
                }
            }

            // Watchguard Real-Time Stability & Hardware Guardrails (libattune_watchguard)
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    div {
                        h3 { style: "font-size: 16px; font-weight: 600;", "{strings.watchguard_stability}" }
                        p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;",
                            "Live DSP buffer underruns, signal clipping, and kernel hardware telemetry guardrails via libattune_watchguard (Desktop Hardware Mode)"
                        }
                    }
                    button {
                        class: "btn-pill",
                        style: "font-size: 12px; display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| {
                            attune_core::engine::mixer::reset_audio_telemetry();
                            if let Err(e) = watchguard::reset_metrics() {
                                trim_status.set(Some((false, format!("✗ Watchguard Reset Failed: {e}"))));
                            } else {
                                watchguard_res.set(watchguard::get_status());
                                trim_status.set(Some((true, "✓ Watchguard counters reset".to_string())));
                            }
                        },
                        span { class: "material-icons md-16", "restart_alt" }
                        "Reset Watchguard"
                    }
                }

                match current_wg {
                    Ok(wg) => rsx! {
                        if let Some(warn) = wg.warning_message.as_ref() {
                            div { style: "background: rgba(251,113,133,0.12); border-left: 3px solid #fb7185; padding: 10px 14px; border-radius: 8px; color: #fb7185; font-size: 12.5px; font-weight: 600;",
                                "{warn}"
                            }
                        }

                        div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px;",
                            div { style: "background: rgba(255,255,255,0.03); padding: 12px 14px; border-radius: 10px; border: 1px solid var(--border-glass);",
                                span { style: "display: block; font-size: 10.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 4px;", "System Stability" }
                                span {
                                    style: if wg.is_overloaded { "font-family: monospace; font-size: 16px; font-weight: 700; color: #fb7185;" } else { "font-family: monospace; font-size: 16px; font-weight: 700; color: var(--accent-mint);" },
                                    if wg.is_overloaded { "OVERLOADED (Score {wg.instability_score:.2})" } else { "STABLE (Score {wg.instability_score:.2})" }
                                }
                            }
                            div { style: "background: rgba(255,255,255,0.03); padding: 12px 14px; border-radius: 10px; border: 1px solid var(--border-glass);",
                                span { style: "display: block; font-size: 10.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 4px;", "Hardware Resource Guard" }
                                span { style: "font-family: monospace; font-size: 16px; font-weight: 700; color: var(--text-primary);",
                                    "{wg.cpu_usage_pct:.1}% CPU • {wg.logical_cpu_cores} Cores"
                                }
                            }
                            div { style: "background: rgba(255,255,255,0.03); padding: 12px 14px; border-radius: 10px; border: 1px solid var(--border-glass);",
                                span { style: "display: block; font-size: 10.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 4px;", "Buffer Underruns" }
                                span {
                                    style: if wg.underrun_count > 0 { "font-family: monospace; font-size: 16px; font-weight: 700; color: #fb7185;" } else { "font-family: monospace; font-size: 16px; font-weight: 700; color: var(--accent-mint);" },
                                    "{wg.underrun_count} total ({wg.recent_underrun_events} recent)"
                                }
                            }
                            div { style: "background: rgba(255,255,255,0.03); padding: 12px 14px; border-radius: 10px; border: 1px solid var(--border-glass);",
                                span { style: "display: block; font-size: 10.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 4px;", "Signal Clipping Events" }
                                span {
                                    style: if wg.clipping_count > 0 { "font-family: monospace; font-size: 16px; font-weight: 700; color: #facc15;" } else { "font-family: monospace; font-size: 16px; font-weight: 700; color: var(--accent-mint);" },
                                    "{wg.clipping_count} total ({wg.recent_clipping_events} recent)"
                                }
                            }
                        }
                    },
                    Err(wg_err) => rsx! {
                        div { style: "background: rgba(251,113,133,0.10); border: 1px solid rgba(251,113,133,0.45); padding: 14px; border-radius: 10px; color: #fb7185; font-family: monospace; font-size: 12px;",
                            "Watchguard Telemetry Error: {wg_err}"
                        }
                    }
                }
            }

            // Certified Dynamic Shared Libraries Card (.so / .dll)
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    div {
                        h3 { style: "font-size: 16px; font-weight: 600;", "{strings.certified_shared_libraries}" }
                        p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;", "Verified dynamic C-ABI components loaded at runtime without static code fallbacks" }
                    }
                    span { class: "quality-badge quality-very-good",
                        span { class: "material-icons md-14", style: "margin-right: 4px; vertical-align: -2px;", "verified" }
                        "All Components Active"
                    }
                }

                div { style: "display: flex; flex-direction: column; gap: 8px;",
                    for comp in crate::dynamic_components::get_loaded_components_summary() {
                        div {
                            key: "{comp.filename}",
                            style: "display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; background: rgba(255,255,255,0.03); border-radius: 10px; border: 1px solid var(--border-glass); font-size: 12.5px;",
                            div { style: "display: flex; flex-direction: column; gap: 2px;",
                                span { style: "font-weight: 600; color: var(--text-primary);", "{comp.name}" }
                                span { style: "font-family: monospace; font-size: 11px; color: var(--text-muted);", "{comp.path.display()}" }
                            }
                            div { style: "display: flex; gap: 8px; align-items: center;",
                                span { style: "padding: 2px 8px; border-radius: 6px; background: rgba(45,212,191,0.12); color: var(--accent-mint); font-family: monospace; font-size: 11px; font-weight: 700;",
                                    "v{comp.version}"
                                }
                                span { style: "padding: 2px 8px; border-radius: 6px; background: rgba(255,255,255,0.06); color: var(--text-secondary); font-family: monospace; font-size: 11px;",
                                    "{comp.filename}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
