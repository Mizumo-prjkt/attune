use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};
use crate::theme::DynamicPalette;
use std::collections::{HashMap, HashSet};
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
    #[props(default)]
    favorites: HashSet<PathBuf>,
    #[props(default)]
    selected_album_name: Option<String>,
    #[props(default)]
    on_select_album: EventHandler<String>,
    #[props(default)]
    on_back: EventHandler<()>,
    on_play_track: EventHandler<TrackEntry>,
    on_play_tracks: EventHandler<Vec<TrackEntry>>,
    on_queue_track: EventHandler<TrackEntry>,
    #[props(default)]
    on_toggle_favorite: EventHandler<PathBuf>,
    #[props(default)]
    on_toggle_play: EventHandler<()>,
    on_edit_album: EventHandler<AlbumGroup>,
) -> Element {
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

    let selected_name = selected_album_name.as_ref();
    let selected_group = selected_name.and_then(|name| album_list.iter().find(|a| &a.name == name).cloned());

    rsx! {
        div { class: "albums-stage",
            if let Some(group) = selected_group {
                {
                    let track_count = group.tracks.len();
                    let total_dur_str = format_total_duration(&group.tracks);
                    let lore_description = format_album_lore(&group);
                    let art_data_url = group.artwork_data_url.clone();

                    let is_current_album = current_track_path.as_ref().map(|p| group.tracks.iter().any(|t| &t.path == p)).unwrap_or(false);
                    let is_current_album_playing = is_playing && is_current_album;

                    // Dominant color extraction for atmospheric header gradient
                    let hero_color_rgb = group.artwork_path.as_deref()
                        .and_then(DynamicPalette::from_image)
                        .map(|p| p.primary)
                        .unwrap_or_else(|| "rgb(75, 88, 115)".to_string());

                    // Year extraction from album name (e.g. "Title (2020)") or first track
                    let album_year = group.tracks.first().and_then(|_| {
                        let re_year = group.name.split('(').nth(1)?.split(')').next()?;
                        if re_year.len() == 4 && re_year.chars().all(|c| c.is_ascii_digit()) {
                            Some(re_year.to_string())
                        } else {
                            None
                        }
                    });

                    rsx! {
                        div {
                            class: "album-detail-view",
                            style: "background: linear-gradient(180deg, {hero_color_rgb} 0%, rgba(18, 18, 18, 0.72) 280px, var(--bg-canvas) 460px);",

                            // Hero Header: 232px Cover Artwork on left + Big Metadata on right
                            div { class: "album-detail-hero",
                                div { class: "album-cover-box",
                                    if let Some(art) = art_data_url.as_ref() {
                                        img {
                                            class: "album-cover-img",
                                            src: "{art}",
                                            alt: "{group.name}",
                                        }
                                    } else {
                                        div { class: "album-cover-fallback",
                                            span { class: "material-icons md-64", "album" }
                                        }
                                    }
                                }

                                div { class: "album-meta-col",
                                    span { class: "album-meta-type", "Album" }
                                    h1 { class: "album-meta-title", "{group.name}" }
                                    div { class: "album-meta-subline",
                                        div { class: "album-artist-avatar",
                                            if let Some(art) = art_data_url.as_ref() {
                                                img { src: "{art}", alt: "{group.artist}" }
                                            } else {
                                                span { class: "material-icons md-14", "person" }
                                            }
                                        }
                                        span { class: "album-artist-name", "{group.artist}" }
                                        if let Some(ref yr) = album_year {
                                            span { class: "album-dot-sep", "•" }
                                            span { "{yr}" }
                                        }
                                        span { class: "album-dot-sep", "•" }
                                        span { class: "album-meta-songs", "{track_count} songs," }
                                        span { class: "album-meta-dur", "{total_dur_str}" }
                                    }
                                }
                            }

                            // 3. Action Bar: Big green Play button, Shuffle, Favorite, Edit Info, More Options
                            div { class: "album-action-bar",
                                button {
                                    class: "album-big-play-btn",
                                    title: if is_current_album_playing { "Pause" } else { "Play entire album" },
                                    onclick: {
                                        let all_tracks = group.tracks.clone();
                                        move |_| {
                                            if is_current_album_playing {
                                                on_toggle_play.call(());
                                            } else {
                                                on_play_tracks.call(all_tracks.clone());
                                            }
                                        }
                                    },
                                    if is_current_album_playing {
                                        span { class: "material-icons md-32", "pause" }
                                    } else {
                                        span { class: "material-icons md-32", "play_arrow" }
                                    }
                                }

                                button {
                                    class: "album-icon-btn",
                                    title: "Shuffle Album",
                                    onclick: {
                                        let tracks_to_shuffle = group.tracks.clone();
                                        move |_| {
                                            let mut shuffled = tracks_to_shuffle.clone();
                                            if shuffled.len() > 1 {
                                                use std::time::SystemTime;
                                                let mut seed = SystemTime::now()
                                                    .duration_since(SystemTime::UNIX_EPOCH)
                                                    .map(|d| d.as_nanos())
                                                    .unwrap_or(42);
                                                for i in (1..shuffled.len()).rev() {
                                                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                                                    let j = ((seed >> 32) as usize) % (i + 1);
                                                    shuffled.swap(i, j);
                                                }
                                            }
                                            on_play_tracks.call(shuffled);
                                        }
                                    },
                                    span { class: "material-icons md-28", "shuffle" }
                                }

                                button {
                                    class: if group.tracks.iter().all(|t| favorites.contains(&t.path)) && !group.tracks.is_empty() {
                                        "album-icon-btn active"
                                    } else {
                                        "album-icon-btn"
                                    },
                                    title: "Favorite All Tracks in Album",
                                    onclick: {
                                        let all_paths: Vec<PathBuf> = group.tracks.iter().map(|t| t.path.clone()).collect();
                                        move |_| {
                                            for p in &all_paths {
                                                on_toggle_favorite.call(p.clone());
                                            }
                                        }
                                    },
                                    if group.tracks.iter().all(|t| favorites.contains(&t.path)) && !group.tracks.is_empty() {
                                        span { class: "material-icons md-28", style: "color: #fb7185;", "favorite" }
                                    } else {
                                        span { class: "material-icons md-28", "favorite_border" }
                                    }
                                }

                                button {
                                    class: "album-icon-btn",
                                    title: "Edit Album Metadata & Artwork",
                                    onclick: {
                                        let g = group.clone();
                                        move |_| on_edit_album.call(g.clone())
                                    },
                                    span { class: "material-icons md-24", "edit" }
                                }

                                button {
                                    class: "album-icon-btn",
                                    title: "Add All to Queue",
                                    onclick: {
                                        let all_tracks = group.tracks.clone();
                                        move |_| {
                                            for t in &all_tracks {
                                                on_queue_track.call(t.clone());
                                            }
                                        }
                                    },
                                    span { class: "material-icons md-28", "more_horiz" }
                                }

                                div { class: "album-action-right",
                                    button {
                                        class: "album-action-pill-btn",
                                        title: "Add Songs to this Album",
                                        onclick: {
                                            let g = group.clone();
                                            move |_| on_edit_album.call(g.clone())
                                        },
                                        span { class: "material-icons md-18", "playlist_add" }
                                        span { "Add Songs" }
                                    }
                                }
                            }

                            // 4. Tracklist Table: Header row + Table Rows
                            div { class: "album-tracklist-container",
                                div { class: "album-table-header",
                                    span { class: "album-table-header-idx", "#" }
                                    span { class: "album-table-header-title", "Title" }
                                    span { class: "album-table-header-audio", "Audio Profile" }
                                    div { class: "album-table-header-dur",
                                        span { class: "material-icons md-18", "schedule" }
                                    }
                                }

                                div { class: "album-table-body",
                                    for (idx, track) in group.tracks.iter().enumerate() {
                                        {
                                            let t = track.clone();
                                            let dur = fmt_ms(t.duration_ms);
                                            let is_current = current_track_path.as_ref().map(|p| p == &t.path).unwrap_or(false);
                                            let is_track_playing = is_current && is_playing;
                                            let is_fav = favorites.contains(&t.path);
                                            let path_for_fav = t.path.clone();
                                            let t_for_play = t.clone();
                                            let t_for_row = t.clone();
                                            let t_for_menu = t.clone();

                                            let audio_details = {
                                                let mut parts = vec![t.format.clone()];
                                                if t.sample_rate > 0 {
                                                    parts.push(format!("{:.0} kHz", t.sample_rate as f32 / 1000.0));
                                                }
                                                if let Some(bpm) = t.bpm {
                                                    parts.push(format!("{:.0} BPM", bpm));
                                                }
                                                if let Some(ref k) = t.key {
                                                    parts.push(k.clone());
                                                }
                                                parts.join(" • ")
                                            };

                                            rsx! {
                                                div {
                                                    key: "{t.path.display()}",
                                                    class: if is_current { "album-table-row active" } else { "album-table-row" },
                                                    ondoubleclick: move |_| on_play_track.call(t_for_row.clone()),

                                                    // Column 1: Index / Play Icon / Volume indicator
                                                    div { class: "album-row-idx-col",
                                                        if is_track_playing {
                                                            span { class: "material-icons md-18", style: "color: var(--accent-mint);", "volume_up" }
                                                        } else {
                                                            span { class: "album-row-number", "{idx + 1}" }
                                                            button {
                                                                class: "album-row-play-btn",
                                                                title: "Play track",
                                                                onclick: {
                                                                    let t_p = t_for_play.clone();
                                                                    move |evt: MouseEvent| {
                                                                        evt.stop_propagation();
                                                                        on_play_track.call(t_p.clone());
                                                                    }
                                                                },
                                                                span { class: "material-icons md-18", "play_arrow" }
                                                            }
                                                        }
                                                    }

                                                    // Column 2: Title and Artist
                                                    div { class: "album-row-title-col",
                                                        span {
                                                            class: if is_current { "album-row-title-text active" } else { "album-row-title-text" },
                                                            "{t.title}"
                                                        }
                                                        span { class: "album-row-artist-text", "{t.artist}" }
                                                    }

                                                    // Column 3: Audio Profile & Format
                                                    div { class: "album-row-audio-col",
                                                        "{audio_details}"
                                                    }

                                                    // Column 4: Favorite + Duration + More options menu
                                                    div { class: "album-row-dur-col",
                                                        button {
                                                            class: if is_fav { "album-row-fav-btn active" } else { "album-row-fav-btn" },
                                                            title: "Favorite",
                                                            onclick: {
                                                                let p = path_for_fav.clone();
                                                                move |evt: MouseEvent| {
                                                                    evt.stop_propagation();
                                                                    on_toggle_favorite.call(p.clone());
                                                                }
                                                            },
                                                            if is_fav {
                                                                span { class: "material-icons md-16", style: "color: #fb7185;", "favorite" }
                                                            } else {
                                                                span { class: "material-icons md-16", "favorite_border" }
                                                            }
                                                        }

                                                        span { class: "album-row-dur-text", "{dur}" }

                                                        button {
                                                            class: "album-row-menu-btn",
                                                            title: "Add to Queue",
                                                            onclick: {
                                                                let t_q = t_for_menu.clone();
                                                                move |evt: MouseEvent| {
                                                                    evt.stop_propagation();
                                                                    on_queue_track.call(t_q.clone());
                                                                }
                                                            },
                                                            span { class: "material-icons md-18", "more_horiz" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // 5. Album Footer / Notes & Dynamic Lore
                                div { class: "album-detail-footer",
                                    p { class: "album-footer-lore", "{lore_description}" }
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
                                            class: "album-grid-card",
                                            onclick: {
                                                let n = name.clone();
                                                move |_| {
                                                    on_select_album.call(n.clone());
                                                }
                                            },
                                            if !art.is_empty() {
                                                img {
                                                    src: "{art}",
                                                    alt: "{album.name}",
                                                    class: "album-grid-art",
                                                }
                                            } else {
                                                div {
                                                    class: "album-grid-fallback",
                                                    span { class: "material-icons md-48", "album" }
                                                }
                                            }
                                            div { style: "display: flex; flex-direction: column; overflow: hidden;",
                                                span { style: "font-weight: 600; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: #ffffff;", "{album.name}" }
                                                span { style: "color: var(--text-secondary); font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 3px;", "{album.artist}" }
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

/// Formats the sum of all track durations into a human-readable string (e.g., "24 min 44 sec" or "1 hr 15 min").
pub fn format_total_duration(tracks: &[TrackEntry]) -> String {
    let total_ms: u64 = tracks.iter().map(|t| t.duration_ms).sum();
    let total_sec = total_ms / 1000;
    let hours = total_sec / 3600;
    let mins = (total_sec % 3600) / 60;
    let secs = total_sec % 60;
    if hours > 0 {
        format!("{hours} hr {mins} min")
    } else if mins > 0 {
        format!("{mins} min {secs} sec")
    } else {
        format!("{secs} sec")
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

    #[test]
    fn test_format_total_duration() {
        let tracks = vec![
            TrackEntry {
                path: PathBuf::from("/music/track1.flac"),
                title: "How You Like That".into(),
                artist: "BLACKPINK".into(),
                album: "THE ALBUM".into(),
                genre: Some("K-Pop".into()),
                duration_ms: 182_000, // 3:02
                bpm: None,
                key: None,
                format: "FLAC".into(),
                sample_rate: 44100,
                bit_depth: 16,
                artwork_path: None,
                artwork_data_url: None,
            },
            TrackEntry {
                path: PathBuf::from("/music/track2.flac"),
                title: "Ice Cream".into(),
                artist: "BLACKPINK".into(),
                album: "THE ALBUM".into(),
                genre: Some("K-Pop".into()),
                duration_ms: 177_000, // 2:57
                bpm: None,
                key: None,
                format: "FLAC".into(),
                sample_rate: 44100,
                bit_depth: 16,
                artwork_path: None,
                artwork_data_url: None,
            },
        ];

        let total = format_total_duration(&tracks);
        assert_eq!(total, "5 min 59 sec");
    }
}
