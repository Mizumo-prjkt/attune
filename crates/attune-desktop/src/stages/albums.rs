use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumGroup {
    pub name: String,
    pub artist: String,
    pub artwork_path: Option<std::path::PathBuf>,
    pub artwork_data_url: Option<String>,
    pub tracks: Vec<TrackEntry>,
}

#[component]
pub fn Albums(
    tracks: Vec<TrackEntry>,
    on_play_track: EventHandler<TrackEntry>,
    on_play_tracks: EventHandler<Vec<TrackEntry>>,
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
                div { style: "display: flex; flex-direction: column; gap: 20px;",
                    button {
                        class: "btn-pill",
                        style: "width: fit-content;",
                        onclick: move |_| selected_album.set(None),
                        "← Back to Albums"
                    }

                    // Album Header Banner
                    div { style: "display: flex; gap: 24px; align-items: flex-end; background: var(--bg-card); padding: 24px; border-radius: 16px; border: 1px solid var(--border-glass);",
                        if let Some(art) = group.artwork_data_url.as_ref() {
                            img {
                                src: "{art}",
                                alt: "{group.name}",
                                style: "width: 150px; height: 150px; border-radius: 12px; object-fit: cover; box-shadow: 0 8px 24px rgba(0,0,0,0.5);",
                            }
                        } else {
                            div {
                                style: "width: 150px; height: 150px; border-radius: 12px; background: rgba(255,255,255,0.06); display: flex; align-items: center; justify-content: center; font-size: 48px; border: 1px solid var(--border-glass);",
                                "💿"
                            }
                        }

                        div { style: "display: flex; flex-direction: column; gap: 6px; flex: 1;",
                            span { style: "color: var(--accent-mint); font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;", "ALBUM" }
                            h1 { style: "font-size: 28px; font-weight: 700;", "{group.name}" }
                            p { style: "color: var(--text-secondary); font-size: 15px;", "{group.artist} • {group.tracks.len()} songs" }

                            div { style: "display: flex; gap: 10px; margin-top: 14px; flex-wrap: wrap;",
                                button {
                                    class: "btn-pill active",
                                    onclick: {
                                        let all = group.tracks.clone();
                                        move |_| on_play_tracks.call(all.clone())
                                    },
                                    "▶ Play Album"
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
                    }

                    // Track List
                    div { style: "display: flex; flex-direction: column; gap: 2px;",
                        for (idx, track) in group.tracks.iter().enumerate() {
                            {
                                let t = track.clone();
                                let dur = fmt_ms(t.duration_ms);
                                rsx! {
                                    div {
                                        key: "{t.path.display()}",
                                        class: "track-row",
                                        style: "display: grid; grid-template-columns: 40px 1fr 80px; align-items: center; padding: 10px 14px;",
                                        ondoubleclick: {
                                            let t_clone = t.clone();
                                            move |_| on_play_track.call(t_clone.clone())
                                        },
                                        span { style: "color: var(--text-muted); font-size: 12px; font-family: monospace;", "{idx + 1}" }
                                        div { style: "display: flex; flex-direction: column;",
                                            span { style: "font-weight: 500; font-size: 14px;", "{t.title}" }
                                            span { style: "color: var(--text-muted); font-size: 12px;", "{t.artist}" }
                                        }
                                        span { style: "text-align: right; color: var(--text-muted); font-family: monospace; font-size: 12px;", "{dur}" }
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
