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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TelemetrySample {
    pub cpu_pct: f32,
    pub rss_mb: f32,
}

/// Computes smooth cubic Bezier stroke line and closed polygon fill for time-series charts.
pub fn generate_svg_chart_paths(data: &[f32], max_val: f32, width: f32, height: f32) -> (String, String) {
    if data.is_empty() {
        return (String::new(), String::new());
    }
    let n = data.len();
    let top_pad = 12.0;
    let bot_pad = 16.0;
    let chart_h = (height - top_pad - bot_pad).max(10.0);
    let safe_max = if max_val <= 0.0 { 1.0 } else { max_val };

    let points: Vec<(f32, f32)> = data
        .iter()
        .enumerate()
        .map(|(i, &val)| {
            let x = if n > 1 { (i as f32) / ((n - 1) as f32) * width } else { 0.0 };
            let ratio = (val / safe_max).clamp(0.0, 1.0);
            let y = height - bot_pad - (ratio * chart_h);
            (x, y)
        })
        .collect();

    // Line path
    let mut line_d = format!("M {:.1} {:.1}", points[0].0, points[0].1);
    for i in 0..points.len() - 1 {
        let (p0, p1) = (points[i], points[i + 1]);
        let cx1 = p0.0 + (p1.0 - p0.0) * 0.5;
        let cy1 = p0.1;
        let cx2 = p0.0 + (p1.0 - p0.0) * 0.5;
        let cy2 = p1.1;
        line_d.push_str(&format!(" C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}", cx1, cy1, cx2, cy2, p1.0, p1.1));
    }

    // Fill path (closes along bottom)
    let fill_d = format!(
        "{} L {:.1} {:.1} L {:.1} {:.1} Z",
        line_d,
        width,
        height - bot_pad,
        0.0,
        height - bot_pad
    );

    (line_d, fill_d)
}

#[component]
pub fn ProcessManager() -> Element {
    let mut metrics_res = use_signal(sample_metrics);
    let mut watchguard_res = use_signal(watchguard::get_status);
    let mut trim_status = use_signal(|| None::<(bool, String)>);

    let initial_sample = metrics_res.read().as_ref().ok().map(|m| TelemetrySample {
        cpu_pct: m.cpu_usage_pct,
        rss_mb: m.rss_bytes as f32 / (1024.0 * 1024.0),
    }).unwrap_or(TelemetrySample { cpu_pct: 0.0, rss_mb: 0.0 });

    let mut history = use_signal(|| vec![initial_sample; 30]);

    // Auto-refresh metrics, watchguard status, and rolling telemetry history every 1 second
    use_future(move || async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            let m = sample_metrics();
            let wg = watchguard::get_status();
            if let Ok(ref met) = m {
                let sample = TelemetrySample {
                    cpu_pct: met.cpu_usage_pct,
                    rss_mb: met.rss_bytes as f32 / (1024.0 * 1024.0),
                };
                let mut hist = history.read().clone();
                hist.push(sample);
                if hist.len() > 30 {
                    hist.remove(0);
                }
                history.set(hist);
            }
            metrics_res.set(m);
            watchguard_res.set(wg);
        }
    });

    let current_metrics = metrics_res.read().clone();
    let current_wg = watchguard_res.read().clone();
    let strings = crate::i18n::current_strings();

    // Compute rolling history metrics and SVG paths for charts
    let hist = history.read().clone();
    let cpu_vals: Vec<f32> = hist.iter().map(|s| s.cpu_pct).collect();
    let mem_vals: Vec<f32> = hist.iter().map(|s| s.rss_mb).collect();

    let cur_cpu = cpu_vals.last().copied().unwrap_or(0.0);
    let avg_cpu = if !cpu_vals.is_empty() { cpu_vals.iter().sum::<f32>() / cpu_vals.len() as f32 } else { 0.0 };
    let peak_cpu = cpu_vals.iter().copied().fold(0.0f32, f32::max);

    let cur_mem = mem_vals.last().copied().unwrap_or(0.0);
    let peak_mem = mem_vals.iter().copied().fold(0.0f32, f32::max);
    let mem_scale_max = (peak_mem * 1.25).max(100.0);
    let mem_scale_mid = mem_scale_max * 0.5;

    let (cpu_line_path, cpu_fill_path) = generate_svg_chart_paths(&cpu_vals, 100.0, 500.0, 120.0);
    let (mem_line_path, mem_fill_path) = generate_svg_chart_paths(&mem_vals, mem_scale_max, 500.0, 120.0);

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
                                    let new_m = sample_metrics();
                                    if let Ok(ref met) = new_m {
                                        let sample = TelemetrySample {
                                            cpu_pct: met.cpu_usage_pct,
                                            rss_mb: met.rss_bytes as f32 / (1024.0 * 1024.0),
                                        };
                                        let mut hist = history.read().clone();
                                        hist.push(sample);
                                        if hist.len() > 30 {
                                            hist.remove(0);
                                        }
                                        history.set(hist);
                                    }
                                    metrics_res.set(new_m);
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

                        // Real-Time Telemetry Timeline Charts (CPU & Memory Footprint)
                        div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(420px, 1fr)); gap: 16px;",
                            // CPU Timeline Chart Card
                            div { style: "background: var(--bg-card); padding: 20px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                                div { style: "display: flex; justify-content: space-between; align-items: center;",
                                    div { style: "display: flex; align-items: center; gap: 8px;",
                                        div { class: "live-pulse-dot", style: "width: 8px; height: 8px; border-radius: 50%; background: var(--accent-mint); box-shadow: 0 0 8px var(--accent-mint);" }
                                        h3 { style: "font-size: 15px; font-weight: 600;", "{strings.cpu_timeline_title}" }
                                    }
                                    div { style: "display: flex; gap: 10px; font-family: monospace; font-size: 11.5px;",
                                        span { style: "color: var(--text-muted);", "Cur: " span { style: "color: var(--accent-mint); font-weight: 700;", "{cur_cpu:.1}%" } }
                                        span { style: "color: var(--text-muted);", "Avg: " span { style: "color: var(--text-primary); font-weight: 600;", "{avg_cpu:.1}%" } }
                                        span { style: "color: var(--text-muted);", "Peak: " span { style: "color: #fb7185; font-weight: 700;", "{peak_cpu:.1}%" } }
                                    }
                                }

                                // SVG Chart Viewport
                                div { style: "position: relative; width: 100%; height: 120px; background: rgba(0,0,0,0.22); border-radius: 12px; border: 1px solid rgba(255,255,255,0.05); overflow: hidden;",
                                    svg {
                                        style: "width: 100%; height: 100%; display: block;",
                                        view_box: "0 0 500 120",
                                        preserve_aspect_ratio: "none",
                                        defs {
                                            linearGradient { id: "cpuAreaGrad", x1: "0%", y1: "0%", x2: "0%", y2: "100%",
                                                stop { offset: "0%", stop_color: "var(--accent-mint)", stop_opacity: "0.35" }
                                                stop { offset: "100%", stop_color: "var(--accent-mint)", stop_opacity: "0.00" }
                                            }
                                        }
                                        // Grid lines
                                        line { x1: "0", y1: "12", x2: "500", y2: "12", stroke: "rgba(255,255,255,0.06)", stroke_dasharray: "3 3", stroke_width: "1" }
                                        line { x1: "0", y1: "58", x2: "500", y2: "58", stroke: "rgba(255,255,255,0.06)", stroke_dasharray: "3 3", stroke_width: "1" }
                                        line { x1: "0", y1: "104", x2: "500", y2: "104", stroke: "rgba(255,255,255,0.08)", stroke_width: "1" }

                                        // Area fill
                                        if !cpu_fill_path.is_empty() {
                                            path { d: "{cpu_fill_path}", fill: "url(#cpuAreaGrad)" }
                                        }
                                        // Smooth curve stroke
                                        if !cpu_line_path.is_empty() {
                                            path { d: "{cpu_line_path}", fill: "none", stroke: "var(--accent-mint)", stroke_width: "2.2" }
                                        }
                                    }
                                    // Axis text overlays
                                    span { style: "position: absolute; top: 4px; left: 8px; font-size: 9.5px; font-family: monospace; color: var(--text-muted); pointer-events: none;", "100%" }
                                    span { style: "position: absolute; top: 50px; left: 8px; font-size: 9.5px; font-family: monospace; color: var(--text-muted); pointer-events: none;", "50%" }
                                    span { style: "position: absolute; bottom: 3px; left: 8px; font-size: 9.5px; font-family: monospace; color: var(--text-muted); pointer-events: none;", "0%" }
                                    span { style: "position: absolute; bottom: 3px; right: 8px; font-size: 9.5px; font-family: monospace; color: var(--text-muted); pointer-events: none;", "-30s" }
                                }
                            }

                            // Memory Timeline Chart Card
                            div { style: "background: var(--bg-card); padding: 20px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                                div { style: "display: flex; justify-content: space-between; align-items: center;",
                                    div { style: "display: flex; align-items: center; gap: 8px;",
                                        div { class: "live-pulse-dot", style: "width: 8px; height: 8px; border-radius: 50%; background: var(--accent-purple); box-shadow: 0 0 8px var(--accent-purple);" }
                                        h3 { style: "font-size: 15px; font-weight: 600;", "{strings.memory_timeline_title}" }
                                    }
                                    div { style: "display: flex; gap: 10px; font-family: monospace; font-size: 11.5px;",
                                        span { style: "color: var(--text-muted);", "Cur: " span { style: "color: var(--accent-purple); font-weight: 700;", "{cur_mem:.1} MB" } }
                                        span { style: "color: var(--text-muted);", "Peak: " span { style: "color: var(--text-primary); font-weight: 600;", "{peak_mem:.1} MB" } }
                                    }
                                }

                                // SVG Chart Viewport
                                div { style: "position: relative; width: 100%; height: 120px; background: rgba(0,0,0,0.22); border-radius: 12px; border: 1px solid rgba(255,255,255,0.05); overflow: hidden;",
                                    svg {
                                        style: "width: 100%; height: 100%; display: block;",
                                        view_box: "0 0 500 120",
                                        preserve_aspect_ratio: "none",
                                        defs {
                                            linearGradient { id: "memAreaGrad", x1: "0%", y1: "0%", x2: "0%", y2: "100%",
                                                stop { offset: "0%", stop_color: "var(--accent-purple)", stop_opacity: "0.35" }
                                                stop { offset: "100%", stop_color: "var(--accent-purple)", stop_opacity: "0.00" }
                                            }
                                        }
                                        // Grid lines
                                        line { x1: "0", y1: "12", x2: "500", y2: "12", stroke: "rgba(255,255,255,0.06)", stroke_dasharray: "3 3", stroke_width: "1" }
                                        line { x1: "0", y1: "58", x2: "500", y2: "58", stroke: "rgba(255,255,255,0.06)", stroke_dasharray: "3 3", stroke_width: "1" }
                                        line { x1: "0", y1: "104", x2: "500", y2: "104", stroke: "rgba(255,255,255,0.08)", stroke_width: "1" }

                                        // Area fill
                                        if !mem_fill_path.is_empty() {
                                            path { d: "{mem_fill_path}", fill: "url(#memAreaGrad)" }
                                        }
                                        // Smooth curve stroke
                                        if !mem_line_path.is_empty() {
                                            path { d: "{mem_line_path}", fill: "none", stroke: "var(--accent-purple)", stroke_width: "2.2" }
                                        }
                                    }
                                    // Axis text overlays
                                    span { style: "position: absolute; top: 4px; left: 8px; font-size: 9.5px; font-family: monospace; color: var(--text-muted); pointer-events: none;", "{mem_scale_max:.0} MB" }
                                    span { style: "position: absolute; top: 50px; left: 8px; font-size: 9.5px; font-family: monospace; color: var(--text-muted); pointer-events: none;", "{mem_scale_mid:.0} MB" }
                                    span { style: "position: absolute; bottom: 3px; left: 8px; font-size: 9.5px; font-family: monospace; color: var(--text-muted); pointer-events: none;", "0 MB" }
                                    span { style: "position: absolute; bottom: 3px; right: 8px; font-size: 9.5px; font-family: monospace; color: var(--text-muted); pointer-events: none;", "-30s" }
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_svg_chart_paths_empty() {
        let (line, fill) = generate_svg_chart_paths(&[], 100.0, 500.0, 120.0);
        assert!(line.is_empty());
        assert!(fill.is_empty());
    }

    #[test]
    fn test_generate_svg_chart_paths_single_and_multi_point() {
        let (line_single, fill_single) = generate_svg_chart_paths(&[50.0], 100.0, 500.0, 120.0);
        assert!(line_single.starts_with("M 0.0"));
        assert!(fill_single.ends_with("Z"));

        let data = vec![10.0, 25.0, 80.0, 45.0, 30.0];
        let (line, fill) = generate_svg_chart_paths(&data, 100.0, 500.0, 120.0);
        assert!(line.starts_with("M 0.0"));
        assert!(line.contains('C'));
        assert!(fill.ends_with("Z"));
    }
}
