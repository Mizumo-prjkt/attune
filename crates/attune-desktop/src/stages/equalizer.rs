use dioxus::prelude::*;
use attune_core::{MasterFxParams, EQ_BANDS_COUNT, EQ_CENTER_FREQS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Flat,
    BassBoost,
    VocalClarity,
    Electronic,
    Rock,
    Acoustic,
}

impl Preset {
    pub const ALL: [Preset; 6] = [
        Preset::Flat,
        Preset::BassBoost,
        Preset::VocalClarity,
        Preset::Electronic,
        Preset::Rock,
        Preset::Acoustic,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Preset::Flat => "Flat",
            Preset::BassBoost => "Bass Boost",
            Preset::VocalClarity => "Vocal Clarity",
            Preset::Electronic => "Electronic / Dance",
            Preset::Rock => "Rock / Punch",
            Preset::Acoustic => "Acoustic / Warm",
        }
    }

    pub fn apply(self, params: &mut MasterFxParams) {
        match self {
            Preset::Flat => {
                params.band_gains_db = [0.0; EQ_BANDS_COUNT];
                params.bass_enabled = false;
                params.clarity_enabled = false;
                params.stereo_width = 1.0;
                params.preamp_db = -3.0;
            }
            Preset::BassBoost => {
                params.band_gains_db = [5.5, 4.5, 3.0, 1.5, 0.0, 0.0, 0.0, 1.0, 1.5, 2.0];
                params.bass_enabled = true;
                params.bass_gain_db = 6.0;
                params.bass_freq_hz = 75.0;
                params.clarity_enabled = false;
                params.stereo_width = 1.1;
                params.preamp_db = -4.5;
            }
            Preset::VocalClarity => {
                params.band_gains_db = [-1.0, -0.5, 0.0, 1.0, 2.5, 3.5, 4.0, 3.0, 2.5, 2.0];
                params.bass_enabled = false;
                params.clarity_enabled = true;
                params.clarity_wet = 0.20;
                params.stereo_width = 1.15;
                params.preamp_db = -3.5;
            }
            Preset::Electronic => {
                params.band_gains_db = [6.0, 5.0, 2.5, -0.5, -1.0, 0.5, 2.0, 3.5, 4.5, 5.0];
                params.bass_enabled = true;
                params.bass_gain_db = 5.0;
                params.bass_freq_hz = 80.0;
                params.clarity_enabled = true;
                params.clarity_wet = 0.15;
                params.stereo_width = 1.25;
                params.preamp_db = -4.0;
            }
            Preset::Rock => {
                params.band_gains_db = [4.0, 3.0, 1.5, 2.0, 2.5, 1.5, 2.0, 3.0, 2.5, 2.0];
                params.bass_enabled = false;
                params.clarity_enabled = true;
                params.clarity_wet = 0.12;
                params.stereo_width = 1.10;
                params.preamp_db = -3.5;
            }
            Preset::Acoustic => {
                params.band_gains_db = [2.0, 2.5, 1.5, 1.0, 1.5, 2.0, 1.5, 1.0, 1.5, 2.0];
                params.bass_enabled = false;
                params.clarity_enabled = false;
                params.stereo_width = 1.05;
                params.preamp_db = -2.5;
            }
        }
    }
}

fn format_freq(freq: f32) -> String {
    if freq >= 1000.0 {
        let k = freq / 1000.0;
        if (k - k.round()).abs() < 0.05 {
            format!("{}k", k.round() as u32)
        } else {
            format!("{:.1}k", k)
        }
    } else {
        format!("{:.0}", freq)
    }
}

/// Computes the SVG smooth cubic spline path, closed fill path, and 10 control points
/// for the frequency response curve.
/// SVG viewBox is (0 0 1000 120).
/// Center (0 dB) is at y = 60.0.
/// Range is -12 dB (y = 108.0) to +12 dB (y = 12.0).
pub fn compute_eq_curve_paths(band_gains_db: &[f32; EQ_BANDS_COUNT]) -> (String, String, Vec<(f32, f32)>) {
    let mut points = Vec::with_capacity(EQ_BANDS_COUNT);
    for (i, &gain) in band_gains_db.iter().enumerate() {
        let x = 50.0 + (i as f32) * 100.0;
        let clamped_gain = gain.clamp(-12.0, 12.0);
        // Map [-12.0, +12.0] dB to y in [108.0, 12.0]
        let y = 60.0 - (clamped_gain / 12.0) * 48.0;
        points.push((x, y));
    }

    let y0 = points[0].1;
    let y_last = points[EQ_BANDS_COUNT - 1].1;

    // Build smooth cubic Bezier path
    let mut stroke_path = format!("M 0,{:.1} L 50,{:.1}", y0, y0);
    for i in 0..(EQ_BANDS_COUNT - 1) {
        let p0 = points[i];
        let p1 = points[i + 1];
        let cx = (p0.0 + p1.0) / 2.0;
        stroke_path.push_str(&format!(" C {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}", cx, p0.1, cx, p1.1, p1.0, p1.1));
    }
    stroke_path.push_str(&format!(" L 1000,{:.1}", y_last));

    // Build fill path down to 0 dB line (y = 60.0)
    let fill_path = format!("{} L 1000,60.0 L 0,60.0 Z", stroke_path);

    (stroke_path, fill_path, points)
}

#[component]
pub fn Equalizer(
    params: MasterFxParams,
    on_change: EventHandler<MasterFxParams>,
) -> Element {
    let mut local_params = use_signal(|| params.clone());

    // Synchronize if parent params change externally
    use_effect(use_reactive(&params, move |p| {
        local_params.set(p);
    }));

    let p = local_params.read().clone();
    let (stroke_path, fill_path, curve_points) = compute_eq_curve_paths(&p.band_gains_db);

    rsx! {
        div { class: "equalizer-stage",
            style: "display: flex; flex-direction: column; gap: 24px; padding-bottom: 40px; max-width: 900px; margin: 0 auto; width: 100%;",

            // Stage Header with Human-Understandable Switch Toggle
            div { style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-glass); padding-bottom: 16px; flex-wrap: wrap; gap: 12px;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700;", "Equalizer & Master FX" }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                        "Studio-Grade 6-Stage Mastering Rack • Real-Time DSP Audio Processing"
                    }
                }
                div { style: "display: flex; gap: 14px; align-items: center;",
                    button {
                        class: "btn-pill",
                        onclick: move |_| {
                            let mut next = local_params.read().clone();
                            Preset::Flat.apply(&mut next);
                            local_params.set(next.clone());
                            on_change.call(next);
                        },
                        "↺ Reset to Flat"
                    }

                    // Human-Understandable Master FX Switch Toggle
                    div {
                        class: "switch-toggle-row",
                        role: "switch",
                        aria_checked: "{p.enabled}",
                        title: if p.enabled { "Click to bypass Master FX" } else { "Click to activate Master FX" },
                        onclick: move |_| {
                            let mut next = local_params.read().clone();
                            next.enabled = !next.enabled;
                            local_params.set(next.clone());
                            on_change.call(next);
                        },
                        span { class: "switch-label", "Master FX" }
                        div { class: if p.enabled { "switch-track active" } else { "switch-track" },
                            div { class: "switch-thumb" }
                        }
                        span {
                            class: if p.enabled { "switch-status active" } else { "switch-status bypassed" },
                            if p.enabled { "ON" } else { "BYPASSED" }
                        }
                    }
                }
            }

            // Presets Bar
            div { style: "background: var(--bg-card); padding: 14px 18px; border-radius: 14px; border: 1px solid var(--border-glass);",
                span { style: "font-size: 11px; font-weight: 700; color: var(--accent-mint); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px; display: block;", "Sound Signature Presets" }
                div { style: "display: flex; gap: 10px; flex-wrap: wrap;",
                    for &pr in Preset::ALL.iter() {
                        button {
                            key: "{pr.name()}",
                            class: "btn-pill",
                            onclick: move |_| {
                                let mut next = local_params.read().clone();
                                pr.apply(&mut next);
                                local_params.set(next.clone());
                                on_change.call(next);
                            },
                            "{pr.name()}"
                        }
                    }
                }
            }

            // Card 1: Preamp Headroom Pad
            div { style: "background: var(--bg-card); padding: 18px 22px; border-radius: 16px; border: 1px solid var(--border-glass);",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;",
                    div {
                        h3 { style: "font-size: 15px; font-weight: 600;", "Preamp Headroom Gain Pad" }
                        p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;", "Protects internal cascaded biquads against inter-sample filter clipping" }
                    }
                    span { style: "font-family: monospace; font-size: 13px; font-weight: 600; color: var(--accent-mint);",
                        "{p.preamp_db:+.1} dB"
                    }
                }
                input {
                    r#type: "range",
                    class: "vol-slider",
                    style: "width: 100%; height: 6px;",
                    min: "-120",
                    max: "60",
                    value: "{(p.preamp_db * 10.0) as i32}",
                    oninput: move |evt| {
                        if let Ok(val) = evt.value().parse::<f32>() {
                            let mut next = local_params.read().clone();
                            next.preamp_db = val / 10.0;
                            local_params.set(next.clone());
                            on_change.call(next);
                        }
                    },
                }
            }

            // Card 2: 10-Band Graphic Equalizer with Visual Frequency Curve & Zero-Overshoot Sliders
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 16px; border: 1px solid var(--border-glass); box-sizing: border-box; overflow: hidden;",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    div {
                        h3 { style: "font-size: 16px; font-weight: 600;", "10-Band Graphic Equalizer" }
                        p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;", "Studio peaking IIR filters (-12 dB to +12 dB range) • Real-Time Frequency Response" }
                    }
                    button {
                        class: "btn-pill",
                        style: "font-size: 11px; padding: 4px 10px;",
                        onclick: move |_| {
                            let mut next = local_params.read().clone();
                            next.band_gains_db = [0.0; EQ_BANDS_COUNT];
                            local_params.set(next.clone());
                            on_change.call(next);
                        },
                        "Zero Bands"
                    }
                }

                // Visual Frequency Response Curve Graph
                div {
                    class: "eq-curve-visualizer",
                    style: if !p.enabled { "opacity: 0.45; filter: grayscale(70%);" } else { "" },

                    svg {
                        view_box: "0 0 1000 120",
                        preserve_aspect_ratio: "none",
                        style: "width: 100%; height: 100%; display: block;",

                        defs {
                            linearGradient {
                                id: "eq-curve-grad",
                                x1: "0",
                                y1: "0",
                                x2: "0",
                                y2: "1",
                                stop { offset: "0%", stop_color: "var(--accent-mint)", stop_opacity: "0.32" }
                                stop { offset: "100%", stop_color: "var(--accent-mint)", stop_opacity: "0.02" }
                            }
                        }

                        // Horizontal Reference Grid
                        line { x1: "0", y1: "12", x2: "1000", y2: "12", stroke: "rgba(255,255,255,0.06)", stroke_width: "1" }
                        text { x: "12", y: "16", fill: "rgba(255,255,255,0.35)", font_size: "10", font_family: "monospace", font_weight: "600", "+12 dB" }

                        line { x1: "0", y1: "60", x2: "1000", y2: "60", stroke: "rgba(255,255,255,0.16)", stroke_width: "1.2", stroke_dasharray: "6,4" }
                        text { x: "12", y: "64", fill: "rgba(255,255,255,0.45)", font_size: "10", font_family: "monospace", font_weight: "600", "0 dB" }

                        line { x1: "0", y1: "108", x2: "1000", y2: "108", stroke: "rgba(255,255,255,0.06)", stroke_width: "1" }
                        text { x: "12", y: "112", fill: "rgba(255,255,255,0.35)", font_size: "10", font_family: "monospace", font_weight: "600", "-12 dB" }

                        // Vertical Band Guide Lines
                        for (i, &freq) in EQ_CENTER_FREQS.iter().enumerate() {
                            {
                                let x = 50.0 + (i as f32) * 100.0;
                                let freq_str = format_freq(freq);
                                rsx! {
                                    line {
                                        key: "vguide-{i}",
                                        x1: "{x}",
                                        y1: "10",
                                        x2: "{x}",
                                        y2: "110",
                                        stroke: "rgba(255,255,255,0.04)",
                                        stroke_width: "1",
                                    }
                                    text {
                                        key: "vtext-{i}",
                                        x: "{x}",
                                        y: "114",
                                        text_anchor: "middle",
                                        fill: "rgba(255,255,255,0.22)",
                                        font_size: "9",
                                        font_family: "sans-serif",
                                        font_weight: "600",
                                        "{freq_str}"
                                    }
                                }
                            }
                        }

                        // Shaded Area Fill
                        path {
                            d: "{fill_path}",
                            fill: "url(#eq-curve-grad)",
                        }

                        // Frequency Response Spline Curve
                        path {
                            d: "{stroke_path}",
                            fill: "none",
                            stroke: "var(--accent-mint)",
                            stroke_width: "2.5",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                        }

                        // Control Node Dots
                        for (i, &(x, y)) in curve_points.iter().enumerate() {
                            {
                                let is_boost_or_cut = p.band_gains_db[i].abs() > 0.1;
                                rsx! {
                                    if is_boost_or_cut {
                                        circle {
                                            key: "dot-glow-{i}",
                                            cx: "{x}",
                                            cy: "{y}",
                                            r: "7",
                                            fill: "rgba(45, 212, 191, 0.25)",
                                        }
                                    }
                                    circle {
                                        key: "dot-{i}",
                                        cx: "{x}",
                                        cy: "{y}",
                                        r: if is_boost_or_cut { "3.5" } else { "2.5" },
                                        fill: if is_boost_or_cut { "#ffffff" } else { "rgba(255,255,255,0.6)" },
                                        stroke: if is_boost_or_cut { "var(--accent-mint)" } else { "transparent" },
                                        stroke_width: "1.5",
                                    }
                                }
                            }
                        }
                    }

                    if !p.enabled {
                        div {
                            style: "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); background: rgba(9, 11, 16, 0.85); border: 1px solid rgba(255,255,255,0.12); border-radius: 20px; padding: 6px 16px; font-size: 11px; font-weight: 700; color: var(--text-muted); letter-spacing: 0.08em; pointer-events: none; text-transform: uppercase;",
                            "Master FX Bypassed — EQ Inactive"
                        }
                    }
                }

                // 10 Vertical Sliders Rack (Zero Overshoot)
                div { class: "eq-sliders-row",
                    for (band_idx, &freq) in EQ_CENTER_FREQS.iter().enumerate() {
                        {
                            let gain = p.band_gains_db[band_idx];
                            let freq_label = format_freq(freq);
                            let gain_str = format!("{gain:+.1}");

                            rsx! {
                                div {
                                    key: "{band_idx}",
                                    class: "eq-band-col",
                                    span {
                                        class: "eq-band-gain",
                                        style: if gain.abs() > 0.1 { "color: var(--accent-mint);" } else { "color: var(--text-muted);" },
                                        "{gain_str}"
                                    }
                                    div { class: "eq-slider-well",
                                        div { class: "eq-track-slot" }
                                        div { class: "eq-zero-tick" }
                                        input {
                                            r#type: "range",
                                            class: "eq-slider-input",
                                            min: "-120",
                                            max: "120",
                                            value: "{(gain * 10.0) as i32}",
                                            oninput: move |evt| {
                                                if let Ok(v) = evt.value().parse::<f32>() {
                                                    let mut next = local_params.read().clone();
                                                    next.band_gains_db[band_idx] = v / 10.0;
                                                    local_params.set(next.clone());
                                                    on_change.call(next);
                                                }
                                            },
                                        }
                                    }
                                    span { class: "eq-band-freq",
                                        "{freq_label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Card 3 & 4: Sub-Harmonic Synthesizer & Harmonic Exciter (2-Column Grid)
            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 20px;",
                // Pure Bass+ Card
                div { style: "background: var(--bg-card); padding: 20px; border-radius: 16px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        div {
                            h3 { style: "font-size: 15px; font-weight: 600;", "Pure Bass+ Synthesizer" }
                            p { style: "font-size: 11.5px; color: var(--text-muted);", "Sub-harmonic acoustic low-end generator" }
                        }
                        div {
                            class: "switch-toggle-row",
                            role: "switch",
                            aria_checked: "{p.bass_enabled}",
                            onclick: move |_| {
                                let mut next = local_params.read().clone();
                                next.bass_enabled = !next.bass_enabled;
                                local_params.set(next.clone());
                                on_change.call(next);
                            },
                            div { class: if p.bass_enabled { "switch-track active" } else { "switch-track" },
                                div { class: "switch-thumb" }
                            }
                            span {
                                class: if p.bass_enabled { "switch-status active" } else { "switch-status bypassed" },
                                if p.bass_enabled { "ON" } else { "OFF" }
                            }
                        }
                    }

                    // Bass Gain Slider
                    div {
                        div { style: "display: flex; justify-content: space-between; font-size: 12px; margin-bottom: 6px;",
                            span { style: "color: var(--text-secondary);", "Sub Boost Gain" }
                            span { style: "font-family: monospace; color: var(--accent-mint);", "{p.bass_gain_db:+.1} dB" }
                        }
                        input {
                            r#type: "range",
                            min: "0",
                            max: "120",
                            disabled: !p.bass_enabled,
                            value: "{(p.bass_gain_db * 10.0) as i32}",
                            style: "width: 100%; accent-color: var(--accent-mint); opacity: if p.bass_enabled { \"1.0\" } else { \"0.4\" };",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse::<f32>() {
                                    let mut next = local_params.read().clone();
                                    next.bass_gain_db = v / 10.0;
                                    local_params.set(next.clone());
                                    on_change.call(next);
                                }
                            },
                        }
                    }

                    // Bass Frequency Slider
                    div {
                        div { style: "display: flex; justify-content: space-between; font-size: 12px; margin-bottom: 6px;",
                            span { style: "color: var(--text-secondary);", "Cutoff Frequency" }
                            span { style: "font-family: monospace; color: var(--accent-mint);", "{p.bass_freq_hz:.0} Hz" }
                        }
                        input {
                            r#type: "range",
                            min: "40",
                            max: "120",
                            disabled: !p.bass_enabled,
                            value: "{p.bass_freq_hz as i32}",
                            style: "width: 100%; accent-color: var(--accent-mint); opacity: if p.bass_enabled { \"1.0\" } else { \"0.4\" };",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse::<f32>() {
                                    let mut next = local_params.read().clone();
                                    next.bass_freq_hz = v;
                                    local_params.set(next.clone());
                                    on_change.call(next);
                                }
                            },
                        }
                    }
                }

                // Clarity Exciter Card
                div { style: "background: var(--bg-card); padding: 20px; border-radius: 16px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center;",
                        div {
                            h3 { style: "font-size: 15px; font-weight: 600;", "Attune Clarity & Vocal Air" }
                            p { style: "font-size: 11.5px; color: var(--text-muted);", "Harmonic exciter for high-end sparkle" }
                        }
                        div {
                            class: "switch-toggle-row",
                            role: "switch",
                            aria_checked: "{p.clarity_enabled}",
                            onclick: move |_| {
                                let mut next = local_params.read().clone();
                                next.clarity_enabled = !next.clarity_enabled;
                                local_params.set(next.clone());
                                on_change.call(next);
                            },
                            div { class: if p.clarity_enabled { "switch-track active" } else { "switch-track" },
                                div { class: "switch-thumb" }
                            }
                            span {
                                class: if p.clarity_enabled { "switch-status active" } else { "switch-status bypassed" },
                                if p.clarity_enabled { "ON" } else { "OFF" }
                            }
                        }
                    }

                    // Clarity Wet Level
                    div {
                        div { style: "display: flex; justify-content: space-between; font-size: 12px; margin-bottom: 6px;",
                            span { style: "color: var(--text-secondary);", "Exciter Wet Level" }
                            span { style: "font-family: monospace; color: var(--accent-mint);", "{(p.clarity_wet * 100.0) as u32}%" }
                        }
                        input {
                            r#type: "range",
                            min: "0",
                            max: "50",
                            disabled: !p.clarity_enabled,
                            value: "{(p.clarity_wet * 100.0) as i32}",
                            style: "width: 100%; accent-color: var(--accent-mint); opacity: if p.clarity_enabled { \"1.0\" } else { \"0.4\" };",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse::<f32>() {
                                    let mut next = local_params.read().clone();
                                    next.clarity_wet = v / 100.0;
                                    local_params.set(next.clone());
                                    on_change.call(next);
                                }
                            },
                        }
                    }
                }
            }

            // Card 5: Stereo Expander & Brickwall Limiter
            div { style: "background: var(--bg-card); padding: 20px 22px; border-radius: 16px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                div {
                    h3 { style: "font-size: 15px; font-weight: 600;", "Stereo Soundstage & Brickwall Limiter" }
                    p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;", "Mid-Side acoustic soundstage widening and true-peak protection" }
                }

                // Stereo Width Slider
                div {
                    div { style: "display: flex; justify-content: space-between; font-size: 12px; margin-bottom: 6px;",
                        span { style: "color: var(--text-secondary);", "Stereo Field Width (0.0x Mono → 2.0x Ultra-Wide)" }
                        span { style: "font-family: monospace; color: var(--accent-mint); font-weight: 600;", "{p.stereo_width:.2}x" }
                    }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "200",
                        value: "{(p.stereo_width * 100.0) as i32}",
                        style: "width: 100%; accent-color: var(--accent-mint);",
                        oninput: move |evt| {
                            if let Ok(v) = evt.value().parse::<f32>() {
                                let mut next = local_params.read().clone();
                                next.stereo_width = v / 100.0;
                                local_params.set(next.clone());
                                on_change.call(next);
                            }
                        },
                    }
                }

                // Limiter Ceiling Slider
                div {
                    div { style: "display: flex; justify-content: space-between; font-size: 12px; margin-bottom: 6px;",
                        span { style: "color: var(--text-secondary);", "Brickwall Limiter Ceiling (True-Peak)" }
                        span { style: "font-family: monospace; color: var(--accent-mint);", "{p.limiter_ceiling_db:.2} dBFS" }
                    }
                    input {
                        r#type: "range",
                        min: "1",
                        max: "10",
                        value: "{(-p.limiter_ceiling_db * 10.0) as i32}",
                        style: "width: 100%; accent-color: var(--accent-mint);",
                        oninput: move |evt| {
                            if let Ok(v) = evt.value().parse::<f32>() {
                                let mut next = local_params.read().clone();
                                next.limiter_ceiling_db = -v / 10.0;
                                local_params.set(next.clone());
                                on_change.call(next);
                            }
                        },
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_compute_eq_curve_paths_flat() {
        let gains = [0.0; EQ_BANDS_COUNT];
        let (stroke, fill, points) = compute_eq_curve_paths(&gains);
        assert_eq!(points.len(), EQ_BANDS_COUNT);
        for p in points {
            assert!((p.1 - 60.0).abs() < 1e-4, "Flat points must be at y=60.0");
        }
        assert!(stroke.starts_with("M 0,60.0 L 50,60.0"));
        assert!(stroke.ends_with("L 1000,60.0"));
        assert!(fill.ends_with("L 1000,60.0 L 0,60.0 Z"));
    }

    #[test]
    fn test_compute_eq_curve_paths_boost_and_cut() {
        let mut gains = [0.0; EQ_BANDS_COUNT];
        gains[0] = 12.0;  // Max boost -> y = 12.0
        gains[9] = -12.0; // Max cut -> y = 108.0
        let (_stroke, _fill, points) = compute_eq_curve_paths(&gains);
        assert!((points[0].1 - 12.0).abs() < 1e-4);
        assert!((points[9].1 - 108.0).abs() < 1e-4);
    }

    #[test]
    fn test_preset_application() {
        let mut params = MasterFxParams::default();
        Preset::BassBoost.apply(&mut params);
        assert!(params.bass_enabled);
        assert!(params.band_gains_db[0] > 0.0);

        Preset::Flat.apply(&mut params);
        assert_eq!(params.band_gains_db, [0.0; EQ_BANDS_COUNT]);
        assert!(!params.bass_enabled);
        assert!(!params.clarity_enabled);
    }
}

