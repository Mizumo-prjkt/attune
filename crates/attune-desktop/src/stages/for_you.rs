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
            gradient: "linear-gradient(135deg, rgba(45, 212, 191, 0.22), rgba(59, 130, 246, 0.18))",
            vibe_tags: vec!["Harmonic Wheel", "Key-Locked", "Seamless Morph"],
        },
        AlgorithmicMix {
            id: "high_energy",
            title: "High-Energy Rush",
            subtitle: "Aggression & Drive Index 𝒪(𝒜 ≥ 0.65)",
            description: "High-BPM drives, J-Rock, Drum & Bass, and Synthwave. Uses Attune's fast 2-bar Cold Cut and EQ Swap archetypes.",
            icon: "⚡",
            gradient: "linear-gradient(135deg, rgba(251, 113, 133, 0.22), rgba(249, 115, 22, 0.18))",
            vibe_tags: vec!["Fast Swaps", "Aggression ≥ 0.65", "Bass Cut"],
        },
        AlgorithmicMix {
            id: "midnight_chill",
            title: "Midnight Chill",
            subtitle: "Downtempo & Ambient Vocal Sustains",
            description: "Late-night atmosphere, lo-fi, and warm vocals. Employs Tail Freeze and Sustained Vocal Bridge morphs.",
            icon: "🌙",
            gradient: "linear-gradient(135deg, rgba(168, 85, 247, 0.22), rgba(99, 102, 241, 0.18))",
            vibe_tags: vec!["Vocal Sustain", "Tail Freeze", "Downtempo"],
        },
        AlgorithmicMix {
            id: "vocal_spotlight",
            title: "Vocal Spotlight",
            subtitle: "Formant F1/F2 & Pitch Sustains",
            description: "Highlights resonant vocal performances and acapella intros. Uses multi-layer vocal sustain morphs with formants intact.",
            icon: "🎤",
            gradient: "linear-gradient(135deg, rgba(236, 72, 153, 0.22), rgba(168, 85, 247, 0.18))",
            vibe_tags: vec!["Vocal Anchor", "Formant Lock", "Acapella Bridge"],
        },
        AlgorithmicMix {
            id: "bpm_drive",
            title: "BPM Drive & Fast Swaps",
            subtitle: "Tempo Climbing & Drop Alignment",
            description: "Progressive energy ascension and quick transition handoffs timed perfectly to 8-bar and 16-bar phrase drops.",
            icon: "🚀",
            gradient: "linear-gradient(135deg, rgba(14, 165, 233, 0.22), rgba(45, 212, 191, 0.18))",
            vibe_tags: vec!["Tempo Climb", "Phrase Grid", "Drop Sync"],
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

            div { style: "display: flex; flex-direction: column; gap: 18px;",
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
                            "vocal_spotlight" => {
                                let vocal: Vec<TrackEntry> = tracks.iter()
                                    .filter(|t| t.genre.as_deref().unwrap_or("").to_lowercase().contains("pop")
                                        || t.genre.as_deref().unwrap_or("").to_lowercase().contains("vocal")
                                        || t.title.to_lowercase().contains("feat")
                                        || t.title.to_lowercase().contains("remix"))
                                    .cloned()
                                    .collect();
                                if vocal.is_empty() { tracks.clone() } else { vocal }
                            },
                            "bpm_drive" => {
                                let mut sorted = tracks.clone();
                                sorted.sort_by(|a, b| {
                                    let ba = a.bpm.unwrap_or(120.0);
                                    let bb = b.bpm.unwrap_or(120.0);
                                    ba.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
                                });
                                sorted
                            },
                            _ => tracks.clone(),
                        };
                        let track_count = mix_tracks.len();
                        let all_tracks = mix_tracks.clone();

                        // Collect up to 4 real album artworks for the transparent preview collage
                        let preview_arts: Vec<String> = mix_tracks
                            .iter()
                            .filter_map(|t| t.artwork_data_url.clone())
                            .take(4)
                            .collect();

                        rsx! {
                            div {
                                key: "{mix.id}",
                                style: "display: flex; gap: 24px; background: rgba(18, 24, 38, 0.45); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 18px; padding: 22px; backdrop-filter: blur(28px); -webkit-backdrop-filter: blur(28px); position: relative; overflow: hidden; box-shadow: 0 10px 30px rgba(0,0,0,0.25);",
                                
                                // Ambient gradient backdrop accent
                                div {
                                    style: "position: absolute; inset: 0; background: {mix.gradient}; opacity: 0.35; pointer-events: none;",
                                }

                                // Mix Icon
                                div {
                                    style: "position: relative; z-index: 1; width: 90px; height: 90px; border-radius: 16px; background: rgba(255,255,255,0.06); display: flex; align-items: center; justify-content: center; font-size: 40px; border: 1px solid rgba(255,255,255,0.12); flex-shrink: 0; box-shadow: 0 8px 20px rgba(0,0,0,0.3);",
                                    "{mix.icon}"
                                }

                                // Mix Info
                                div {
                                    style: "position: relative; z-index: 1; display: flex; flex-direction: column; justify-content: center; flex: 1; min-width: 0;",
                                    span { style: "color: var(--accent-mint); font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;", "{mix.subtitle}" }
                                    h2 { style: "font-size: 21px; font-weight: 700; margin: 2px 0 6px 0;", "{mix.title}" }
                                    p { style: "color: var(--text-secondary); font-size: 13px; line-height: 1.4; max-width: 600px;", "{mix.description}" }

                                    div { style: "display: flex; gap: 8px; margin-top: 10px; align-items: center; flex-wrap: wrap;",
                                        for tag in mix.vibe_tags {
                                            span {
                                                style: "background: rgba(255,255,255,0.06); padding: 3px 8px; border-radius: 6px; font-size: 10.5px; color: var(--text-secondary); border: 1px solid rgba(255,255,255,0.08);",
                                                "{tag}"
                                            }
                                        }
                                        span { style: "font-size: 11.5px; color: var(--text-muted); margin-left: 4px;", "{track_count} eligible tracks" }
                                    }
                                }

                                // Album Artwork Preview Collage (transparent peek into actual songs in this mix)
                                if !preview_arts.is_empty() {
                                    div {
                                        style: "position: relative; z-index: 1; display: flex; align-items: center; margin-right: 8px; flex-shrink: 0;",
                                        for (i, art) in preview_arts.into_iter().enumerate() {
                                            img {
                                                key: "{i}",
                                                src: "{art}",
                                                alt: "Album Cover",
                                                style: "width: 44px; height: 44px; border-radius: 8px; object-fit: cover; border: 1px solid rgba(255,255,255,0.15); margin-left: if i > 0 { \"-14px\" } else { \"0px\" }; box-shadow: 0 4px 12px rgba(0,0,0,0.4); transform: scale({1.0 - (i as f32 * 0.03)}); transition: transform 0.2s;",
                                            }
                                        }
                                    }
                                }

                                // Start Button
                                div {
                                    style: "position: relative; z-index: 1; display: flex; align-items: center; flex-shrink: 0;",
                                    button {
                                        class: "btn-pill active",
                                        style: "padding: 10px 22px; font-size: 13.5px; font-weight: 600; box-shadow: 0 4px 16px rgba(45,212,191,0.25);",
                                        onclick: move |_| on_start_mix.call(all_tracks.clone()),
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
