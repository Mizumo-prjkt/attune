use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};

#[component]
pub fn Home(
    current_track: Option<TrackEntry>,
    is_playing: bool,
    playback_pos_ms: u64,
    playback_dur_ms: u64,
    recent_tracks: Vec<TrackEntry>,
    harmonic_matches: Vec<TrackEntry>,
    on_play_track: EventHandler<TrackEntry>,
    on_toggle_play: EventHandler<()>,
) -> Element {
    let username = std::env::var("USER").unwrap_or_else(|_| "Listener".to_string());
    
    let greeting = {
        use std::time::SystemTime;
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let hour = ((secs % 86400) / 3600 + 8) % 24; // Approximate local time
        if hour < 12 {
            "Good morning"
        } else if hour < 18 {
            "Good afternoon"
        } else {
            "Good evening"
        }
    };

    rsx! {
        div { class: "home-stage",
            // Header with greeting (redundant Live Lyrics button removed)
            div { class: "home-header",
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;",
                div {
                    h1 { style: "font-size: 28px; font-weight: 700; letter-spacing: -0.02em;",
                        "{greeting}, {username}"
                    }
                    p { style: "color: var(--text-secondary); font-size: 13.5px; margin-top: 4px;",
                        "Attune Dynamic DJ Transition Engine is active"
                    }
                }
            }

            // Hero Now Playing Liquid Glass Card
            if let Some(track) = current_track.as_ref() {
                {
                    let pos_str = fmt_ms(playback_pos_ms);
                    let dur_str = fmt_ms(playback_dur_ms);
                    let progress_pct = if playback_dur_ms > 0 {
                        ((playback_pos_ms as f64 / playback_dur_ms as f64) * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    };
                    let key_label = track.key.as_deref().unwrap_or("—");
                    let bpm_label = track.bpm.map(|b| format!("{:.1}", b)).unwrap_or_else(|| "—".to_string());
                    let art_data = track.artwork_data_url.as_deref().unwrap_or_default();

                    rsx! {
                        div { class: "hero-card",
                            // Album Art using Data URL for 100% reliable rendering without CORS blocks
                            if !art_data.is_empty() {
                                img {
                                    class: "hero-art",
                                    src: "{art_data}",
                                    alt: "{track.title}",
                                }
                            } else {
                                div {
                                    class: "hero-art",
                                    style: "display: flex; align-items: center; justify-content: center; background: linear-gradient(135deg, rgba(45,212,191,0.2), rgba(168,85,247,0.2)); font-size: 36px;",
                                    "🎵"
                                }
                            }

                            // Track Info & Progress
                            div { class: "hero-info",
                                div { style: "display: flex; align-items: center; gap: 8px;",
                                    span { class: "hero-tag", "NOW PLAYING" }
                                }
                                h2 { class: "hero-title", "{track.title}" }
                                p { class: "hero-artist", "{track.artist} • {track.album}" }

                                // Profile Metadata Chips
                                div { style: "display: flex; gap: 8px; margin: 6px 0 10px 0; font-size: 11.5px;",
                                    div { style: "background: rgba(255,255,255,0.06); padding: 3px 8px; border-radius: 6px; border: 1px solid var(--border-glass);",
                                        "Key: " span { style: "color: var(--accent-mint); font-weight: 600;", "{key_label}" }
                                    }
                                    div { style: "background: rgba(255,255,255,0.06); padding: 3px 8px; border-radius: 6px; border: 1px solid var(--border-glass);",
                                        "BPM: " span { style: "color: var(--accent-mint); font-weight: 600;", "{bpm_label}" }
                                    }
                                    div { style: "background: rgba(255,255,255,0.06); padding: 3px 8px; border-radius: 6px; border: 1px solid var(--border-glass);",
                                        "Format: " span { style: "color: var(--text-secondary);", "{track.format}" }
                                    }
                                }

                                // Mini Progress Bar
                                div { style: "display: flex; align-items: center; gap: 10px; width: 100%;",
                                    span { style: "font-family: monospace; font-size: 11px; color: var(--text-muted);", "{pos_str}" }
                                    div { style: "flex: 1; height: 4px; background: rgba(255,255,255,0.12); border-radius: 2px; overflow: hidden;",
                                        div { style: "width: {progress_pct}%; height: 100%; background: var(--accent-mint); border-radius: 2px; transition: width 0.1s linear;" }
                                    }
                                    span { style: "font-family: monospace; font-size: 11px; color: var(--text-muted);", "{dur_str}" }
                                }
                            }

                            // Quick Play Control
                            div { style: "display: flex; align-items: center;",
                                button {
                                    class: "btn-ctrl btn-play",
                                    style: "width: 48px; height: 48px; font-size: 20px;",
                                    onclick: move |_| on_toggle_play.call(()),
                                    if is_playing { "⏸" } else { "▶" }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "hero-card",
                    style: "align-items: center; justify-content: center; padding: 36px;",
                    div { style: "text-align: center;",
                        p { style: "font-size: 18px; font-weight: 600; color: var(--text-secondary);", "No track currently playing" }
                        p { style: "font-size: 13px; color: var(--text-muted); margin-top: 6px;", "Pick a track from Harmonic Matches or your Library below to start listening." }
                    }
                }
            }

            // Harmonic Matches Section
            div { style: "margin-top: 32px; margin-bottom: 32px;",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    h3 { style: "font-size: 18px; font-weight: 700;", "Harmonic Matches (Camelot Compatible)" }
                    span { style: "font-size: 12px; color: var(--text-muted);", "Optimal for seamless DJ transitions" }
                }

                if harmonic_matches.is_empty() {
                    div { style: "background: var(--bg-card); border: 1px solid var(--border-glass); border-radius: 12px; padding: 20px; text-align: center; color: var(--text-muted);",
                        "Scan your music library to generate harmonic transition pairings."
                    }
                } else {
                    div { class: "matches-grid",
                        for track in harmonic_matches.iter() {
                            {
                                let t = track.clone();
                                let t_click = t.clone();
                                let art = t.artwork_data_url.as_deref().unwrap_or_default();
                                let key = t.key.as_deref().unwrap_or("—");
                                let bpm = t.bpm.map(|b| format!("{:.0} BPM", b)).unwrap_or_else(|| "—".to_string());

                                rsx! {
                                    div {
                                        key: "{t.path.display()}",
                                        class: "match-tile",
                                        onclick: move |_| on_play_track.call(t_click.clone()),
                                        if !art.is_empty() {
                                            img { class: "match-thumb", src: "{art}", alt: "{t.title}" }
                                        } else {
                                            div {
                                                class: "match-thumb",
                                                style: "display: flex; align-items: center; justify-content: center; font-size: 20px;",
                                                "🎵"
                                            }
                                        }
                                        div { style: "display: flex; flex-direction: column; overflow: hidden; flex: 1;",
                                            span { style: "font-weight: 600; font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{t.title}" }
                                            span { style: "font-size: 11.5px; color: var(--text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{t.artist}" }
                                            div { style: "display: flex; gap: 6px; align-items: center; margin-top: 4px;",
                                                span { style: "font-size: 10px; color: var(--accent-mint); font-weight: 600;", "{key}" }
                                                span { style: "font-size: 10px; color: var(--text-muted);", "•" }
                                                span { style: "font-size: 10px; color: var(--text-muted);", "{bpm}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Recently Discovered / Played Section
            div { style: "margin-bottom: 32px;",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    h3 { style: "font-size: 18px; font-weight: 700;", "Recently Played & Library Highlights" }
                    span { style: "font-size: 12px; color: var(--text-muted);", "{recent_tracks.len()} tracks" }
                }

                if recent_tracks.is_empty() {
                    div { style: "background: var(--bg-card); border: 1px solid var(--border-glass); border-radius: 12px; padding: 20px; text-align: center; color: var(--text-muted);",
                        "Your library is empty. Click [Choose] in the sidebar to select your music directory."
                    }
                } else {
                    div { class: "matches-grid",
                        for track in recent_tracks.iter().take(8) {
                            {
                                let t = track.clone();
                                let t_click = t.clone();
                                let art = t.artwork_data_url.as_deref().unwrap_or_default();
                                let dur = fmt_ms(t.duration_ms);

                                rsx! {
                                    div {
                                        key: "{t.path.display()}",
                                        class: "match-tile",
                                        onclick: move |_| on_play_track.call(t_click.clone()),
                                        if !art.is_empty() {
                                            img { class: "match-thumb", src: "{art}", alt: "{t.title}" }
                                        } else {
                                            div {
                                                class: "match-thumb",
                                                style: "display: flex; align-items: center; justify-content: center; font-size: 20px;",
                                                "💿"
                                            }
                                        }
                                        div { style: "display: flex; flex-direction: column; overflow: hidden; flex: 1;",
                                            span { style: "font-weight: 600; font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{t.title}" }
                                            span { style: "font-size: 11.5px; color: var(--text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{t.artist}" }
                                            div { style: "display: flex; gap: 6px; align-items: center; margin-top: 4px;",
                                                span { style: "font-size: 10px; color: var(--text-muted); font-family: monospace;", "{dur}" }
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
