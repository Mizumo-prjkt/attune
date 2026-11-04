use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};
use std::collections::HashSet;
use std::path::PathBuf;

#[component]
pub fn Favorites(
    tracks: Vec<TrackEntry>,
    favorites: HashSet<PathBuf>,
    current_track_path: Option<PathBuf>,
    on_play_track: EventHandler<TrackEntry>,
    on_play_tracks: EventHandler<Vec<TrackEntry>>,
    on_queue_track: EventHandler<TrackEntry>,
    on_toggle_favorite: EventHandler<PathBuf>,
) -> Element {
    let fav_tracks: Vec<TrackEntry> = tracks
        .into_iter()
        .filter(|t| favorites.contains(&t.path))
        .collect();

    let total_count = fav_tracks.len();
    let total_ms: u64 = fav_tracks.iter().map(|t| t.duration_ms).sum();
    let dur_str = fmt_ms(total_ms);

    let all_favs_play = fav_tracks.clone();
    let all_favs_shuffle = {
        let mut s = fav_tracks.clone();
        s.reverse();
        s
    };

    rsx! {
        div { class: "favorites-stage",
            style: "display: flex; flex-direction: column; height: 100%;",

            // Top Header
            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700;", "Favorite Songs" }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                        "{total_count} favorites • {dur_str} total"
                    }
                }
                div { style: "display: flex; gap: 10px;",
                    button {
                        class: "btn-pill active",
                        disabled: fav_tracks.is_empty(),
                        onclick: move |_| on_play_tracks.call(all_favs_play.clone()),
                        "▶ Play Favorites"
                    }
                    button {
                        class: "btn-pill",
                        disabled: fav_tracks.is_empty(),
                        onclick: move |_| on_play_tracks.call(all_favs_shuffle.clone()),
                        "🔀 Shuffle"
                    }
                }
            }

            // Table Header
            div { style: "display: grid; grid-template-columns: 44px 2fr 1.5fr 1.5fr 70px 70px 70px 60px; padding: 8px 12px; border-bottom: 1px solid var(--border-glass); color: var(--text-muted); font-size: 11.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;",
                span { "#" }
                span { "Title" }
                span { "Artist" }
                span { "Album" }
                span { "BPM" }
                span { "Key" }
                span { style: "text-align: right;", "Time" }
                span { style: "text-align: center;", "Actions" }
            }

            // Track Rows
            div { style: "overflow-y: auto; flex: 1;",
                if fav_tracks.is_empty() {
                    div { style: "text-align: center; padding: 60px; color: var(--text-muted); font-size: 15px;",
                        "No favorite tracks yet. Click the heart icon (♡) on any track in your library to add it here."
                    }
                } else {
                    for (idx, track) in fav_tracks.iter().enumerate() {
                        {
                            let t = track.clone();
                            let is_current = current_track_path.as_ref() == Some(&t.path);
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
                                            style: "background: transparent; border: none; cursor: pointer; color: #fb7185; font-size: 12px;",
                                            title: "Remove from Favorites",
                                            onclick: move |_| on_toggle_favorite.call(path_clone.clone()),
                                            "♥"
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
