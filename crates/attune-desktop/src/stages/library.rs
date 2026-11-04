use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};
use std::collections::HashSet;
use std::path::PathBuf;

#[component]
pub fn Library(
    tracks: Vec<TrackEntry>,
    search_query: String,
    current_track_path: Option<PathBuf>,
    favorites: HashSet<PathBuf>,
    on_play_track: EventHandler<TrackEntry>,
    on_queue_track: EventHandler<TrackEntry>,
    on_toggle_favorite: EventHandler<PathBuf>,
    on_search_change: EventHandler<String>,
) -> Element {
    let mut sort_by = use_signal(|| "title");
    let mut sort_asc = use_signal(|| true);

    let query_lower = search_query.to_lowercase();
    let mut filtered_tracks: Vec<TrackEntry> = tracks
        .into_iter()
        .filter(|t| t.matches(&query_lower))
        .collect();

    // Sort
    let asc = *sort_asc.read();
    match *sort_by.read() {
        "title" => filtered_tracks.sort_by(|a, b| if asc { a.title.cmp(&b.title) } else { b.title.cmp(&a.title) }),
        "artist" => filtered_tracks.sort_by(|a, b| if asc { a.artist.cmp(&b.artist) } else { b.artist.cmp(&a.artist) }),
        "album" => filtered_tracks.sort_by(|a, b| if asc { a.album.cmp(&b.album) } else { b.album.cmp(&a.album) }),
        "bpm" => filtered_tracks.sort_by(|a, b| {
            let ba = a.bpm.unwrap_or(0.0);
            let bb = b.bpm.unwrap_or(0.0);
            if asc { ba.partial_cmp(&bb).unwrap() } else { bb.partial_cmp(&ba).unwrap() }
        }),
        "duration" => filtered_tracks.sort_by(|a, b| if asc { a.duration_ms.cmp(&b.duration_ms) } else { b.duration_ms.cmp(&a.duration_ms) }),
        _ => {}
    }

    let total_count = filtered_tracks.len();
    let total_ms: u64 = filtered_tracks.iter().map(|t| t.duration_ms).sum();
    let total_dur_str = fmt_ms(total_ms);

    rsx! {
        div { class: "library-stage",
            style: "display: flex; flex-direction: column; height: 100%;",

            // Library Top Header
            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700;", "Music Library" }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                        "{total_count} songs • {total_dur_str} total listening time"
                    }
                }
                div { style: "display: flex; gap: 10px; align-items: center;",
                    div { class: "search-bar", style: "width: 260px;",
                        span { "🔍" }
                        input {
                            placeholder: "Search library...",
                            value: "{search_query}",
                            oninput: move |evt| on_search_change.call(evt.value()),
                        }
                    }
                }
            }

            // Track Table Header
            div { style: "display: grid; grid-template-columns: 44px 2fr 1.5fr 1.5fr 70px 70px 70px 60px; padding: 8px 12px; border-bottom: 1px solid var(--border-glass); color: var(--text-muted); font-size: 11.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;",
                span { "#" }
                span {
                    style: "cursor: pointer;",
                    onclick: move |_| {
                        if *sort_by.read() == "title" {
                            let curr = *sort_asc.read();
                            sort_asc.set(!curr);
                        } else {
                            sort_by.set("title");
                            sort_asc.set(true);
                        }
                    },
                    "Title" if *sort_by.read() == "title" { if *sort_asc.read() { " ▲" } else { " ▼" } }
                }
                span {
                    style: "cursor: pointer;",
                    onclick: move |_| {
                        if *sort_by.read() == "artist" {
                            let curr = *sort_asc.read();
                            sort_asc.set(!curr);
                        } else {
                            sort_by.set("artist");
                            sort_asc.set(true);
                        }
                    },
                    "Artist" if *sort_by.read() == "artist" { if *sort_asc.read() { " ▲" } else { " ▼" } }
                }
                span {
                    style: "cursor: pointer;",
                    onclick: move |_| {
                        if *sort_by.read() == "album" {
                            let curr = *sort_asc.read();
                            sort_asc.set(!curr);
                        } else {
                            sort_by.set("album");
                            sort_asc.set(true);
                        }
                    },
                    "Album" if *sort_by.read() == "album" { if *sort_asc.read() { " ▲" } else { " ▼" } }
                }
                span {
                    style: "cursor: pointer;",
                    onclick: move |_| {
                        if *sort_by.read() == "bpm" {
                            let curr = *sort_asc.read();
                            sort_asc.set(!curr);
                        } else {
                            sort_by.set("bpm");
                            sort_asc.set(true);
                        }
                    },
                    "BPM" if *sort_by.read() == "bpm" { if *sort_asc.read() { " ▲" } else { " ▼" } }
                }
                span { "Key" }
                span {
                    style: "cursor: pointer; text-align: right;",
                    onclick: move |_| {
                        if *sort_by.read() == "duration" {
                            let curr = *sort_asc.read();
                            sort_asc.set(!curr);
                        } else {
                            sort_by.set("duration");
                            sort_asc.set(true);
                        }
                    },
                    "Time" if *sort_by.read() == "duration" { if *sort_asc.read() { " ▲" } else { " ▼" } }
                }
                span { style: "text-align: center;", "Actions" }
            }

            // Track Table Rows
            div { style: "overflow-y: auto; flex: 1;",
                if filtered_tracks.is_empty() {
                    div { style: "text-align: center; padding: 60px; color: var(--text-muted); font-size: 15px;",
                        if search_query.is_empty() {
                            "No songs in library yet. Add music by choosing a folder in the sidebar."
                        } else {
                            "No tracks found matching your search query."
                        }
                    }
                } else {
                    for (idx, track) in filtered_tracks.into_iter().enumerate() {
                        {
                            let t = track.clone();
                            let is_current = current_track_path.as_ref() == Some(&t.path);
                            let is_fav = favorites.contains(&t.path);
                            let dur_str = fmt_ms(t.duration_ms);
                            let key_label = t.key.as_deref().unwrap_or("—");
                            let bpm_label = t.bpm.map(|b| format!("{:.0}", b)).unwrap_or_else(|| "—".to_string());
                            let path_clone = t.path.clone();

                            rsx! {
                                div {
                                    key: "{t.path.display()}",
                                    class: if is_current { "track-row playing" } else { "track-row" },
                                    style: "display: grid; grid-template-columns: 44px 2fr 1.5fr 1.5fr 70px 70px 70px 60px; align-items: center; padding: 7px 12px; font-size: 13px;",
                                    ondoubleclick: {
                                        let t_clone = t.clone();
                                        move |_| on_play_track.call(t_clone.clone())
                                    },

                                    // Index / Play Icon
                                    span {
                                        style: "color: var(--text-muted); font-family: monospace; font-size: 12px; cursor: pointer;",
                                        onclick: {
                                            let t_clone = t.clone();
                                            move |_| on_play_track.call(t_clone.clone())
                                        },
                                        if is_current { "▶" } else { "{idx + 1}" }
                                    }

                                    // Title
                                    span {
                                        style: "font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding-right: 12px;",
                                        "{t.title}"
                                    }

                                    // Artist
                                    span {
                                        style: "color: var(--text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding-right: 12px;",
                                        "{t.artist}"
                                    }

                                    // Album
                                    span {
                                        style: "color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding-right: 12px;",
                                        "{t.album}"
                                    }

                                    // BPM
                                    span {
                                        style: "color: var(--accent-mint); font-family: monospace; font-size: 12px;",
                                        "{bpm_label}"
                                    }

                                    // Key
                                    span {
                                        style: "color: var(--accent-mint); font-weight: 500;",
                                        "{key_label}"
                                    }

                                    // Duration
                                    span {
                                        style: "text-align: right; color: var(--text-muted); font-family: monospace; font-size: 12px;",
                                        "{dur_str}"
                                    }

                                    // Actions: Queue / Favorite
                                    div { style: "display: flex; gap: 6px; justify-content: center;",
                                        button {
                                            style: "background: transparent; border: none; cursor: pointer; color: var(--text-muted); font-size: 13px;",
                                            title: "Add to Queue",
                                            onclick: {
                                                let t_clone = t.clone();
                                                move |_| on_queue_track.call(t_clone.clone())
                                            },
                                            "+"
                                        }
                                        button {
                                            style: "background: transparent; border: none; cursor: pointer; color: if is_fav { \"#fb7185\" } else { \"var(--text-muted)\" }; font-size: 12px;",
                                            title: "Toggle Favorite",
                                            onclick: move |_| on_toggle_favorite.call(path_clone.clone()),
                                            if is_fav { "♥" } else { "♡" }
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
