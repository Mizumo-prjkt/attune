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
    on_select_artist: EventHandler<String>,
    #[props(default)]
    on_back: EventHandler<()>,
    on_play_track: EventHandler<TrackEntry>,
    on_play_tracks: EventHandler<Vec<TrackEntry>>,
    on_queue_track: EventHandler<TrackEntry>,
    #[props(default)]
    on_play_next: EventHandler<TrackEntry>,
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

    let mut active_menu_track = use_signal(|| None::<TrackEntry>);
    let mut show_credits_track = use_signal(|| None::<TrackEntry>);

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

                    let artist_clean = crate::artist_profile::clean_primary_artist(&group.artist).to_lowercase();
                    let mut artist_releases: Vec<AlbumGroup> = album_list
                        .iter()
                        .filter(|a| {
                            let a_clean = crate::artist_profile::clean_primary_artist(&a.artist).to_lowercase();
                            (a_clean == artist_clean || a.artist.to_lowercase().contains(&artist_clean))
                                && a.name != group.name
                        })
                        .cloned()
                        .collect();

                    let has_artist_discography = !artist_releases.is_empty();
                    let catalog_pool: Vec<AlbumGroup> = if has_artist_discography {
                        artist_releases.sort_by(|a, b| b.tracks.len().cmp(&a.tracks.len()).then_with(|| a.name.cmp(&b.name)));
                        artist_releases
                    } else {
                        let mut others: Vec<AlbumGroup> = album_list
                            .iter()
                            .filter(|a| a.name != group.name)
                            .cloned()
                            .collect();
                        others.sort_by(|a, b| b.tracks.len().cmp(&a.tracks.len()).then_with(|| a.name.cmp(&b.name)));
                        others
                    };

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
                                        div {
                                            class: "album-artist-avatar",
                                            style: "cursor: pointer;",
                                            onclick: {
                                                let art = crate::artist_profile::clean_primary_artist(&group.artist);
                                                move |_| on_select_artist.call(art.clone())
                                            },
                                            if let Some(art) = art_data_url.as_ref() {
                                                img { src: "{art}", alt: "{group.artist}" }
                                            } else {
                                                span { class: "material-icons md-14", "person" }
                                            }
                                        }
                                        span {
                                            class: "album-artist-name",
                                            style: "cursor: pointer;",
                                            onclick: {
                                                let art = crate::artist_profile::clean_primary_artist(&group.artist);
                                                move |_| on_select_artist.call(art.clone())
                                            },
                                            "{group.artist}"
                                        }
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

                                            let is_menu_open = active_menu_track.read().as_ref().map(|m| m.path == t.path).unwrap_or(false);

                                            rsx! {
                                                div {
                                                    key: "{t.path.display()}",
                                                    class: if is_current { "album-table-row active" } else { "album-table-row" },
                                                    style: "position: relative;",
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
                                                            title: "More options",
                                                            onclick: {
                                                                let t_m = t_for_menu.clone();
                                                                move |evt: MouseEvent| {
                                                                    evt.stop_propagation();
                                                                    let curr = active_menu_track.read().clone();
                                                                    if curr.as_ref().map(|c| c.path == t_m.path).unwrap_or(false) {
                                                                        active_menu_track.set(None);
                                                                    } else {
                                                                        active_menu_track.set(Some(t_m.clone()));
                                                                    }
                                                                }
                                                            },
                                                            span { class: "material-icons md-18", "more_horiz" }
                                                        }

                                                        // Context Menu Popover
                                                        if is_menu_open {
                                                            div {
                                                                class: "track-context-menu",
                                                                onclick: move |evt: MouseEvent| evt.stop_propagation(),

                                                                button {
                                                                    class: "track-menu-item",
                                                                    onclick: {
                                                                        let t_p = t.clone();
                                                                        move |_| {
                                                                            active_menu_track.set(None);
                                                                            on_play_track.call(t_p.clone());
                                                                        }
                                                                    },
                                                                    span { class: "material-icons md-16", "play_arrow" }
                                                                    span { "Play Now" }
                                                                }

                                                                button {
                                                                    class: "track-menu-item",
                                                                    onclick: {
                                                                        let t_n = t.clone();
                                                                        move |_| {
                                                                            active_menu_track.set(None);
                                                                            on_play_next.call(t_n.clone());
                                                                        }
                                                                    },
                                                                    span { class: "material-icons md-16", "skip_next" }
                                                                    span { "Play Next" }
                                                                }

                                                                button {
                                                                    class: "track-menu-item",
                                                                    onclick: {
                                                                        let t_q = t.clone();
                                                                        move |_| {
                                                                            active_menu_track.set(None);
                                                                            on_queue_track.call(t_q.clone());
                                                                        }
                                                                    },
                                                                    span { class: "material-icons md-16", "playlist_add" }
                                                                    span { "Add to Queue" }
                                                                }

                                                                button {
                                                                    class: "track-menu-item",
                                                                    onclick: {
                                                                        let p = t.path.clone();
                                                                        move |_| {
                                                                            active_menu_track.set(None);
                                                                            on_toggle_favorite.call(p.clone());
                                                                        }
                                                                    },
                                                                    if is_fav {
                                                                        span { class: "material-icons md-16", style: "color: #fb7185;", "favorite" }
                                                                    } else {
                                                                        span { class: "material-icons md-16", "favorite_border" }
                                                                    }
                                                                    span { if is_fav { "Remove Favorite" } else { "Add to Favorites" } }
                                                                }

                                                                div { class: "track-menu-divider" }

                                                                button {
                                                                    class: "track-menu-item",
                                                                    onclick: {
                                                                        let artist = crate::artist_profile::clean_primary_artist(&t.artist);
                                                                        move |_| {
                                                                            active_menu_track.set(None);
                                                                            on_select_artist.call(artist.clone());
                                                                        }
                                                                    },
                                                                    span { class: "material-icons md-16", "person" }
                                                                    span { "Go to Artist" }
                                                                }

                                                                button {
                                                                    class: "track-menu-item",
                                                                    onclick: {
                                                                        let t_c = t.clone();
                                                                        move |_| {
                                                                            active_menu_track.set(None);
                                                                            show_credits_track.set(Some(t_c.clone()));
                                                                        }
                                                                    },
                                                                    span { class: "material-icons md-16", "info" }
                                                                    span { "View Track Credits & Metadata" }
                                                                }
                                                            }
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

                                // 6. Discography & More Catalogs Section (No filter chips, those belong to Artist view)
                                div { class: "album-discography-section",
                                    div { class: "discography-header-row",
                                        div { style: "display: flex; align-items: center; gap: 16px; flex-wrap: wrap;",
                                            h2 { class: "discography-title",
                                                if has_artist_discography {
                                                    "More by {group.artist}"
                                                } else {
                                                    "More Albums in Your Library"
                                                }
                                            }
                                        }
                                        span { class: "discography-count", "{catalog_pool.len()} releases" }
                                    }

                                    if catalog_pool.is_empty() {
                                        div { class: "discography-empty",
                                            "No other albums found in your library."
                                        }
                                    } else {
                                        div { class: "discography-shelf",
                                            for disc_album in catalog_pool.iter() {
                                                {
                                                    let a_name = disc_album.name.clone();
                                                    let a_name_click = disc_album.name.clone();
                                                    let a_art = disc_album.artwork_data_url.clone();
                                                    let a_tracks = disc_album.tracks.clone();
                                                    let track_len = disc_album.tracks.len();

                                                    let rel_type = if track_len == 1 {
                                                        "Single".to_string()
                                                    } else if track_len <= 5 {
                                                        "Single • EP".to_string()
                                                    } else {
                                                        "Album".to_string()
                                                    };

                                                    let year_str = disc_album.tracks.first().and_then(|_| {
                                                        let re_year = disc_album.name.split('(').nth(1)?.split(')').next()?;
                                                        if re_year.len() == 4 && re_year.chars().all(|c| c.is_ascii_digit()) {
                                                            Some(re_year.to_string())
                                                        } else {
                                                            None
                                                        }
                                                    });

                                                    let subtitle = if let Some(yr) = year_str {
                                                        format!("{yr} • {rel_type}")
                                                    } else {
                                                        format!("{rel_type} • {track_len} songs")
                                                    };

                                                    rsx! {
                                                        div {
                                                            key: "{a_name}",
                                                            class: "discography-card",
                                                            onclick: move |_| on_select_album.call(a_name_click.clone()),

                                                            div { class: "discography-art-box",
                                                                if let Some(ref art) = a_art {
                                                                    img { class: "discography-art-img", src: "{art}", alt: "{a_name}" }
                                                                } else {
                                                                    div { class: "discography-art-fallback",
                                                                        span { class: "material-icons md-36", "album" }
                                                                    }
                                                                }

                                                                // Green circular hover play button
                                                                button {
                                                                    class: "discography-hover-play",
                                                                    title: "Play entire release",
                                                                    onclick: {
                                                                        let t_play = a_tracks.clone();
                                                                        move |evt: MouseEvent| {
                                                                            evt.stop_propagation();
                                                                            on_play_tracks.call(t_play.clone());
                                                                        }
                                                                    },
                                                                    span { class: "material-icons md-28", "play_arrow" }
                                                                }
                                                            }

                                                            div { class: "discography-meta-col",
                                                                span { class: "discography-card-title", title: "{a_name}", "{a_name}" }
                                                                span { class: "discography-card-sub", "{subtitle}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Track Credits & Metadata Modal
                            if let Some(track) = show_credits_track.read().clone() {
                                {
                                    let dur_str = fmt_ms(track.duration_ms);
                                    let vocalist_str = track.vocalist.clone().unwrap_or_else(|| {
                                        if track.artist.contains("feat.") || track.artist.contains("ft.") {
                                            crate::artist_profile::clean_primary_artist(&track.artist)
                                        } else {
                                            "—".to_string()
                                        }
                                    });
                                    let writer_str = track.writer.clone().or(track.composer.clone()).unwrap_or_else(|| "—".to_string());
                                    let producer_str = track.producer.clone().unwrap_or_else(|| "—".to_string());
                                    let sample_rate_str = format!("{} Hz ({:.1} kHz)", track.sample_rate, track.sample_rate as f32 / 1000.0);
                                    let bpm_str = track.bpm.map(|b| format!("{:.1} BPM", b)).unwrap_or_else(|| "—".to_string());
                                    let key_str = track.key.clone().unwrap_or_else(|| "—".to_string());
                                    let genre_str = track.genre.clone().unwrap_or_else(|| "—".to_string());

                                    rsx! {
                                        div {
                                            class: "credits-modal-backdrop",
                                            onclick: move |_| show_credits_track.set(None),

                                            div {
                                                class: "credits-modal-dialog",
                                                onclick: move |e: MouseEvent| e.stop_propagation(),

                                                div { class: "credits-modal-header",
                                                    div {
                                                        h2 { style: "font-size: 18px; font-weight: 700; margin: 0;", "Track Credits & Metadata" }
                                                        p { style: "font-size: 12px; color: var(--text-muted); margin: 4px 0 0 0;", "{track.title}" }
                                                    }
                                                    button {
                                                        class: "modal-close-btn",
                                                        onclick: move |_| show_credits_track.set(None),
                                                        span { class: "material-icons md-20", "close" }
                                                    }
                                                }

                                                div { class: "credits-modal-body",
                                                    // Creative Roles
                                                    div { class: "credits-section",
                                                        h3 { class: "credits-section-title", "Creative Credits" }
                                                        div { class: "credits-grid",
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Performed By / Artist" }
                                                                span { class: "credits-value", "{track.artist}" }
                                                            }
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Vocals" }
                                                                span { class: "credits-value", "{vocalist_str}" }
                                                            }
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Writers & Composers" }
                                                                span { class: "credits-value", "{writer_str}" }
                                                            }
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Producer / Arranger" }
                                                                span { class: "credits-value", "{producer_str}" }
                                                            }
                                                        }
                                                    }

                                                    // Audio & Acoustic DSP Profile
                                                    div { class: "credits-section",
                                                        h3 { class: "credits-section-title", "Audio Fidelity & Profile" }
                                                        div { class: "credits-grid",
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Format & Codec" }
                                                                span { class: "credits-value", "{track.format}" }
                                                            }
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Sample Rate" }
                                                                span { class: "credits-value", "{sample_rate_str}" }
                                                            }
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Duration" }
                                                                span { class: "credits-value", "{dur_str}" }
                                                            }
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Tempo (BPM)" }
                                                                span { class: "credits-value", "{bpm_str}" }
                                                            }
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Harmonic Key (Camelot)" }
                                                                span { class: "credits-value", "{key_str}" }
                                                            }
                                                            div { class: "credits-cell",
                                                                span { class: "credits-label", "Genre" }
                                                                span { class: "credits-value", "{genre_str}" }
                                                            }
                                                        }
                                                    }

                                                    // Disk Location
                                                    div { class: "credits-section",
                                                        h3 { class: "credits-section-title", "File Source" }
                                                        div { class: "credits-path-box",
                                                            span { style: "font-family: monospace; font-size: 11.5px; color: var(--text-muted); word-break: break-all;",
                                                                "{track.path.display()}"
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
                    aggression: None,
                    vibe_family: None,
                    ..Default::default()
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
                    aggression: None,
                    vibe_family: None,
                    ..Default::default()
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
                aggression: None,
                vibe_family: None,
                ..Default::default()
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
                aggression: None,
                vibe_family: None,
                ..Default::default()
            },
        ];

        let total = format_total_duration(&tracks);
        assert_eq!(total, "5 min 59 sec");
    }

    #[test]
    fn test_discography_filter_categorization() {
        let single = AlbumGroup {
            name: "Vampire".into(),
            artist: "DECO*27".into(),
            artwork_path: None,
            artwork_data_url: None,
            tracks: vec![TrackEntry {
                path: PathBuf::from("/music/vampire.flac"),
                title: "Vampire".into(),
                artist: "DECO*27".into(),
                album: "Vampire".into(),
                genre: None,
                duration_ms: 180000,
                bpm: None,
                key: None,
                format: "FLAC".into(),
                sample_rate: 44100,
                bit_depth: 16,
                artwork_path: None,
                artwork_data_url: None,
                aggression: None,
                vibe_family: None,
                ..Default::default()
            }],
        };

        let ep = AlbumGroup {
            name: "Rabbit Hole EP".into(),
            artist: "DECO*27".into(),
            artwork_path: None,
            artwork_data_url: None,
            tracks: (0..3)
                .map(|i| TrackEntry {
                    path: PathBuf::from(format!("/music/track{i}.flac")),
                    title: format!("Track {i}"),
                    artist: "DECO*27".into(),
                    album: "Rabbit Hole EP".into(),
                    genre: None,
                    duration_ms: 180000,
                    bpm: None,
                    key: None,
                    format: "FLAC".into(),
                    sample_rate: 44100,
                    bit_depth: 16,
                    artwork_path: None,
                    artwork_data_url: None,
                    aggression: None,
                    vibe_family: None,
                    ..Default::default()
                })
                .collect(),
        };

        let lp = AlbumGroup {
            name: "GHOST".into(),
            artist: "DECO*27".into(),
            artwork_path: None,
            artwork_data_url: None,
            tracks: (0..12)
                .map(|i| TrackEntry {
                    path: PathBuf::from(format!("/music/ghost{i}.flac")),
                    title: format!("Ghost {i}"),
                    artist: "DECO*27".into(),
                    album: "GHOST".into(),
                    genre: None,
                    duration_ms: 180000,
                    bpm: None,
                    key: None,
                    format: "FLAC".into(),
                    sample_rate: 44100,
                    bit_depth: 16,
                    artwork_path: None,
                    artwork_data_url: None,
                    aggression: None,
                    vibe_family: None,
                    ..Default::default()
                })
                .collect(),
        };

        let all = vec![single, ep, lp];

        // Filter: Albums (> 5 tracks)
        let albums_only: Vec<_> = all.iter().filter(|a| a.tracks.len() > 5).collect();
        assert_eq!(albums_only.len(), 1);
        assert_eq!(albums_only[0].name, "GHOST");

        // Filter: Singles and EPs (<= 5 tracks)
        let singles_eps: Vec<_> = all.iter().filter(|a| a.tracks.len() <= 5).collect();
        assert_eq!(singles_eps.len(), 2);
    }
}
