use dioxus::prelude::*;
use crate::process_manager::{clean_process_memory, sample_metrics};
use crate::library::clear_thumbnail_cache;

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
    let mut metrics = use_signal(sample_metrics);
    let mut trim_status = use_signal(|| None::<String>);

    // Auto-refresh metrics every 1 second
    use_future(move || async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            metrics.set(sample_metrics());
        }
    });

    let m = metrics.read().clone();
    let rss_mb = m.rss_bytes as f64 / (1024.0 * 1024.0);

    let (mem_status_label, mem_status_color) = if rss_mb < 250.0 {
        ("OPTIMAL (<250 MB)", "var(--accent-mint)")
    } else if rss_mb < 800.0 {
        ("MODERATE (250-800 MB)", "#facc15")
    } else {
        ("HIGH MEMORY BLOAT (>800 MB)", "#fb7185")
    };

    rsx! {
        div { class: "process-manager-stage",
            style: "display: flex; flex-direction: column; gap: 24px; max-width: 960px; margin: 0 auto; width: 100%; padding-bottom: 50px;",

            // Header
            div { style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-glass); padding-bottom: 16px;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700;", "Process & Hardware Resource Manager" }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                        "Real-time system & hardware telemetry: CPU, GPU, Memory RSS, and Audio Stream Hardware"
                    }
                }
                div { style: "display: flex; gap: 12px; align-items: center;",
                    if let Some(status) = trim_status.read().as_ref() {
                        span { style: "font-size: 12px; color: var(--accent-mint); font-weight: 600;", "{status}" }
                    }
                    button {
                        class: "btn-pill active",
                        style: "font-weight: 600; padding: 8px 18px; display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| {
                            clear_thumbnail_cache();
                            let (before, after) = clean_process_memory();
                            metrics.set(sample_metrics());
                            let freed = before.saturating_sub(after);
                            trim_status.set(Some(format!("✓ Memory Cleaned (Freed {})", format_bytes(freed))));
                        },
                        span { class: "material-icons md-16", "cleaning_services" }
                        "Clean up Process Memory"
                    }
                }
            }

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
                            h2 { style: "font-size: 22px; font-weight: 600; color: var(--text-secondary);",
                                "{m.gpu_device_name}"
                            }
                        }
                    }
                    div { style: "margin-top: 12px; font-size: 11.5px; color: var(--text-secondary); line-height: 1.4;",
                        if let Some(max) = m.gpu_max_freq_mhz {
                            div { "Max Clock: " span { style: "font-family: monospace; color: var(--text-primary);", "{max} MHz" } }
                        }
                        div { "Driver: " span { style: "color: var(--accent-mint); font-weight: 600;", "{m.gpu_driver}" } }
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
                h3 { style: "font-size: 16px; font-weight: 600;", "ALSA Kernel Stream & Hardware DAC Status" }
                
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

                div { style: "font-size: 12px; color: var(--text-muted); line-height: 1.5; background: rgba(45,212,191,0.05); padding: 12px; border-radius: 8px; border-left: 3px solid var(--accent-mint);",
                    "The ALSA driver hardware interrupt occurs once per period buffer (typically 1024 frames @ 48,000 Hz = ~46.8 IRQs/sec) to request audio chunks without starving DAC FIFO queues."
                }
            }

            // Certified Dynamic Shared Libraries Card (.so / .dll)
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    div {
                        h3 { style: "font-size: 16px; font-weight: 600;", "Certified External Shared Libraries (.so / .dll)" }
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
