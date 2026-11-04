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

    rsx! {
        div { class: "equalizer-stage",
            style: "display: flex; flex-direction: column; gap: 24px; padding-bottom: 40px; max-width: 900px; margin: 0 auto; width: 100%;",

            // Stage Header
            div { style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-glass); padding-bottom: 16px;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700;", "Equalizer & Master FX" }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                        "Studio-Grade 6-Stage Mastering Rack • Real-Time DSP Audio Processing"
                    }
                }
                div { style: "display: flex; gap: 12px; align-items: center;",
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
                    button {
                        class: if p.enabled { "btn-pill active" } else { "btn-pill" },
                        style: if !p.enabled { "opacity: 0.6;" } else { "" },
                        onclick: move |_| {
                            let mut next = local_params.read().clone();
                            next.enabled = !next.enabled;
                            local_params.set(next.clone());
                            on_change.call(next);
                        },
                        if p.enabled { "Master FX: ACTIVE" } else { "Master FX: BYPASSED" }
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

            // Card 2: 10-Band Graphic Equalizer (Sliders Rack)
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 16px; border: 1px solid var(--border-glass);",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;",
                    div {
                        h3 { style: "font-size: 16px; font-weight: 600;", "10-Band Graphic Equalizer" }
                        p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;", "Studio peaking IIR filters (-12 dB to +12 dB range)" }
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

                // 10 vertical sliders
                div { style: "display: flex; justify-content: space-between; align-items: flex-end; gap: 8px; height: 200px; padding: 10px 10px 24px 10px; background: rgba(0,0,0,0.25); border-radius: 12px; border: 1px solid rgba(255,255,255,0.04);",
                    for (band_idx, &freq) in EQ_CENTER_FREQS.iter().enumerate() {
                        {
                            let gain = p.band_gains_db[band_idx];
                            let freq_label = format_freq(freq);
                            let gain_str = format!("{gain:+.1}");

                            rsx! {
                                div {
                                    key: "{band_idx}",
                                    style: "display: flex; flex-direction: column; align-items: center; justify-content: space-between; height: 100%; flex: 1;",
                                    span { style: "font-size: 10px; font-family: monospace; color: if gain.abs() > 0.1 { \"var(--accent-mint)\" } else { \"var(--text-muted)\" }; font-weight: 600;",
                                        "{gain_str}"
                                    }
                                    div { style: "flex: 1; display: flex; align-items: center; justify-content: center; width: 100%; position: relative; margin: 8px 0;",
                                        input {
                                            r#type: "range",
                                            min: "-120",
                                            max: "120",
                                            value: "{(gain * 10.0) as i32}",
                                            style: "transform: rotate(-90deg); width: 110px; cursor: pointer; accent-color: var(--accent-mint);",
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
                                    span { style: "font-size: 11px; color: var(--text-secondary); font-weight: 600; margin-top: 4px;",
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
                        button {
                            class: if p.bass_enabled { "btn-pill active" } else { "btn-pill" },
                            style: "font-size: 11px; padding: 4px 10px;",
                            onclick: move |_| {
                                let mut next = local_params.read().clone();
                                next.bass_enabled = !next.bass_enabled;
                                local_params.set(next.clone());
                                on_change.call(next);
                            },
                            if p.bass_enabled { "ENABLED" } else { "OFF" }
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
                        button {
                            class: if p.clarity_enabled { "btn-pill active" } else { "btn-pill" },
                            style: "font-size: 11px; padding: 4px 10px;",
                            onclick: move |_| {
                                let mut next = local_params.read().clone();
                                next.clarity_enabled = !next.clarity_enabled;
                                local_params.set(next.clone());
                                on_change.call(next);
                            },
                            if p.clarity_enabled { "ENABLED" } else { "OFF" }
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
