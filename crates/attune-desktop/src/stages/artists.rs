use dioxus::prelude::*;
use crate::artist_profile::{
    clean_primary_artist, extract_all_artists_from_track, extract_all_unique_artists,
    fetch_online_artist_profile, track_has_artist, OnlineArtistInfo,
};
use crate::library::{fmt_ms, TrackEntry};
use crate::stages::albums::AlbumGroup;
use crate::theme::DynamicPalette;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[component]
pub fn Artists(
    tracks: Vec<TrackEntry>,
    on_select_artist: EventHandler<String>,
) -> Element {
    let mut search_query = use_signal(String::new);
    let mut is_syncing = use_signal(|| false);
    let mut sync_status = use_signal(String::new);
    let mut refresh_trigger = use_signal(|| 0usize);

    let _ = *refresh_trigger.read();
    let all_artists = extract_all_unique_artists(&tracks);

    // Proactive background profile resolver for uncached artists (throttled)
    let uncached_artists: Vec<String> = all_artists
        .iter()
        .filter(|a| a.photo_url.is_none())
        .map(|a| a.name.clone())
        .take(12)
        .collect();

    use_effect(move || {
        let uncached = uncached_artists.clone();
        if !uncached.is_empty() {
            spawn(async move {
                let mut any_fetched = false;
                for name in uncached {
                    let n = name.clone();
                    let res = tokio::task::spawn_blocking(move || {
                        fetch_online_artist_profile(&n, false)
                    })
                    .await;
                    if let Ok(info) = res {
                        if info.image_url.is_some() || info.bio.is_some() {
                            any_fetched = true;
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                }
                if any_fetched {
                    let current = *refresh_trigger.read();
                    refresh_trigger.set(current + 1);
                }
            });
        }
    });

    let query = search_query.read().trim().to_lowercase();
    let filtered_artists: Vec<_> = all_artists
        .into_iter()
        .filter(|a| query.is_empty() || a.name.to_lowercase().contains(&query))
        .collect();
    let strings = crate::i18n::current_strings();

    rsx! {
        div { class: "artists-stage",
            div { class: "stage-header-row", style: "display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 24px;",
                div {
                    h1 { style: "font-size: 26px; font-weight: 700; margin-bottom: 6px;", "{strings.artists}" }
                    p { style: "color: var(--text-secondary); font-size: 13px;",
                        "{filtered_artists.len()} {strings.artists_in_library}"
                    }
                }
                div { style: "display: flex; gap: 12px; align-items: center;",
                    button {
                        class: "btn-glass",
                        style: "padding: 8px 14px; font-size: 13px; display: flex; align-items: center; gap: 6px; cursor: pointer;",
                        disabled: *is_syncing.read(),
                        title: "Query and cache web artist portraits (YouTube Music Innertube / Wikipedia) for your library",
                        onclick: {
                            let missing_artists: Vec<String> = filtered_artists
                                .iter()
                                .filter(|a| a.photo_url.is_none())
                                .map(|a| a.name.clone())
                                .collect();
                            move |_| {
                                if missing_artists.is_empty() {
                                    sync_status.set("All profiles are up to date!".to_string());
                                    return;
                                }
                                is_syncing.set(true);
                                let to_fetch = missing_artists.clone();
                                let total = to_fetch.len();
                                sync_status.set(format!("Fetching 0/{total}..."));
                                spawn(async move {
                                    for (idx, name) in to_fetch.into_iter().enumerate() {
                                        let n = name.clone();
                                        let n_fetch = n.clone();
                                        let _ = tokio::task::spawn_blocking(move || {
                                            fetch_online_artist_profile(&n_fetch, false)
                                        })
                                        .await;
                                        sync_status.set(format!("{}/{total} ({n})", idx + 1));
                                    }
                                    is_syncing.set(false);
                                    sync_status.set("Profiles updated!".to_string());
                                    let current = *refresh_trigger.read();
                                    refresh_trigger.set(current + 1);
                                });
                            }
                        },
                        span { class: "material-icons md-18", "cloud_sync" }
                        span {
                            if *is_syncing.read() {
                                "{sync_status}"
                            } else if !sync_status.read().is_empty() {
                                "{sync_status}"
                            } else {
                                "Fetch Web Profiles"
                            }
                        }
                    }
                    div { style: "width: 220px;",
                        input {
                            class: "form-input",
                            style: "padding: 8px 12px; font-size: 13px;",
                            placeholder: "Filter artists...",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value()),
                        }
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
                            let fallback_art = artist.fallback_artwork_url.clone();
                            let photo_art = artist.photo_url.clone();

                            rsx! {
                                div {
                                    key: "{name}",
                                    class: "artist-grid-card",
                                    onclick: move |_| on_select_artist.call(name_click.clone()),

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

    // 3. Online profile query via YouTube Music Innertube + Wikipedia REST API
    let mut online_info = use_signal(OnlineArtistInfo::default);
    let mut show_artist_options = use_signal(|| false);
    let mut is_refreshing_profile = use_signal(|| false);
    let mut copy_feedback = use_signal(|| None::<String>);
    let artist_for_fetch = artist_name.clone();

    use_effect(move || {
        let a = artist_for_fetch.clone();
        spawn(async move {
            let info = tokio::task::spawn_blocking(move || {
                fetch_online_artist_profile(&a, false)
            })
            .await
            .unwrap_or_default();
            online_info.set(info);
        });
    });

    let profile = online_info.read().clone();
    let fallback_art = artist_tracks.iter().find_map(|t| t.artwork_data_url.clone());
    let online_photo = profile.image_url.clone();

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
    let strings = crate::i18n::current_strings();

    let total_dur_ms: u64 = artist_tracks.iter().map(|t| t.duration_ms).sum();
    let total_dur_str = fmt_ms(total_dur_ms);

    let is_current_artist = current_track_path.as_ref().map(|p| artist_tracks.iter().any(|t| &t.path == p)).unwrap_or(false);
    let is_current_artist_playing = is_playing && is_current_artist;

    // Atmospheric dynamic gradient extracted from artist's tracks artwork or deterministic hash palette
    let hero_color_rgb = artist_tracks
        .iter()
        .find_map(|t| t.artwork_path.as_deref())
        .and_then(DynamicPalette::from_image)
        .map(|p| p.primary)
        .unwrap_or_else(|| {
            let seed = artist_name.bytes().fold(5381u32, |acc, b| acc.wrapping_mul(33).wrapping_add(b as u32));
            match seed % 5 {
                0 => "rgb(20, 110, 120)".to_string(), // Teal/Cyan
                1 => "rgb(112, 38, 115)".to_string(), // Purple/Violet
                2 => "rgb(120, 60, 20)".to_string(),  // Amber/Warm Gold
                3 => "rgb(30, 90, 60)".to_string(),   // Emerald
                _ => "rgb(65, 80, 115)".to_string(),  // Deep Blue
            }
        });

    let lossless_count = artist_tracks.iter().filter(|t| {
        let f = t.format.to_uppercase();
        f == "FLAC" || f == "WAV" || f == "ALAC" || f == "AIFF"
    }).count();
    let format_desc = if lossless_count > 0 { "FLAC" } else { "lossy" };
    let summary_description = format!(
        "Collection of {} tracks by {}. {} lossless tracks ({}). Tempo profiling pending audio transition scan. Harmonic chroma pending CQT analysis. Notes and background lore can be edited or fetched dynamically.",
        total_tracks,
        artist_name,
        lossless_count,
        format_desc,
    );

    // Track Context Menu & Credits Modal State
    let mut active_menu_track = use_signal(|| None::<TrackEntry>);
    let mut show_credits_track = use_signal(|| None::<TrackEntry>);

    let hero_bg_style = if let Some(ref banner) = profile.banner_url {
        format!(
            "background: linear-gradient(180deg, rgba(18, 18, 18, 0.40) 0%, rgba(18, 18, 18, 0.85) 280px, var(--bg-canvas) 460px), url('{banner}') top center / cover no-repeat;"
        )
    } else {
        format!(
            "background: linear-gradient(180deg, {hero_color_rgb} 0%, rgba(18, 18, 18, 0.72) 280px, var(--bg-canvas) 460px);"
        )
    };

    rsx! {
        div {
            class: "artist-detail-view",
            style: "{hero_bg_style}",
            onclick: move |_| {
                active_menu_track.set(None);
                show_artist_options.set(false);
            },

            // 1. Hero Header: 232px Artist Avatar on left + Big Metadata on right
            div { class: "album-detail-hero",
                div { class: "artist-avatar-box",
                    // Base layer: Silhouette person icon
                    div { class: "album-cover-fallback",
                        span { class: "material-icons md-64", "person" }
                    }
                    // Mid layer: Local album artwork if available
                    if let Some(ref fallback) = fallback_art {
                        img {
                            class: "album-cover-img",
                            src: "{fallback}",
                            alt: "",
                            style: "position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: 1;",
                        }
                    }
                    // Top layer: Online portrait if available and distinct
                    if let Some(ref photo) = online_photo {
                        if Some(photo) != fallback_art.as_ref() {
                            img {
                                class: "album-cover-img",
                                src: "{photo}",
                                alt: "",
                                style: "position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: 2;",
                            }
                        }
                    }
                }

                div { class: "album-meta-col",
                    span { class: "album-meta-type", "{strings.artist}" }
                    h1 { class: "album-meta-title", "{artist_name}" }
                    div { class: "album-meta-subline",
                        div { class: "album-artist-avatar",
                            span { class: "material-icons md-14", style: "color: var(--accent-mint);", "verified" }
                            if let Some(ref fallback) = fallback_art {
                                img {
                                    src: "{fallback}",
                                    alt: "",
                                    style: "position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: 1;",
                                }
                            }
                            if let Some(ref photo) = online_photo {
                                if Some(photo) != fallback_art.as_ref() {
                                    img {
                                        src: "{photo}",
                                        alt: "",
                                        style: "position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: 2;",
                                    }
                                }
                            }
                        }
                        span { class: "album-artist-name", "{strings.verified_artist}" }
                        if let Some(ref subs) = profile.subscribers {
                            span { class: "album-dot-sep", "•" }
                            span { class: "album-meta-dur", "{subs} subscribers" }
                        }
                        span { class: "album-dot-sep", "•" }
                        span { class: "album-meta-songs", "{total_tracks} songs," }
                        span { class: "album-meta-dur", "{total_dur_str}" }
                        span { class: "album-dot-sep", "•" }
                        span { class: "album-meta-dur", "{total_albums} albums" }
                    }
                }
            }

            // 2. Action Bar: Big green Play button, Shuffle, Favorite, Add to Queue
            div { class: "album-action-bar",
                button {
                    class: "album-big-play-btn",
                    title: if is_current_artist_playing { "Pause" } else { "Play all songs by this artist" },
                    onclick: {
                        let all_tracks = artist_tracks.clone();
                        move |_| {
                            if is_current_artist_playing {
                                on_toggle_play.call(());
                            } else {
                                on_play_tracks.call(all_tracks.clone());
                            }
                        }
                    },
                    if is_current_artist_playing {
                        span { class: "material-icons md-32", "pause" }
                    } else {
                        span { class: "material-icons md-32", "play_arrow" }
                    }
                }

                button {
                    class: "album-icon-btn",
                    title: "Shuffle artist tracks",
                    onclick: {
                        let tracks_to_shuffle = artist_tracks.clone();
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
                    class: if artist_tracks.iter().all(|t| favorites.contains(&t.path)) && !artist_tracks.is_empty() {
                        "album-icon-btn active"
                    } else {
                        "album-icon-btn"
                    },
                    title: "Favorite All Tracks by Artist",
                    onclick: {
                        let all_paths: Vec<PathBuf> = artist_tracks.iter().map(|t| t.path.clone()).collect();
                        move |_| {
                            for p in &all_paths {
                                on_toggle_favorite.call(p.clone());
                            }
                        }
                    },
                    if artist_tracks.iter().all(|t| favorites.contains(&t.path)) && !artist_tracks.is_empty() {
                        span { class: "material-icons md-28", style: "color: #fb7185;", "favorite" }
                    } else {
                        span { class: "material-icons md-28", "favorite_border" }
                    }
                }

                div {
                    style: "position: relative; display: inline-block;",
                    button {
                        class: if *show_artist_options.read() { "album-icon-btn active" } else { "album-icon-btn" },
                        title: "Artist options",
                        onclick: move |e: MouseEvent| {
                            e.stop_propagation();
                            let cur = *show_artist_options.read();
                            show_artist_options.set(!cur);
                        },
                        span { class: "material-icons md-28", "more_horiz" }
                    }

                    if *show_artist_options.read() {
                        div {
                            class: "track-context-menu",
                            style: "top: 48px; left: 0; right: auto; min-width: 250px; z-index: 100;",
                            onclick: move |e: MouseEvent| e.stop_propagation(),

                            // 1. Add All to Queue
                            button {
                                class: "track-menu-item",
                                onclick: {
                                    let all_tracks = artist_tracks.clone();
                                    move |_| {
                                        show_artist_options.set(false);
                                        for t in &all_tracks {
                                            on_queue_track.call(t.clone());
                                        }
                                    }
                                },
                                span { class: "material-icons md-16", "playlist_add" }
                                span { "{strings.add_to_queue}" }
                            }

                            // 2. Play Next
                            button {
                                class: "track-menu-item",
                                onclick: {
                                    let all_tracks = artist_tracks.clone();
                                    move |_| {
                                        show_artist_options.set(false);
                                        for t in all_tracks.iter().rev() {
                                            on_play_next.call(t.clone());
                                        }
                                    }
                                },
                                span { class: "material-icons md-16", "skip_next" }
                                span { "{strings.play_next}" }
                            }

                            div { class: "track-menu-divider" }

                            // 3. Query / Refresh Profile from Web
                            button {
                                class: "track-menu-item",
                                disabled: *is_refreshing_profile.read(),
                                onclick: {
                                    let a_name = artist_name.clone();
                                    move |_| {
                                        is_refreshing_profile.set(true);
                                        let a = a_name.clone();
                                        spawn(async move {
                                            let refreshed = tokio::task::spawn_blocking(move || {
                                                fetch_online_artist_profile(&a, true)
                                            })
                                            .await
                                            .unwrap_or_default();
                                            online_info.set(refreshed);
                                            is_refreshing_profile.set(false);
                                            show_artist_options.set(false);
                                        });
                                    }
                                },
                                span { class: "material-icons md-16", "sync" }
                                span {
                                    if *is_refreshing_profile.read() {
                                        "Querying Web Profile..."
                                    } else {
                                        "Query / Refresh Profile from Web"
                                    }
                                }
                            }

                            // 4. Copy Artist Name
                            button {
                                class: "track-menu-item",
                                onclick: {
                                    let a_name = artist_name.clone();
                                    move |_| {
                                        show_artist_options.set(false);
                                        copy_feedback.set(Some(format!("Copied '{a_name}'!")));
                                        spawn({
                                            let mut fb = copy_feedback.clone();
                                            async move {
                                                tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
                                                fb.set(None);
                                            }
                                        });
                                        #[cfg(target_os = "linux")]
                                        {
                                            use std::io::Write;
                                            if let Ok(mut child) = std::process::Command::new("wl-copy")
                                                .stdin(std::process::Stdio::piped())
                                                .spawn()
                                            {
                                                if let Some(mut stdin) = child.stdin.take() {
                                                    let _ = stdin.write_all(a_name.as_bytes());
                                                }
                                            } else if let Ok(mut child) = std::process::Command::new("xclip")
                                                .args(["-selection", "clipboard"])
                                                .stdin(std::process::Stdio::piped())
                                                .spawn()
                                            {
                                                if let Some(mut stdin) = child.stdin.take() {
                                                    let _ = stdin.write_all(a_name.as_bytes());
                                                }
                                            }
                                        }
                                    }
                                },
                                span { class: "material-icons md-16", "content_copy" }
                                span { "Copy Artist Name" }
                            }

                            div { class: "track-menu-divider" }

                            // 5. Exclude All Artist Songs from Library
                            button {
                                class: "track-menu-item danger",
                                onclick: {
                                    let all_tracks = artist_tracks.clone();
                                    move |_| {
                                        show_artist_options.set(false);
                                        for t in &all_tracks {
                                            crate::config::add_song_exclusion(t.path.to_string_lossy().to_string());
                                        }
                                    }
                                },
                                span { class: "material-icons md-16", style: "color: #fb7185;", "block" }
                                span { style: "color: #fb7185;", "Exclude Artist from Library" }
                            }
                        }
                    }
                }
            }

            // 3. Tracklist Table Container (Identical to Album view with 32px side padding)
            div { class: "album-tracklist-container",
                div { class: "album-table-header",
                    span { class: "album-table-header-idx", "#" }
                    span { class: "album-table-header-title", "{strings.title}" }
                    span { class: "album-table-header-audio", "{strings.audio_profile}" }
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
                                            span {
                                                class: "album-row-artist-text",
                                                style: "cursor: pointer;",
                                                title: "Go to album {t.album}",
                                                onclick: {
                                                    let alb = t.album.clone();
                                                    move |evt: MouseEvent| {
                                                        evt.stop_propagation();
                                                        on_select_album.call(alb.clone());
                                                    }
                                                },
                                                "{t.album}"
                                            }
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
                                                        span { "{strings.play_now}" }
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
                                                        span { "{strings.play_next}" }
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
                                                        span { "{strings.add_to_queue}" }
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
                                                        span { if is_fav { "{strings.remove_favorite}" } else { "{strings.add_to_favorites}" } }
                                                    }

                                                    div { class: "track-menu-divider" }

                                                    for other in extract_all_artists_from_track(&t) {
                                                        if crate::artist_profile::canonical_artist_key(&other) != crate::artist_profile::canonical_artist_key(&artist_name) {
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
                                                        span { "{strings.view_track_credits}" }
                                                    }

                                                    button {
                                                        class: "track-menu-item",
                                                        style: "color: #fb7185;",
                                                        onclick: {
                                                            let t_p = t.path.to_string_lossy().to_string();
                                                            move |_| {
                                                                active_menu_track.set(None);
                                                                crate::config::add_song_exclusion(t_p.clone());
                                                            }
                                                        },
                                                        span { class: "material-icons md-16", style: "color: #fb7185;", "block" }
                                                        span { "{strings.exclude_from_library}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                // 4. Collection Footer Note & Dynamic Lore
                div { class: "album-detail-footer",
                    p { class: "album-footer-lore", "{summary_description}" }
                }

                // 5. Artist Discography Section with Release Type Filter Chips
                div { class: "album-discography-section",
                    div { class: "discography-header-row",
                        div { style: "display: flex; align-items: center; gap: 16px; flex-wrap: wrap;",
                            h2 { class: "discography-title", "{strings.discography}" }
                            div { class: "discography-chips",
                                button {
                                    class: if active_filter == "all" { "discography-chip active" } else { "discography-chip" },
                                    onclick: move |_| discography_filter.set("all".to_string()),
                                    "{strings.popular_releases}"
                                }
                                button {
                                    class: if active_filter == "albums" { "discography-chip active" } else { "discography-chip" },
                                    onclick: move |_| discography_filter.set("albums".to_string()),
                                    "{strings.albums}"
                                }
                                button {
                                    class: if active_filter == "singles" { "discography-chip active" } else { "discography-chip" },
                                    onclick: move |_| discography_filter.set("singles".to_string()),
                                    "{strings.singles_and_eps}"
                                }
                            }
                        }
                        span { class: "discography-count", "{filtered_albums.len()} {strings.releases}" }
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

                // 6. About Artist (YouTube Music Innertube / Wikipedia Bio & Description)
                if let Some(ref bio) = profile.bio {
                    div { class: "artist-bio-card",
                        h3 { style: "font-size: 16px; font-weight: 700; margin-bottom: 10px; display: flex; align-items: center; gap: 8px;",
                            span { class: "material-icons md-20", style: "color: var(--accent-mint);", "menu_book" }
                            "About {artist_name}"
                        }
                        if let Some(ref desc) = profile.description {
                            p { style: "color: var(--accent-mint); font-size: 13.5px; font-weight: 600; margin-bottom: 8px;", "{desc}" }
                        }
                        p { class: "artist-bio-text", "{bio}" }
                        if profile.subscribers.is_some() || profile.views.is_some() {
                            div { style: "margin-top: 14px; padding-top: 10px; border-top: 1px solid var(--border-glass); display: flex; gap: 20px; font-size: 12.5px; color: var(--text-muted); align-items: center;",
                                if let Some(ref subs) = profile.subscribers {
                                    div { style: "display: flex; align-items: center; gap: 6px;",
                                        span { class: "material-icons md-16", style: "color: #f43f5e;", "smart_display" }
                                        span { "{subs} subscribers" }
                                    }
                                }
                                if let Some(ref views) = profile.views {
                                    div { style: "display: flex; align-items: center; gap: 6px;",
                                        span { class: "material-icons md-16", "visibility" }
                                        span { "{views}" }
                                    }
                                }
                                if let Some(ref b_id) = profile.browse_id {
                                    div { style: "font-size: 11px; color: var(--text-muted); opacity: 0.7;",
                                        "ID: {b_id}"
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

            if let Some(msg) = copy_feedback.read().as_ref() {
                div {
                    class: "copy-feedback-toast",
                    span { class: "material-icons md-16", "check_circle" }
                    span { "{msg}" }
                }
            }
        }
    }
}
