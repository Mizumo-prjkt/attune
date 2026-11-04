use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumGroup {
    pub name: String,
    pub artist: String,
    pub artwork_path: Option<PathBuf>,
    pub artwork_data_url: Option<String>,
    pub tracks: Vec<TrackEntry>,
}

#[component]
pub fn Albums(
    tracks: Vec<TrackEntry>,
    current_track_path: Option<PathBuf>,
    is_playing: bool,
    on_play_track: EventHandler<TrackEntry>,
    on_play_tracks: EventHandler<Vec<TrackEntry>>,
    on_queue_track: EventHandler<TrackEntry>,
    on_edit_album: EventHandler<AlbumGroup>,
) -> Element {
    let mut selected_album = use_signal(|| None::<String>);

    // Group tracks by album
    let mut album_map: HashMap<String, AlbumGroup> = HashMap::new();
    for track in tracks {
        let entry = album_map.entry(track.album.clone()).or_insert_with(|| AlbumGroup {
            name: track.album.clone(),
            artist: track.artist.clone(),
            artwork_path: track.artwork_path.clone(),
            artwork_data_url: track.artwork_data_url.clone(),
            tracks: Vec::new(),
        });
        if entry.artwork_path.is_none() && track.artwork_path.is_some() {
            entry.artwork_path = track.artwork_path.clone();
        }
        if entry.artwork_data_url.is_none() && track.artwork_data_url.is_some() {
            entry.artwork_data_url = track.artwork_data_url.clone();
        }
        entry.tracks.push(track);
    }

    let mut album_list: Vec<AlbumGroup> = album_map.into_values().collect();
    album_list.sort_by(|a, b| a.name.cmp(&b.name));

    let selected_name = selected_album.read().clone();
    let selected_group = selected_name.as_ref().and_then(|name| album_list.iter().find(|a| &a.name == name).cloned());

    rsx! {
        div { class: "albums-stage",
            if let Some(group) = selected_group {
                {
                    let track_count = group.tracks.len();
                    let lore_description = format_album_lore(&group);
                    let art_data_url = group.artwork_data_url.clone();

                    rsx! {
                        div { style: "display: flex; flex-direction: column; gap: 20px; width: 100%; max-width: 960px; margin: 0 auto; padding-bottom: 40px;",
                            // 1. Top Splash Backdrop Banner
                            div { class: "album-hero-banner",
                                if let Some(art) = art_data_url.as_ref() {
                                    img {
                                        class: "album-hero-img",
                                        src: "{art}",
                                        alt: "{group.name}",
                                    }
                                } else {
                                    div { class: "album-hero-fallback", "💿" }
                                }
                                div { class: "album-hero-overlay" }

                                // Back button pill (Top Left): "← Collections"
                                button {
                                    class: "album-back-pill",
                                    onclick: move |_| selected_album.set(None),
                                    span { style: "font-size: 14px;", "←" }
                                    span { "Collections" }
                                }

                                // Prominent Title Overlay (Lower Banner Area)
                                div { class: "album-hero-meta",
                                    div { class: "album-type-badge", "ALBUM" }
                                    h1 { class: "album-hero-title", "{group.name}" }
                                }
                            }

                            // 2. Overlapping Floating Liquid Glass Actions Card
                            div { class: "album-actions-card",
                                // Top Header Row: "👤 By {artist}" & "{count} songs" & Edit button
                                div { class: "album-meta-header",
                                    div { style: "display: flex; align-items: center; gap: 8px;",
                                        span { style: "color: var(--accent-mint); font-size: 16px;", "👤" }
                                        span { style: "font-weight: 700; font-size: 16px; color: #ffffff;", "By {group.artist}" }
                                    }
                                    div { style: "display: flex; align-items: center; gap: 14px;",
                                        span { style: "color: var(--accent-mint); font-size: 13px; font-weight: 600;", "{track_count} songs" }
                                        button {
                                            class: "btn-icon-subtle",
                                            title: "Edit Album Metadata & Splash Art",
                                            onclick: {
                                                let g = group.clone();
                                                move |_| on_edit_album.call(g.clone())
                                            },
                                            "✏️"
                                        }
                                    }
                                }

                                // Bottom Button Row: M3 Split Pill [ ▶ Play Album | ▾ ], Shuffle, Edit
                                div { class: "album-button-row",
                                    div { class: "m3-split-pill",
                                        button {
                                            class: "m3-split-main",
                                            title: "Play entire album",
                                            onclick: {
                                                let all = group.tracks.clone();
                                                move |_| on_play_tracks.call(all.clone())
                                            },
                                            span { "▶" }
                                            span { "Play Album" }
                                        }
                                        div { class: "m3-split-divider" }
                                        button {
                                            class: "m3-split-dropdown",
                                            title: "Quick shuffle",
                                            onclick: {
                                                let mut shuffled = group.tracks.clone();
                                                shuffled.reverse();
                                                move |_| on_play_tracks.call(shuffled.clone())
                                            },
                                            "▾"
                                        }
                                    }
                                    button {
                                        class: "btn-pill",
                                        onclick: {
                                            let mut shuffled = group.tracks.clone();
                                            shuffled.reverse();
                                            move |_| on_play_tracks.call(shuffled.clone())
                                        },
                                        "🔀 Shuffle Album"
                                    }
                                    button {
                                        class: "btn-pill",
                                        style: "border-color: rgba(45,212,191,0.3); color: var(--accent-mint);",
                                        onclick: {
                                            let g = group.clone();
                                            move |_| on_edit_album.call(g.clone())
                                        },
                                        "✏️ Edit Album Info"
                                    }
                                }
                            }

                            // 3. Rich Liner Notes / Description Card
                            div { class: "album-lore-card",
                                div { style: "display: flex; justify-content: space-between; align-items: center;",
                                    div { style: "display: flex; align-items: center; gap: 8px;",
                                        span { style: "font-size: 15px;", "📖" }
                                        span { class: "album-card-section-title", "ALBUM LINER NOTES & LORE" }
                                    }
                                    button {
                                        class: "btn-icon-subtle",
                                        title: "Edit Album Notes & Lore",
                                        onclick: {
                                            let g = group.clone();
                                            move |_| on_edit_album.call(g.clone())
                                        },
                                        "✏️"
                                    }
                                }
                                p { class: "album-lore-text",
                                    "{lore_description}"
                                }
                            }

                            // 4. Tracklist Section: Header + Track Cards
                            div { class: "album-tracklist-section",
                                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; padding: 0 4px;",
                                    span { class: "album-card-section-title", "TRACKLIST ({track_count})" }
                                    button {
                                        class: "btn-text-action",
                                        onclick: {
                                            let g = group.clone();
                                            move |_| on_edit_album.call(g.clone())
                                        },
                                        "+ Add Song"
                                    }
                                }

                                div { style: "display: flex; flex-direction: column; gap: 8px;",
                                    for (idx, track) in group.tracks.iter().enumerate() {
                                        {
                                            let t = track.clone();
                                            let dur = fmt_ms(t.duration_ms);
                                            let is_current = current_track_path.as_ref().map(|p| p == &t.path).unwrap_or(false);
                                            let t_for_play = t.clone();
                                            let t_for_menu = t.clone();
                                            let t_for_row = t.clone();

                                            rsx! {
                                                div {
                                                    key: "{t.path.display()}",
                                                    class: if is_current { "album-track-card active" } else { "album-track-card" },
                                                    ondoubleclick: move |_| on_play_track.call(t_for_row.clone()),

                                                    // Track number or playing indicator
                                                    span { class: "album-track-idx",
                                                        if is_current && is_playing {
                                                            "🔊"
                                                        } else {
                                                            "{idx + 1}"
                                                        }
                                                    }

                                                    // Track Title & Details
                                                    div { style: "display: flex; flex-direction: column; flex: 1; min-width: 0;",
                                                        span { class: if is_current { "album-track-title active" } else { "album-track-title" },
                                                            "{t.title}"
                                                        }
                                                        span { class: "album-track-sub",
                                                            "{t.artist} • {t.format}"
                                                            if let Some(bpm) = t.bpm {
                                                                " • {bpm:.0} BPM"
                                                            }
                                                            if let Some(ref k) = t.key {
                                                                " • {k}"
                                                            }
                                                        }
                                                    }

                                                    // Right-side actions: Duration, Menu, Play/Pause circle
                                                    div { style: "display: flex; align-items: center; gap: 10px;",
                                                        span { class: "album-track-dur", "{dur}" }
                                                        button {
                                                            class: "btn-track-menu",
                                                            title: "Track Actions / Add to Queue",
                                                            onclick: {
                                                                let t_q = t_for_menu.clone();
                                                                move |evt: MouseEvent| {
                                                                    evt.stop_propagation();
                                                                    on_queue_track.call(t_q.clone());
                                                                }
                                                            },
                                                            "⋮"
                                                        }
                                                        button {
                                                            class: if is_current && is_playing { "album-track-play-btn active" } else { "album-track-play-btn" },
                                                            title: if is_current && is_playing { "Pause" } else { "Play track" },
                                                            onclick: {
                                                                let t_p = t_for_play.clone();
                                                                move |evt: MouseEvent| {
                                                                    evt.stop_propagation();
                                                                    on_play_track.call(t_p.clone());
                                                                }
                                                            },
                                                            if is_current && is_playing { "⏸" } else { "▶" }
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
            } else {
                // Grid view of all albums
                div {
                    h1 { style: "font-size: 26px; font-weight: 700; margin-bottom: 6px;", "Albums" }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-bottom: 24px;",
                        "{album_list.len()} albums in your library"
                    }

                    if album_list.is_empty() {
                        div { style: "background: var(--bg-card); border: 1px solid var(--border-glass); border-radius: 12px; padding: 36px; text-align: center; color: var(--text-muted);",
                            "No albums discovered yet. Scan your music library to organize by album."
                        }
                    } else {
                        div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 20px;",
                            for album in album_list.iter() {
                                {
                                    let name = album.name.clone();
                                    let art = album.artwork_data_url.as_deref().unwrap_or_default();
                                    let track_count = album.tracks.len();

                                    rsx! {
                                        div {
                                            key: "{album.name}",
                                            style: "background: var(--bg-card); border: 1px solid var(--border-glass); border-radius: 14px; padding: 12px; cursor: pointer; transition: all 0.2s ease;",
                                            onclick: move |_| selected_album.set(Some(name.clone())),
                                            if !art.is_empty() {
                                                img {
                                                    src: "{art}",
                                                    alt: "{album.name}",
                                                    style: "width: 100%; aspect-ratio: 1; border-radius: 10px; object-fit: cover; margin-bottom: 10px; box-shadow: 0 4px 14px rgba(0,0,0,0.4);",
                                                }
                                            } else {
                                                div {
                                                    style: "width: 100%; aspect-ratio: 1; border-radius: 10px; background: rgba(255,255,255,0.05); display: flex; align-items: center; justify-content: center; font-size: 36px; margin-bottom: 10px; border: 1px solid var(--border-glass);",
                                                    "💿"
                                                }
                                            }
                                            div { style: "display: flex; flex-direction: column; overflow: hidden;",
                                                span { style: "font-weight: 600; font-size: 13.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{album.name}" }
                                                span { style: "color: var(--text-secondary); font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px;", "{album.artist}" }
                                                span { style: "color: var(--text-muted); font-size: 11px; margin-top: 4px;", "{track_count} songs" }
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

/// Computes the dynamic liner notes & lore summary for an album group.
pub fn format_album_lore(group: &AlbumGroup) -> String {
    let track_count = group.tracks.len();
    let lossless_count = group.tracks.iter().filter(|t| {
        let fmt = t.format.to_lowercase();
        fmt.contains("flac") || fmt.contains("wav") || fmt.contains("alac") || fmt.contains("aiff")
    }).count();

    let mut formats: Vec<String> = group.tracks.iter().map(|t| t.format.clone()).collect();
    formats.sort();
    formats.dedup();
    let formats_str = if formats.is_empty() { "Audio".to_string() } else { formats.join(", ") };

    format!(
        "Collection of {} tracks by {}. {} lossless tracks ({}). Tempo profiling pending audio transition scan. Harmonic chroma pending CQT analysis. Notes and background lore can be edited or fetched dynamically.",
        track_count,
        group.artist,
        lossless_count,
        formats_str
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_album_lore_generation() {
        let group = AlbumGroup {
            name: "Super Trouper".into(),
            artist: "ABBA".into(),
            artwork_path: None,
            artwork_data_url: None,
            tracks: vec![
                TrackEntry {
                    path: PathBuf::from("/music/track1.flac"),
                    title: "Lay All Your Love On Me".into(),
                    artist: "ABBA".into(),
                    album: "Super Trouper".into(),
                    genre: Some("Pop".into()),
                    duration_ms: 270000,
                    bpm: Some(125.0),
                    key: Some("11B".into()),
                    format: "FLAC".into(),
                    sample_rate: 44100,
                    bit_depth: 16,
                    artwork_path: None,
                    artwork_data_url: None,
                },
                TrackEntry {
                    path: PathBuf::from("/music/track2.flac"),
                    title: "The Winner Takes It All".into(),
                    artist: "ABBA".into(),
                    album: "Super Trouper".into(),
                    genre: Some("Pop".into()),
                    duration_ms: 295000,
                    bpm: Some(128.0),
                    key: Some("11B".into()),
                    format: "FLAC".into(),
                    sample_rate: 44100,
                    bit_depth: 16,
                    artwork_path: None,
                    artwork_data_url: None,
                },
            ],
        };

        let lore = format_album_lore(&group);
        assert!(lore.contains("Collection of 2 tracks by ABBA."));
        assert!(lore.contains("2 lossless tracks (FLAC)."));
        assert!(lore.contains("Harmonic chroma pending CQT analysis."));
    }
}

