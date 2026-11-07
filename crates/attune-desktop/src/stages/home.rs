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
    #[props(default)]
    all_tracks: Vec<TrackEntry>,
    on_play_track: EventHandler<TrackEntry>,
    #[props(default)]
    on_play_tracks: EventHandler<Vec<TrackEntry>>,
    #[props(default)]
    on_queue_track: EventHandler<TrackEntry>,
    #[props(default)]
    on_select_artist: EventHandler<String>,
    on_toggle_play: EventHandler<()>,
) -> Element {
    let username = std::env::var("USER").unwrap_or_else(|_| "Listener".to_string());

    let fav_catalog = crate::artist_profile::find_favorite_artist(&recent_tracks, &all_tracks);
    let mut wiki_artist_image = use_signal(|| None::<String>);
    let fav_artist_name = fav_catalog.as_ref().map(|c| c.artist.clone());

    use_effect(move || {
        if let Some(artist) = fav_artist_name.clone() {
            spawn(async move {
                let img = tokio::task::spawn_blocking(move || {
                    crate::artist_profile::fetch_wikipedia_artist_image(&artist)
                })
                .await
                .ok()
                .flatten();
                if let Some(img_url) = img {
                    wiki_artist_image.set(Some(img_url));
                }
            });
        }
    });

    let effective_image_url = wiki_artist_image
        .read()
        .clone()
        .or_else(|| fav_catalog.as_ref().and_then(|f| f.fallback_artwork_url.clone()));
    
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
                    let key_label = if let Some(k) = track.key.as_deref().filter(|k| !k.is_empty() && *k != "—" && *k != "Unknown") {
                        if let Some(cam) = crate::smart_shuffle::normalize_to_camelot(k) {
                            if k.to_uppercase() != cam {
                                format!("{k} ({cam})")
                            } else {
                                k.to_string()
                            }
                        } else {
                            k.to_string()
                        }
                    } else {
                        "—".to_string()
                    };
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
                                    style: "display: flex; align-items: center; justify-content: center; background: linear-gradient(135deg, rgba(45,212,191,0.2), rgba(168,85,247,0.2));",
                                    span { class: "material-icons md-48", "music_note" }
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
                                    style: "width: 48px; height: 48px;",
                                    onclick: move |_| on_toggle_play.call(()),
                                    if is_playing {
                                        span { class: "material-icons md-28", "pause" }
                                    } else {
                                        span { class: "material-icons md-28", "play_arrow" }
                                    }
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

            // Favorite Artist Algorithmic Catalog Card ("Top Picks for You")
            if let Some(fav) = fav_catalog.as_ref() {
                {
                    let artist_name = fav.artist.clone();
                    let track_count = fav.tracks.len();
                    let dur_str = fav.total_duration_str.clone();
                    let rationale = fav.rationale.clone();
                    let tracks_for_play = fav.tracks.clone();
                    let tracks_for_queue = fav.tracks.clone();
                    let art_url = effective_image_url.clone();

                    rsx! {
                        div { class: "fav-artist-section", style: "margin-top: 32px; margin-bottom: 32px;",
                            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px;",
                                h3 { style: "font-size: 18px; font-weight: 700; display: flex; align-items: center; gap: 8px;",
                                    span { class: "material-icons md-20", style: "color: var(--accent-mint);", "auto_awesome" }
                                    "Top Picks for You"
                                }
                                span { style: "font-size: 12px; color: var(--text-muted);", "Algorithmic bundling" }
                            }

                            div { class: "fav-artist-card",
                                // SVG Artwork with half-opacity artist image
                                div { class: "fav-artist-art-box",
                                    {crate::artist_profile::render_favorite_artist_svg(&artist_name, art_url.as_deref())}
                                }

                                // Information & Rationale
                                div { class: "fav-artist-content",
                                    div { class: "fav-artist-tag-row",
                                        span { class: "fav-artist-badge", "MADE FOR YOU • ARTIST CATALOG" }
                                    }
                                    {
                                        let a_click = artist_name.clone();
                                        rsx! {
                                            h2 {
                                                class: "fav-artist-title",
                                                style: "cursor: pointer;",
                                                title: "View {artist_name} catalog",
                                                onclick: move |_| on_select_artist.call(a_click.clone()),
                                                "Your favourite {artist_name} catalogs."
                                            }
                                        }
                                    }
                                    p { class: "fav-artist-rationale", "{rationale}" }

                                    div { class: "fav-artist-meta",
                                        span { class: "fav-artist-count", "{track_count} songs bundled" }
                                        span { class: "fav-artist-dot", "•" }
                                        span { class: "fav-artist-dur", "{dur_str}" }
                                    }

                                    // Action buttons
                                    div { class: "fav-artist-actions",
                                        button {
                                            class: "btn-fav-play",
                                            title: "Play bundled catalog",
                                            onclick: move |_| on_play_tracks.call(tracks_for_play.clone()),
                                            span { class: "material-icons md-20", "play_arrow" }
                                            span { "Play Catalog" }
                                        }
                                        button {
                                            class: "btn-fav-queue",
                                            title: "Add all songs to Queue",
                                            onclick: move |_| {
                                                for t in &tracks_for_queue {
                                                    on_queue_track.call(t.clone());
                                                }
                                            },
                                            span { class: "material-icons md-18", "playlist_add" }
                                            span { "Add to Queue" }
                                        }
                                        {
                                            let a_btn = artist_name.clone();
                                            rsx! {
                                                button {
                                                    class: "btn-fav-queue",
                                                    title: "View Artist Discography & Profile",
                                                    onclick: move |_| on_select_artist.call(a_btn.clone()),
                                                    span { class: "material-icons md-18", "person" }
                                                    span { "View Artist" }
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
                                                style: "display: flex; align-items: center; justify-content: center; color: var(--text-muted);",
                                                span { class: "material-icons md-20", "music_note" }
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
                                                style: "display: flex; align-items: center; justify-content: center; color: var(--text-muted);",
                                                span { class: "material-icons md-20", "album" }
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
