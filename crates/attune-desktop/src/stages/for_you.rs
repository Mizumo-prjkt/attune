use dioxus::prelude::*;
use crate::library::TrackEntry;

#[derive(Debug, Clone, PartialEq)]
pub struct AlgorithmicMix {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub gradient: &'static str,
    pub vibe_tags: Vec<&'static str>,
}

#[component]
pub fn ForYou(
    tracks: Vec<TrackEntry>,
    on_start_mix: EventHandler<Vec<TrackEntry>>,
) -> Element {
    let mixes = vec![
        AlgorithmicMix {
            id: "harmonic_flow",
            title: "Harmonic Flow",
            subtitle: "Camelot Wheel Continuous Mix",
            description: "Sequences tracks along adjacent Camelot keys (1A → 2A → 2B) for pitch-perfect, in-key harmonic blends without tonal clashes.",
            icon: "🪐",
            gradient: "linear-gradient(135deg, rgba(45, 212, 191, 0.25), rgba(59, 130, 246, 0.25))",
            vibe_tags: vec!["Harmonic Wheel", "Key-Locked", "Seamless Morph"],
        },
        AlgorithmicMix {
            id: "high_energy",
            title: "High-Energy Rush",
            subtitle: "Aggression & Drive Index 𝒪(𝒜 ≥ 0.65)",
            description: "High-BPM drives, J-Rock, Drum & Bass, and Synthwave. Uses Attune's fast 2-bar Cold Cut and EQ Swap archetypes.",
            icon: "⚡",
            gradient: "linear-gradient(135deg, rgba(251, 113, 133, 0.25), rgba(249, 115, 22, 0.25))",
            vibe_tags: vec!["Fast Swaps", "Aggression ≥ 0.65", "Bass Cut"],
        },
        AlgorithmicMix {
            id: "midnight_chill",
            title: "Midnight Chill",
            subtitle: "Downtempo & Ambient Vocal Sustains",
            description: "Late-night atmosphere, lo-fi, and warm vocals. Employs Tail Freeze and Sustained Vocal Bridge morphs.",
            icon: "🌙",
            gradient: "linear-gradient(135deg, rgba(168, 85, 247, 0.25), rgba(99, 102, 241, 0.25))",
            vibe_tags: vec!["Vocal Sustain", "Tail Freeze", "Downtempo"],
        },
    ];

    rsx! {
        div { class: "for-you-stage",
            div { style: "margin-bottom: 24px;",
                h1 { style: "font-size: 26px; font-weight: 700;", "For You — Smart Algorithmic Mixes" }
                p { style: "color: var(--text-secondary); font-size: 13.5px; margin-top: 4px;",
                    "Multi-dimensional DJ recipes generated directly from your local music collection."
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 20px;",
                for mix in mixes.into_iter() {
                    {
                        let mix_tracks = match mix.id {
                            "high_energy" => {
                                let high: Vec<TrackEntry> = tracks.iter()
                                    .filter(|t| t.bpm.unwrap_or(0.0) >= 135.0 || t.format == "FLAC")
                                    .cloned()
                                    .collect();
                                if high.is_empty() { tracks.clone() } else { high }
                            },
                            "midnight_chill" => {
                                let chill: Vec<TrackEntry> = tracks.iter()
                                    .filter(|t| t.bpm.unwrap_or(120.0) < 125.0)
                                    .cloned()
                                    .collect();
                                if chill.is_empty() { tracks.clone() } else { chill }
                            },
                            _ => tracks.clone(),
                        };
                        let track_count = mix_tracks.len();

                        rsx! {
                            div {
                                key: "{mix.id}",
                                style: "display: flex; gap: 24px; background: var(--bg-card); border: 1px solid var(--border-glass); border-radius: 18px; padding: 24px; backdrop-filter: blur(24px); position: relative; overflow: hidden;",
                                
                                // Ambient gradient backdrop accent
                                div {
                                    style: "position: absolute; inset: 0; background: {mix.gradient}; opacity: 0.4; pointer-events: none;",
                                }

                                // Mix Icon
                                div {
                                    style: "position: relative; z-index: 1; width: 100px; height: 100px; border-radius: 16px; background: rgba(255,255,255,0.08); display: flex; align-items: center; justify-content: center; font-size: 44px; border: 1px solid var(--border-glass); flex-shrink: 0;",
                                    "{mix.icon}"
                                }

                                // Mix Info
                                div {
                                    style: "position: relative; z-index: 1; display: flex; flex-direction: column; justify-content: center; flex: 1;",
                                    span { style: "color: var(--accent-mint); font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;", "{mix.subtitle}" }
                                    h2 { style: "font-size: 22px; font-weight: 700; margin: 2px 0 6px 0;", "{mix.title}" }
                                    p { style: "color: var(--text-secondary); font-size: 13.5px; line-height: 1.4; max-width: 650px;", "{mix.description}" }

                                    div { style: "display: flex; gap: 8px; margin-top: 12px; align-items: center;",
                                        for tag in mix.vibe_tags {
                                            span {
                                                style: "background: rgba(255,255,255,0.08); padding: 3px 8px; border-radius: 6px; font-size: 11px; color: var(--text-secondary); border: 1px solid var(--border-glass);",
                                                "{tag}"
                                            }
                                        }
                                        span { style: "font-size: 12px; color: var(--text-muted); margin-left: 6px;", "{track_count} eligible tracks" }
                                    }
                                }

                                // Start Button
                                div {
                                    style: "position: relative; z-index: 1; display: flex; align-items: center;",
                                    button {
                                        class: "btn-pill active",
                                        style: "padding: 10px 22px; font-size: 14px; font-weight: 600;",
                                        onclick: move |_| on_start_mix.call(mix_tracks.clone()),
                                        "▶ Start Mix"
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
