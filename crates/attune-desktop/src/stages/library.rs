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
    #[props(default)]
    on_artwork_updated: EventHandler<(Option<PathBuf>, PathBuf, String)>,
) -> Element {
    let mut sort_by = use_signal(|| "title");
    let mut sort_asc = use_signal(|| true);
    let selected_track_for_art = use_signal(|| None::<TrackEntry>);
    let active_menu_track = use_signal(|| None::<TrackEntry>);
    let show_info_track = use_signal(|| None::<TrackEntry>);

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
    let strings = crate::i18n::current_strings();

    rsx! {
        div { class: "library-stage",
            style: "display: flex; flex-direction: column; height: 100%;",
            onclick: {
                let mut active_menu = active_menu_track;
                move |_| active_menu.set(None)
            },

            // Library Top Header
            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700;", "{strings.music_library}" }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                        "{total_count} {strings.songs.to_lowercase()} • {total_dur_str} {strings.total_listening_time}"
                    }
                }
                div { style: "display: flex; gap: 10px; align-items: center;",
                    div { class: "search-bar", style: "width: 260px;",
                        span { class: "material-icons md-18", "search" }
                        input {
                            placeholder: "{strings.search_library_placeholder}",
                            value: "{search_query}",
                            oninput: move |evt| on_search_change.call(evt.value()),
                        }
                    }
                }
            }

            // Track Table Header
            div { style: "display: grid; grid-template-columns: 44px 2fr 1.5fr 1.5fr 68px 68px 80px 84px; padding: 8px 12px; border-bottom: 1px solid var(--border-glass); color: var(--text-muted); font-size: 11.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; align-items: center;",
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
                    "{strings.title}" if *sort_by.read() == "title" { if *sort_asc.read() { " ▲" } else { " ▼" } }
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
                    "{strings.artist}" if *sort_by.read() == "artist" { if *sort_asc.read() { " ▲" } else { " ▼" } }
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
                    "{strings.album}" if *sort_by.read() == "album" { if *sort_asc.read() { " ▲" } else { " ▼" } }
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
                    style: "cursor: pointer; text-align: right; padding-right: 14px;",
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
                            let is_menu_open = active_menu_track.read().as_ref().map(|c| c.path == t.path).unwrap_or(false);
                            let is_near_bottom = idx >= total_count.saturating_sub(4) && total_count > 4;

                            rsx! {
                                div {
                                    key: "{t.path.display()}",
                                    class: if is_current { "track-row playing" } else { "track-row" },
                                    style: "display: grid; grid-template-columns: 44px 2fr 1.5fr 1.5fr 68px 68px 80px 84px; align-items: center; padding: 7px 12px; font-size: 13px; position: relative;",
                                    ondoubleclick: {
                                        let t_clone = t.clone();
                                        move |_| on_play_track.call(t_clone.clone())
                                    },

                                    // Index / Play Icon
                                    span {
                                        style: "color: var(--text-muted); font-family: monospace; font-size: 12px; cursor: pointer; display: flex; align-items: center;",
                                        onclick: {
                                            let t_clone = t.clone();
                                            move |_| on_play_track.call(t_clone.clone())
                                        },
                                        if is_current {
                                            span { class: "material-icons md-16", style: "color: var(--accent-mint);", "play_arrow" }
                                        } else {
                                            "{idx + 1}"
                                        }
                                    }

                                    // Title
                                    div { class: "marquee-container", style: "padding-right: 12px;",
                                        span { class: "marquee-text", style: "font-weight: 500;", title: "{t.title}", "{t.title}" }
                                    }

                                    // Artist
                                    div { class: "marquee-container", style: "padding-right: 12px;",
                                        span { class: "marquee-text", style: "color: var(--text-secondary);", title: "{t.artist}", "{t.artist}" }
                                    }

                                    // Album
                                    div { class: "marquee-container", style: "padding-right: 12px;",
                                        span { class: "marquee-text", style: "color: var(--text-muted);", title: "{t.album}", "{t.album}" }
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
                                        style: "text-align: right; padding-right: 14px; color: var(--text-muted); font-family: monospace; font-size: 12px;",
                                        "{dur_str}"
                                    }

                                    // Actions: Favorite Heart + More Options (...)
                                    div {
                                        style: "display: flex; gap: 4px; justify-content: center; align-items: center; position: relative;",
                                        onclick: move |e: MouseEvent| e.stop_propagation(),

                                        // Favorite Heart button
                                        button {
                                            class: if is_fav { "action-icon-btn active" } else { "action-icon-btn" },
                                            style: if is_fav { "color: #fb7185;" } else { "" },
                                            title: if is_fav { strings.remove_favorite } else { strings.add_to_favorites },
                                            onclick: {
                                                let p = path_clone.clone();
                                                move |e: MouseEvent| {
                                                    e.stop_propagation();
                                                    on_toggle_favorite.call(p.clone());
                                                }
                                            },
                                            if is_fav {
                                                span { class: "material-icons md-16", "favorite" }
                                            } else {
                                                span { class: "material-icons md-16", "favorite_border" }
                                            }
                                        }

                                        // More Options (...) button
                                        button {
                                            class: if is_menu_open { "action-icon-btn active" } else { "action-icon-btn" },
                                            title: "More options",
                                            onclick: {
                                                let t_m = t.clone();
                                                let mut active_menu = active_menu_track;
                                                move |e: MouseEvent| {
                                                    e.stop_propagation();
                                                    let curr = active_menu.read().clone();
                                                    if curr.as_ref().map(|c| c.path == t_m.path).unwrap_or(false) {
                                                        active_menu.set(None);
                                                    } else {
                                                        active_menu.set(Some(t_m.clone()));
                                                    }
                                                }
                                            },
                                            span { class: "material-icons md-18", "more_horiz" }
                                        }

                                        // Popover Menu for More Options
                                        if is_menu_open {
                                            div {
                                                class: "track-context-menu",
                                                style: if is_near_bottom {
                                                    "position: absolute; bottom: 100%; top: auto; right: 0; margin-bottom: 4px; z-index: 100;"
                                                } else {
                                                    "position: absolute; top: 100%; right: 0; margin-top: 4px; z-index: 100;"
                                                },
                                                onclick: move |e: MouseEvent| e.stop_propagation(),

                                                // 1. Play Now
                                                button {
                                                    class: "track-menu-item",
                                                    onclick: {
                                                        let t_play = t.clone();
                                                        let mut active_menu = active_menu_track;
                                                        move |_| {
                                                            active_menu.set(None);
                                                            on_play_track.call(t_play.clone());
                                                        }
                                                    },
                                                    span { class: "material-icons md-16", "play_arrow" }
                                                    span { "{strings.play_now}" }
                                                }

                                                // 2. Add to Queue
                                                button {
                                                    class: "track-menu-item",
                                                    onclick: {
                                                        let t_queue = t.clone();
                                                        let mut active_menu = active_menu_track;
                                                        move |_| {
                                                            active_menu.set(None);
                                                            on_queue_track.call(t_queue.clone());
                                                        }
                                                    },
                                                    span { class: "material-icons md-16", "playlist_add" }
                                                    span { "{strings.add_to_queue}" }
                                                }

                                                // 3. Change Song Artwork
                                                button {
                                                    class: "track-menu-item",
                                                    onclick: {
                                                        let t_art = t.clone();
                                                        let mut active_menu = active_menu_track;
                                                        let mut sel_art = selected_track_for_art;
                                                        move |_| {
                                                            active_menu.set(None);
                                                            sel_art.set(Some(t_art.clone()));
                                                        }
                                                    },
                                                    span { class: "material-icons md-16", "image_search" }
                                                    span { "Change Song Artwork..." }
                                                }

                                                div { class: "track-menu-divider" }

                                                // 4. Song Information & Metadata
                                                button {
                                                    class: "track-menu-item",
                                                    onclick: {
                                                        let t_info = t.clone();
                                                        let mut active_menu = active_menu_track;
                                                        let mut show_info = show_info_track;
                                                        move |_| {
                                                            active_menu.set(None);
                                                            show_info.set(Some(t_info.clone()));
                                                        }
                                                    },
                                                    span { class: "material-icons md-16", "info" }
                                                    span { "Song Information" }
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

            if let Some(ref t) = *selected_track_for_art.read() {
                crate::components::artwork_selector::ArtworkSelectorModal {
                    title: t.title.clone(),
                    artist: t.artist.clone(),
                    album: t.album.clone(),
                    track_path: Some(t.path.clone()),
                    current_artwork_url: t.artwork_data_url.clone(),
                    is_album_level: false,
                    on_close: {
                        let mut sel_art = selected_track_for_art;
                        move |_| sel_art.set(None)
                    },
                    on_artwork_applied: {
                        let t_p = t.path.clone();
                        let mut sel_art = selected_track_for_art;
                        move |(path, data_url)| {
                            sel_art.set(None);
                            on_artwork_updated.call((Some(t_p.clone()), path, data_url));
                        }
                    },
                }
            }

            if let Some(track) = show_info_track.read().clone() {
                crate::components::track_info_modal::TrackInfoModal {
                    track,
                    on_close: {
                        let mut show_info = show_info_track;
                        move |_| show_info.set(None)
                    },
                }
            }
        }
    }
}
