use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Active tab in the unified Favorites stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FavoritesTab {
    Songs,
    Artists,
    Albums,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FavoriteAlbumGroup {
    pub name: String,
    pub artist: String,
    pub artwork_data_url: Option<String>,
    pub tracks: Vec<TrackEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FavoriteArtistGroup {
    pub name: String,
    pub track_count: usize,
    pub album_count: usize,
    pub photo_url: Option<String>,
    pub fallback_artwork_url: Option<String>,
    pub is_verified: bool,
    pub tracks: Vec<TrackEntry>,
}

#[component]
pub fn Favorites(
    tracks: Vec<TrackEntry>,
    favorites: HashSet<PathBuf>,
    #[props(default)]
    favorite_artists: HashSet<String>,
    #[props(default)]
    favorite_albums: HashSet<String>,
    current_track_path: Option<PathBuf>,
    on_play_track: EventHandler<TrackEntry>,
    on_play_tracks: EventHandler<Vec<TrackEntry>>,
    on_queue_track: EventHandler<TrackEntry>,
    on_toggle_favorite: EventHandler<PathBuf>,
    #[props(default)]
    on_toggle_favorite_artist: EventHandler<String>,
    #[props(default)]
    on_toggle_favorite_album: EventHandler<String>,
    #[props(default)]
    on_select_artist: EventHandler<String>,
    #[props(default)]
    on_select_album: EventHandler<String>,
) -> Element {
    let mut active_tab = use_signal(|| FavoritesTab::Songs);
    let mut search_query = use_signal(String::new);
    let strings = crate::i18n::current_strings();

    let query = search_query.read().trim().to_lowercase();

    // 1. Gather Favorite Songs
    let all_fav_tracks: Vec<TrackEntry> = tracks
        .iter()
        .filter(|t| favorites.contains(&t.path))
        .cloned()
        .collect();

    let filtered_fav_tracks: Vec<TrackEntry> = all_fav_tracks
        .iter()
        .filter(|t| {
            if query.is_empty() {
                true
            } else {
                t.title.to_lowercase().contains(&query)
                    || t.artist.to_lowercase().contains(&query)
                    || t.album.to_lowercase().contains(&query)
            }
        })
        .cloned()
        .collect();

    // 2. Gather Favorite Artists
    let all_unique_artists = crate::artist_profile::extract_all_unique_artists(&tracks);
    let mut all_fav_artists: Vec<FavoriteArtistGroup> = Vec::new();
    for artist in all_unique_artists {
        let artist_tracks: Vec<TrackEntry> = tracks
            .iter()
            .filter(|t| crate::artist_profile::track_has_artist(t, &artist.name))
            .cloned()
            .collect();

        let is_fav = favorite_artists.contains(&artist.name)
            || artist_tracks.iter().any(|t| favorites.contains(&t.path));

        if is_fav {
            all_fav_artists.push(FavoriteArtistGroup {
                name: artist.name,
                track_count: artist.track_count,
                album_count: artist.album_count,
                photo_url: artist.photo_url,
                fallback_artwork_url: artist.fallback_artwork_url,
                is_verified: artist.is_verified,
                tracks: artist_tracks,
            });
        }
    }
    all_fav_artists.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let filtered_fav_artists: Vec<FavoriteArtistGroup> = all_fav_artists
        .iter()
        .filter(|a| query.is_empty() || a.name.to_lowercase().contains(&query))
        .cloned()
        .collect();

    // 3. Gather Favorite Albums
    let mut album_map: HashMap<String, FavoriteAlbumGroup> = HashMap::new();
    for track in &tracks {
        let entry = album_map.entry(track.album.clone()).or_insert_with(|| FavoriteAlbumGroup {
            name: track.album.clone(),
            artist: track.artist.clone(),
            artwork_data_url: track.artwork_data_url.clone(),
            tracks: Vec::new(),
        });
        if entry.artwork_data_url.is_none() && track.artwork_data_url.is_some() {
            entry.artwork_data_url = track.artwork_data_url.clone();
        }
        entry.tracks.push(track.clone());
    }

    let mut all_fav_albums: Vec<FavoriteAlbumGroup> = album_map
        .into_values()
        .filter(|alb| {
            favorite_albums.contains(&alb.name)
                || alb.tracks.iter().any(|t| favorites.contains(&t.path))
        })
        .collect();
    all_fav_albums.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let filtered_fav_albums: Vec<FavoriteAlbumGroup> = all_fav_albums
        .iter()
        .filter(|a| {
            if query.is_empty() {
                true
            } else {
                a.name.to_lowercase().contains(&query) || a.artist.to_lowercase().contains(&query)
            }
        })
        .cloned()
        .collect();

    // Compute playback pool based on current active tab
    let (tab_tracks_to_play, tab_tracks_to_shuffle) = match *active_tab.read() {
        FavoritesTab::Songs => {
            let p = filtered_fav_tracks.clone();
            let mut s = p.clone();
            s.reverse();
            (p, s)
        }
        FavoritesTab::Artists => {
            let mut p: Vec<TrackEntry> = Vec::new();
            for a in &filtered_fav_artists {
                p.extend(a.tracks.clone());
            }
            let mut s = p.clone();
            s.reverse();
            (p, s)
        }
        FavoritesTab::Albums => {
            let mut p: Vec<TrackEntry> = Vec::new();
            for a in &filtered_fav_albums {
                p.extend(a.tracks.clone());
            }
            let mut s = p.clone();
            s.reverse();
            (p, s)
        }
    };

    let total_songs_ms: u64 = filtered_fav_tracks.iter().map(|t| t.duration_ms).sum();
    let songs_dur_str = fmt_ms(total_songs_ms);

    let can_play = !tab_tracks_to_play.is_empty();

    rsx! {
        div { class: "favorites-stage",
            style: "display: flex; flex-direction: column; height: 100%; overflow: hidden;",

            // Top Header Row
            div { style: "display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 20px; flex-shrink: 0;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700; margin-bottom: 6px;", "{strings.favorites}" }
                    p { style: "color: var(--text-secondary); font-size: 13px;",
                        match *active_tab.read() {
                            FavoritesTab::Songs => rsx! {
                                "{filtered_fav_tracks.len()} {strings.favorited_tracks_in_collection} • {songs_dur_str} {strings.total_listening_time}"
                            },
                            FavoritesTab::Artists => rsx! {
                                "{filtered_fav_artists.len()} {strings.favorite_artists_in_collection}"
                            },
                            FavoritesTab::Albums => rsx! {
                                "{filtered_fav_albums.len()} {strings.favorite_albums_in_collection}"
                            },
                        }
                    }
                }

                div { style: "display: flex; gap: 10px; align-items: center;",
                    div { style: "width: 200px;",
                        input {
                            class: "form-input",
                            style: "padding: 6px 12px; font-size: 12.5px; border-radius: 20px;",
                            placeholder: "Filter favorites...",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value()),
                        }
                    }
                    button {
                        class: "btn-pill active",
                        style: "display: inline-flex; align-items: center; gap: 6px;",
                        disabled: !can_play,
                        onclick: {
                            let p_list = tab_tracks_to_play.clone();
                            move |_| on_play_tracks.call(p_list.clone())
                        },
                        span { class: "material-icons md-16", "play_arrow" }
                        "{strings.play_all}"
                    }
                    button {
                        class: "btn-pill",
                        style: "display: inline-flex; align-items: center; gap: 6px;",
                        disabled: !can_play,
                        onclick: {
                            let s_list = tab_tracks_to_shuffle.clone();
                            move |_| on_play_tracks.call(s_list.clone())
                        },
                        span { class: "material-icons md-16", "shuffle" }
                        "{strings.shuffle_all}"
                    }
                }
            }

            // Liquid Material Glass Segmented Tabs Bar
            div {
                class: "favorites-tab-bar",
                style: "display: flex; gap: 8px; margin-bottom: 18px; border-bottom: 1px solid var(--border-glass); padding-bottom: 12px; flex-shrink: 0;",
                button {
                    class: if *active_tab.read() == FavoritesTab::Songs { "btn-pill active" } else { "btn-pill" },
                    style: "display: inline-flex; align-items: center; gap: 6px; font-size: 13px; padding: 6px 14px; cursor: pointer;",
                    onclick: move |_| active_tab.set(FavoritesTab::Songs),
                    span { class: "material-icons md-16", "music_note" }
                    span { "{strings.songs}" }
                    span {
                        style: "background: rgba(255, 255, 255, 0.15); border-radius: 9999px; padding: 1px 7px; font-size: 11px; font-weight: 700; margin-left: 2px;",
                        "{all_fav_tracks.len()}"
                    }
                }
                button {
                    class: if *active_tab.read() == FavoritesTab::Artists { "btn-pill active" } else { "btn-pill" },
                    style: "display: inline-flex; align-items: center; gap: 6px; font-size: 13px; padding: 6px 14px; cursor: pointer;",
                    onclick: move |_| active_tab.set(FavoritesTab::Artists),
                    span { class: "material-icons md-16", "person" }
                    span { "{strings.artists}" }
                    span {
                        style: "background: rgba(255, 255, 255, 0.15); border-radius: 9999px; padding: 1px 7px; font-size: 11px; font-weight: 700; margin-left: 2px;",
                        "{all_fav_artists.len()}"
                    }
                }
                button {
                    class: if *active_tab.read() == FavoritesTab::Albums { "btn-pill active" } else { "btn-pill" },
                    style: "display: inline-flex; align-items: center; gap: 6px; font-size: 13px; padding: 6px 14px; cursor: pointer;",
                    onclick: move |_| active_tab.set(FavoritesTab::Albums),
                    span { class: "material-icons md-16", "album" }
                    span { "{strings.albums}" }
                    span {
                        style: "background: rgba(255, 255, 255, 0.15); border-radius: 9999px; padding: 1px 7px; font-size: 11px; font-weight: 700; margin-left: 2px;",
                        "{all_fav_albums.len()}"
                    }
                }
            }

            // Tab Content Panes
            div { style: "overflow-y: auto; flex: 1;",
                match *active_tab.read() {
                    FavoritesTab::Songs => rsx! {
                        // Songs Table Header
                        div {
                            style: "display: grid; grid-template-columns: 44px 2fr 1.5fr 1.5fr 70px 70px 70px 60px; padding: 8px 12px; border-bottom: 1px solid var(--border-glass); color: var(--text-muted); font-size: 11.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; position: sticky; top: 0; background: var(--bg-canvas); z-index: 10;",
                            span { "#" }
                            span { "{strings.title}" }
                            span { "{strings.artist}" }
                            span { "{strings.album}" }
                            span { "{strings.bpm}" }
                            span { "{strings.key}" }
                            span { style: "text-align: right;", "{strings.time}" }
                            span { style: "text-align: center;", "{strings.actions}" }
                        }

                        if filtered_fav_tracks.is_empty() {
                            div {
                                style: "display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 60px 20px; color: var(--text-muted); gap: 12px; text-align: center;",
                                span { class: "material-icons md-48", style: "opacity: 0.4;", "favorite_border" }
                                p { style: "font-size: 16px; font-weight: 600; color: var(--text-primary);", "{strings.no_favorites_yet}" }
                            }
                        } else {
                            for (idx, track) in filtered_fav_tracks.iter().enumerate() {
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
                                                span {
                                                    class: "marquee-text",
                                                    style: "color: var(--text-secondary); cursor: pointer;",
                                                    title: "{t.artist}",
                                                    onclick: {
                                                        let a_name = t.artist.clone();
                                                        move |e: MouseEvent| {
                                                            e.stop_propagation();
                                                            on_select_artist.call(a_name.clone());
                                                        }
                                                    },
                                                    "{t.artist}"
                                                }
                                            }

                                            // Album
                                            div { class: "marquee-container", style: "padding-right: 12px;",
                                                span {
                                                    class: "marquee-text",
                                                    style: "color: var(--text-muted); cursor: pointer;",
                                                    title: "{t.album}",
                                                    onclick: {
                                                        let alb_name = t.album.clone();
                                                        move |e: MouseEvent| {
                                                            e.stop_propagation();
                                                            on_select_album.call(alb_name.clone());
                                                        }
                                                    },
                                                    "{t.album}"
                                                }
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

                                            // Actions
                                            div { style: "display: flex; gap: 8px; justify-content: center; align-items: center;",
                                                button {
                                                    style: "background: transparent; border: none; cursor: pointer; color: var(--text-muted); padding: 2px;",
                                                    title: "{strings.add_to_queue}",
                                                    onclick: {
                                                        let t_clone = t.clone();
                                                        move |_| on_queue_track.call(t_clone.clone())
                                                    },
                                                    span { class: "material-icons md-16", "playlist_add" }
                                                }
                                                button {
                                                    style: "background: transparent; border: none; cursor: pointer; color: #fb7185; padding: 2px;",
                                                    title: "{strings.remove_favorite}",
                                                    onclick: move |_| on_toggle_favorite.call(path_clone.clone()),
                                                    span { class: "material-icons md-16", "favorite" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },

                    FavoritesTab::Artists => rsx! {
                        if filtered_fav_artists.is_empty() {
                            div {
                                style: "display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 60px 20px; color: var(--text-muted); gap: 12px; text-align: center;",
                                span { class: "material-icons md-48", style: "opacity: 0.4;", "person_outline" }
                                p { style: "font-size: 16px; font-weight: 600; color: var(--text-primary);", "{strings.no_favorite_artists_yet}" }
                            }
                        } else {
                            div { class: "artists-grid",
                                for artist in filtered_fav_artists.iter() {
                                    {
                                        let name = artist.name.clone();
                                        let name_click = name.clone();
                                        let name_fav = name.clone();
                                        let track_count = artist.track_count;
                                        let album_count = artist.album_count;
                                        let fallback_art = artist.fallback_artwork_url.clone();
                                        let photo_art = artist.photo_url.clone();
                                        let is_verified = artist.is_verified;

                                        rsx! {
                                            div {
                                                key: "{name}",
                                                class: "artist-grid-card",
                                                style: "position: relative;",
                                                onclick: move |_| on_select_artist.call(name_click.clone()),

                                                div { class: "artist-avatar-wrapper",
                                                    div { class: "artist-avatar-container",
                                                        // Base layer: Silhouette person icon
                                                        div { class: "artist-avatar-fallback",
                                                            span { class: "material-icons md-48", "person" }
                                                        }
                                                        // Mid layer: Local album artwork if available
                                                        if let Some(ref fallback) = fallback_art {
                                                            img {
                                                                src: "{fallback}",
                                                                alt: "",
                                                                class: "artist-avatar-img",
                                                                style: "position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: 1;",
                                                            }
                                                        }
                                                        // Top layer: Online portrait if available and distinct
                                                        if let Some(ref photo) = photo_art {
                                                            if Some(photo) != fallback_art.as_ref() {
                                                                img {
                                                                    src: "{photo}",
                                                                    alt: "",
                                                                    class: "artist-avatar-img",
                                                                    style: "position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: 2;",
                                                                }
                                                            }
                                                        }
                                                    }

                                                    // Verified checkmark badge overlay
                                                    if is_verified {
                                                        div {
                                                            class: "artist-avatar-verified-badge",
                                                            title: "{strings.verified_artist}",
                                                            span { class: "material-icons md-14", "verified" }
                                                        }
                                                    }

                                                    // Floating Favorite Heart Button
                                                    button {
                                                        style: "position: absolute; top: -2px; right: -2px; z-index: 5; background: rgba(13, 17, 24, 0.85); backdrop-filter: blur(8px); border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; cursor: pointer; color: #fb7185; transition: all 0.15s ease;",
                                                        title: "{strings.remove_favorite}",
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            on_toggle_favorite_artist.call(name_fav.clone());
                                                        },
                                                        span { class: "material-icons md-16", "favorite" }
                                                    }
                                                }

                                                div { class: "artist-card-info",
                                                    div { class: "marquee-container",
                                                        span { class: "artist-card-name marquee-text", title: "{name}", "{name}" }
                                                    }
                                                    span { class: "artist-card-meta", "{track_count} songs • {album_count} albums" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },

                    FavoritesTab::Albums => rsx! {
                        if filtered_fav_albums.is_empty() {
                            div {
                                style: "display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 60px 20px; color: var(--text-muted); gap: 12px; text-align: center;",
                                span { class: "material-icons md-48", style: "opacity: 0.4;", "album" }
                                p { style: "font-size: 16px; font-weight: 600; color: var(--text-primary);", "{strings.no_favorite_albums_yet}" }
                            }
                        } else {
                            div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 20px;",
                                for album in filtered_fav_albums.iter() {
                                    {
                                        let name = album.name.clone();
                                        let name_click = name.clone();
                                        let name_fav = name.clone();
                                        let art = album.artwork_data_url.as_deref().unwrap_or_default();
                                        let track_count = album.tracks.len();

                                        rsx! {
                                            div {
                                                key: "{album.name}",
                                                class: "album-grid-card",
                                                style: "position: relative;",
                                                onclick: move |_| on_select_album.call(name_click.clone()),

                                                div { style: "position: relative; width: 100%;",
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

                                                    // Floating Favorite Heart Button on album cover
                                                    button {
                                                        style: "position: absolute; top: 8px; right: 8px; z-index: 5; background: rgba(13, 17, 24, 0.85); backdrop-filter: blur(8px); border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 50%; width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; cursor: pointer; color: #fb7185; transition: all 0.15s ease;",
                                                        title: "{strings.remove_favorite}",
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            on_toggle_favorite_album.call(name_fav.clone());
                                                        },
                                                        span { class: "material-icons md-16", "favorite" }
                                                    }
                                                }

                                                div { style: "display: flex; flex-direction: column; overflow: hidden; width: 100%;",
                                                    div { class: "marquee-container",
                                                        span {
                                                            class: "marquee-text",
                                                            style: "font-weight: 600; font-size: 14px; color: #ffffff;",
                                                            title: "{album.name}",
                                                            "{album.name}"
                                                        }
                                                    }
                                                    div { class: "marquee-container",
                                                        span {
                                                            class: "marquee-text",
                                                            style: "color: var(--text-secondary); font-size: 12px; margin-top: 3px;",
                                                            title: "{album.artist}",
                                                            "{album.artist}"
                                                        }
                                                    }
                                                    span { style: "color: var(--text-muted); font-size: 11px; margin-top: 4px;", "{track_count} songs" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_favorites_tab_equality_and_variants() {
        assert_eq!(FavoritesTab::Songs, FavoritesTab::Songs);
        assert_ne!(FavoritesTab::Songs, FavoritesTab::Artists);
        assert_ne!(FavoritesTab::Artists, FavoritesTab::Albums);
    }

    #[test]
    fn test_favorite_album_item_grouping() {
        let t1 = TrackEntry {
            title: "Song 1".to_string(),
            artist: "Artist A".to_string(),
            album: "Album Alpha".to_string(),
            duration_ms: 180000,
            path: PathBuf::from("/music/song1.mp3"),
            ..Default::default()
        };
        let t2 = TrackEntry {
            title: "Song 2".to_string(),
            artist: "Artist A".to_string(),
            album: "Album Alpha".to_string(),
            duration_ms: 200000,
            path: PathBuf::from("/music/song2.mp3"),
            ..Default::default()
        };

        let album = FavoriteAlbumGroup {
            name: "Album Alpha".to_string(),
            artist: "Artist A".to_string(),
            artwork_data_url: None,
            tracks: vec![t1, t2],
        };

        assert_eq!(album.tracks.len(), 2);
        assert_eq!(album.name, "Album Alpha");
    }

    #[test]
    fn test_favorite_artist_item_grouping() {
        let artist = FavoriteArtistGroup {
            name: "DECO*27".to_string(),
            track_count: 12,
            album_count: 3,
            photo_url: None,
            fallback_artwork_url: None,
            is_verified: true,
            tracks: vec![],
        };

        assert_eq!(artist.name, "DECO*27");
        assert!(artist.is_verified);
        assert_eq!(artist.track_count, 12);
    }
}
