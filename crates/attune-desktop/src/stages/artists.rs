use dioxus::prelude::*;
use crate::artist_profile::{
    clean_primary_artist, extract_all_artists_from_track, extract_all_unique_artists,
    fetch_wikipedia_artist_profile, track_has_artist, OnlineArtistInfo,
};
use crate::library::{fmt_ms, TrackEntry};
use crate::stages::albums::AlbumGroup;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[component]
pub fn Artists(
    tracks: Vec<TrackEntry>,
    on_select_artist: EventHandler<String>,
) -> Element {
    let mut search_query = use_signal(String::new);
    let all_artists = extract_all_unique_artists(&tracks);

    let query = search_query.read().trim().to_lowercase();
    let filtered_artists: Vec<_> = all_artists
        .into_iter()
        .filter(|a| query.is_empty() || a.name.to_lowercase().contains(&query))
        .collect();

    rsx! {
        div { class: "artists-stage",
            div { class: "stage-header-row", style: "display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700; margin-bottom: 6px;", "Artists" }
                    p { style: "color: var(--text-secondary); font-size: 13px;",
                        "{filtered_artists.len()} artists in your library"
                    }
                }
                div { style: "width: 240px;",
                    input {
                        class: "form-input",
                        style: "padding: 8px 12px; font-size: 13px;",
                        placeholder: "Filter artists...",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value()),
                    }
                }
            }

            if filtered_artists.is_empty() {
                div { style: "background: var(--bg-card); border: 1px solid var(--border-glass); border-radius: 12px; padding: 36px; text-align: center; color: var(--text-muted);",
                    "No artists match your search query."
                }
            } else {
                div { class: "artists-grid",
                    for artist in filtered_artists.iter() {
                        {
                            let name = artist.name.clone();
                            let name_click = name.clone();
                            let track_count = artist.track_count;
                            let album_count = artist.album_count;
                            let art_url = artist.fallback_artwork_url.clone();

                            rsx! {
                                div {
                                    key: "{name}",
                                    class: "artist-grid-card",
                                    onclick: move |_| on_select_artist.call(name_click.clone()),

                                    div { class: "artist-avatar-container",
                                        if let Some(ref art) = art_url {
                                            img {
                                                src: "{art}",
                                                alt: "{name}",
                                                class: "artist-avatar-img",
                                            }
                                        } else {
                                            div { class: "artist-avatar-fallback",
                                                span { class: "material-icons md-48", "person" }
                                            }
                                        }
                                    }

                                    div { class: "artist-card-info",
                                        span { class: "artist-card-name", title: "{name}", "{name}" }
                                        span { class: "artist-card-meta", "{track_count} songs • {album_count} albums" }
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

#[component]
pub fn ArtistDetail(
    artist_name: String,
    tracks: Vec<TrackEntry>,
    current_track_path: Option<PathBuf>,
    is_playing: bool,
    #[props(default)]
    favorites: HashSet<PathBuf>,
    on_select_album: EventHandler<String>,
    #[props(default)]
    on_select_artist: EventHandler<String>,
    on_play_track: EventHandler<TrackEntry>,
    on_play_tracks: EventHandler<Vec<TrackEntry>>,
    on_queue_track: EventHandler<TrackEntry>,
    #[props(default)]
    on_play_next: EventHandler<TrackEntry>,
    #[props(default)]
    on_toggle_favorite: EventHandler<PathBuf>,
    #[props(default)]
    on_toggle_play: EventHandler<()>,
) -> Element {
    // 1. Gather all tracks featuring or by this artist using multi-artist pinpointing
    let artist_tracks: Vec<TrackEntry> = tracks
        .iter()
        .filter(|t| track_has_artist(t, &artist_name))
        .cloned()
        .collect();

    // 2. Group into albums by this artist
    let mut album_map: HashMap<String, AlbumGroup> = HashMap::new();
    for track in &artist_tracks {
        let entry = album_map.entry(track.album.clone()).or_insert_with(|| AlbumGroup {
            name: track.album.clone(),
            artist: artist_name.clone(),
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
        entry.tracks.push(track.clone());
    }

    let mut artist_albums: Vec<AlbumGroup> = album_map.into_values().collect();
    artist_albums.sort_by(|a, b| b.tracks.len().cmp(&a.tracks.len()).then_with(|| a.name.cmp(&b.name)));

    // 3. Online profile query via Wikipedia REST API
    let mut online_info = use_signal(OnlineArtistInfo::default);
    let artist_for_fetch = artist_name.clone();

    use_effect(move || {
        let a = artist_for_fetch.clone();
        spawn(async move {
            let info = tokio::task::spawn_blocking(move || {
                fetch_wikipedia_artist_profile(&a)
            })
            .await
            .unwrap_or_default();
            online_info.set(info);
        });
    });

    let profile = online_info.read().clone();
    let effective_photo = profile.image_url.clone().or_else(|| {
        artist_tracks.iter().find_map(|t| t.artwork_data_url.clone())
    });

    // 4. Discography Filter state for Artist View: "all", "albums", "singles"
    let mut discography_filter = use_signal(|| "all".to_string());
    let active_filter = discography_filter.read().clone();

    let filtered_albums: Vec<AlbumGroup> = artist_albums
        .clone()
        .into_iter()
        .filter(|a| match active_filter.as_str() {
            "albums" => a.tracks.len() > 5,
            "singles" => a.tracks.len() <= 5,
            _ => true,
        })
        .collect();

    let total_tracks = artist_tracks.len();
    let total_albums = artist_albums.len();

    // Track Context Menu & Credits Modal State
    let mut active_menu_track = use_signal(|| None::<TrackEntry>);
    let mut show_credits_track = use_signal(|| None::<TrackEntry>);

    rsx! {
        div { class: "artist-detail-view",
            // Hero Section
            div { class: "artist-hero",
                div { class: "artist-hero-avatar-box",
                    if let Some(ref photo) = effective_photo {
                        img { class: "artist-hero-avatar-img", src: "{photo}", alt: "{artist_name}" }
                    } else {
                        div { class: "artist-hero-avatar-fallback",
                            span { class: "material-icons md-64", "person" }
                        }
                    }
                }

                div { class: "artist-hero-info",
                    span { class: "artist-hero-tag", "ARTIST" }
                    h1 { class: "artist-hero-title", "{artist_name}" }

                    if let Some(ref desc) = profile.description {
                        p { class: "artist-hero-desc", "{desc}" }
                    }

                    div { class: "artist-hero-meta",
                        span { "{total_tracks} songs in your library" }
                        span { "•" }
                        span { "{total_albums} albums" }
                    }

                    // Action buttons
                    div { style: "display: flex; gap: 14px; align-items: center; margin-top: 16px;",
                        button {
                            class: "album-big-play-btn",
                            title: "Play all tracks by this artist",
                            onclick: {
                                let tracks_to_play = artist_tracks.clone();
                                move |_| on_play_tracks.call(tracks_to_play.clone())
                            },
                            span { class: "material-icons md-32", "play_arrow" }
                        }
                        button {
                            class: "album-icon-btn",
                            title: "Shuffle artist tracks",
                            onclick: {
                                let mut shuffled = artist_tracks.clone();
                                move |_| {
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
                                    on_play_tracks.call(shuffled.clone());
                                }
                            },
                            span { class: "material-icons md-28", "shuffle" }
                        }
                    }
                }
            }

            // Expandable Bio from Wikipedia REST API if available
            if let Some(ref bio) = profile.bio {
                div { class: "artist-bio-card",
                    h3 { style: "font-size: 15px; font-weight: 700; margin-bottom: 8px; display: flex; align-items: center; gap: 6px;",
                        span { class: "material-icons md-18", style: "color: var(--accent-mint);", "menu_book" }
                        "About {artist_name}"
                    }
                    p { class: "artist-bio-text", "{bio}" }
                }
            }

            // Artist Discography Section with Release Type Filter Chips
            div { class: "album-discography-section", style: "margin-top: 28px;",
                div { class: "discography-header-row",
                    div { style: "display: flex; align-items: center; gap: 16px; flex-wrap: wrap;",
                        h2 { class: "discography-title", "Discography" }
                        div { class: "discography-chips",
                            button {
                                class: if active_filter == "all" { "discography-chip active" } else { "discography-chip" },
                                onclick: move |_| discography_filter.set("all".to_string()),
                                "Popular releases"
                            }
                            button {
                                class: if active_filter == "albums" { "discography-chip active" } else { "discography-chip" },
                                onclick: move |_| discography_filter.set("albums".to_string()),
                                "Albums"
                            }
                            button {
                                class: if active_filter == "singles" { "discography-chip active" } else { "discography-chip" },
                                onclick: move |_| discography_filter.set("singles".to_string()),
                                "Singles and EPs"
                            }
                        }
                    }
                    span { class: "discography-count", "{filtered_albums.len()} releases" }
                }

                if filtered_albums.is_empty() {
                    div { class: "discography-empty", "No releases found matching the selected category." }
                } else {
                    div { class: "discography-shelf",
                        for album in filtered_albums.iter() {
                            {
                                let a_name = album.name.clone();
                                let a_name_click = album.name.clone();
                                let a_art = album.artwork_data_url.clone();
                                let a_tracks = album.tracks.clone();
                                let track_len = album.tracks.len();

                                let rel_type = if track_len == 1 {
                                    "Single".to_string()
                                } else if track_len <= 5 {
                                    "EP".to_string()
                                } else {
                                    "Album".to_string()
                                };

                                let year_str = album.tracks.first().and_then(|_t| {
                                    let re_year = album.name.split('(').nth(1)?.split(')').next()?;
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

                                            button {
                                                class: "discography-hover-play",
                                                title: "Play album",
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

            // All Songs Table by this Artist
            div { style: "margin-top: 36px;",
                h2 { style: "font-size: 20px; font-weight: 700; margin-bottom: 16px;", "All Songs" }

                div { class: "album-tracklist-container", style: "padding: 0;",
                    div { class: "album-table-header",
                        span { class: "album-table-header-idx", "#" }
                        span { class: "album-table-header-title", "Title" }
                        span { class: "album-table-header-audio", "Audio Profile" }
                        div { class: "album-table-header-dur",
                            span { class: "material-icons md-18", "schedule" }
                        }
                    }

                    div { class: "album-table-body",
                        for (idx, track) in artist_tracks.iter().enumerate() {
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
                                let is_menu_open = active_menu_track.read().as_ref().map(|m| m.path == t.path).unwrap_or(false);

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
                                        style: "position: relative;",
                                        ondoubleclick: move |_| on_play_track.call(t_for_row.clone()),

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

                                        div { class: "album-row-title-col",
                                            span {
                                                class: if is_current { "album-row-title-text active" } else { "album-row-title-text" },
                                                "{t.title}"
                                            }
                                            span { class: "album-row-artist-text", "{t.album}" }
                                        }

                                        div { class: "album-row-audio-col", "{audio_details}" }

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

                                            // Track "..." Menu Trigger
                                            button {
                                                class: "album-row-menu-btn",
                                                title: "Track Actions",
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

                                                    for other in extract_all_artists_from_track(&t) {
                                                        if other.to_lowercase() != artist_name.to_lowercase() {
                                                            {
                                                                let other_name = other.clone();
                                                                rsx! {
                                                                    button {
                                                                        key: "{other_name}",
                                                                        class: "track-menu-item",
                                                                        onclick: {
                                                                            let a_click = other_name.clone();
                                                                            move |_| {
                                                                                active_menu_track.set(None);
                                                                                on_select_artist.call(a_click.clone());
                                                                            }
                                                                        },
                                                                        span { class: "material-icons md-16", "person" }
                                                                        span { "Go to {other_name}" }
                                                                    }
                                                                }
                                                            }
                                                        }
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
                }
            }

            // Track Credits & Metadata Modal
            if let Some(track) = show_credits_track.read().clone() {
                {
                    let dur_str = fmt_ms(track.duration_ms);
                    let vocalist_str = track.vocalist.clone().unwrap_or_else(|| {
                        if track.artist.contains("feat.") || track.artist.contains("ft.") {
                            clean_primary_artist(&track.artist)
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
