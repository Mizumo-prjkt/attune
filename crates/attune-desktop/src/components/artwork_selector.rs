use std::path::PathBuf;
use dioxus::prelude::*;
use crate::dynamic_components::music_metadata_changer::{
    download_youtube_thumbnail, search_youtube_thumbnails, YouTubeThumbnailCandidate,
};
use crate::library::resolve_thumbnail_data_url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkSourceTab {
    YouTube,
    LocalFolder,
    BrowseFile,
}

#[component]
pub fn ArtworkSelectorModal(
    title: String,
    artist: String,
    album: String,
    track_path: Option<PathBuf>,
    current_artwork_url: Option<String>,
    is_album_level: bool,
    on_close: EventHandler<()>,
    on_artwork_applied: EventHandler<(PathBuf, String)>,
) -> Element {
    let mut active_tab = use_signal(|| ArtworkSourceTab::YouTube);
    let initial_query = if is_album_level && !album.is_empty() && album != "Unknown Album" {
        format!("{album} {artist} album")
    } else {
        format!("{title} {artist}")
    };
    let mut search_query = use_signal(|| initial_query.trim().to_string());
    let mut is_searching = use_signal(|| false);
    let mut yt_candidates = use_signal(Vec::<YouTubeThumbnailCandidate>::new);
    let mut selected_yt = use_signal(|| None::<YouTubeThumbnailCandidate>);
    let mut local_folder_images = use_signal(Vec::<PathBuf>::new);
    let mut selected_local_path = use_signal(|| None::<PathBuf>);
    let mut preview_url = use_signal(|| current_artwork_url.clone());
    let mut is_applying = use_signal(|| false);
    let mut status_msg = use_signal(|| None::<(bool, String)>);

    // Initial search & local folder scan
    use_hook({
        let path_opt = track_path.clone();
        let query_str = search_query.read().clone();
        move || {
            // 1. Scan parent folder for local images
            if let Some(ref p) = path_opt {
                if let Some(parent) = p.parent() {
                    let mut found = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                    let ext_lower = ext.to_lowercase();
                                    if ext_lower == "jpg" || ext_lower == "jpeg" || ext_lower == "png" || ext_lower == "webp" {
                                        found.push(path);
                                    }
                                }
                            }
                        }
                    }
                    found.sort_by(|a, b| {
                        let a_name = a.file_name().and_then(|f| f.to_str()).unwrap_or("").to_lowercase();
                        let b_name = b.file_name().and_then(|f| f.to_str()).unwrap_or("").to_lowercase();
                        let a_priority = a_name.starts_with("cover") || a_name.starts_with("folder") || a_name.starts_with("front");
                        let b_priority = b_name.starts_with("cover") || b_name.starts_with("folder") || b_name.starts_with("front");
                        b_priority.cmp(&a_priority).then_with(|| a_name.cmp(&b_name))
                    });
                    local_folder_images.set(found);
                }
            }

            // 2. Perform initial YouTube search in background task
            if !query_str.is_empty() {
                is_searching.set(true);
                spawn(async move {
                    match search_youtube_thumbnails(&query_str) {
                        Ok(cands) => {
                            yt_candidates.set(cands);
                            is_searching.set(false);
                        }
                        Err(e) => {
                            log::warn!("ArtworkSelector: Initial YouTube search error: {e}");
                            is_searching.set(false);
                        }
                    }
                });
            }
        }
    });

    let trigger_search = {
        let mut is_searching = is_searching.clone();
        let mut status_msg = status_msg.clone();
        let yt_candidates = yt_candidates.clone();
        let search_query = search_query.clone();
        move || {
            let q = search_query.read().trim().to_string();
            if q.is_empty() {
                return;
            }
            is_searching.set(true);
            status_msg.set(None);
            let mut yt_cands = yt_candidates.clone();
            let mut is_s = is_searching.clone();
            let mut s_msg = status_msg.clone();
            spawn(async move {
                match search_youtube_thumbnails(&q) {
                    Ok(cands) => {
                        yt_cands.set(cands);
                        is_s.set(false);
                    }
                    Err(e) => {
                        s_msg.set(Some((false, format!("YouTube Search Error: {e}"))));
                        is_s.set(false);
                    }
                }
            });
        }
    };

    let local_count = local_folder_images.read().len();

    rsx! {
        div {
            class: "artwork-selector-overlay",
            style: "position: fixed; inset: 0; z-index: 9999; background: rgba(0, 0, 0, 0.78); backdrop-filter: blur(24px); -webkit-backdrop-filter: blur(24px); display: flex; align-items: center; justify-content: center; padding: 24px; animation: fadeIn 0.15s ease-out;",
            onclick: move |_| on_close.call(()),

            div {
                class: "artwork-selector-modal",
                style: "background: rgba(18, 24, 38, 0.95); border: 1px solid var(--border-glass); border-radius: 20px; box-shadow: 0 24px 60px rgba(0,0,0,0.8); width: 100%; max-width: 900px; max-height: 85vh; display: flex; flex-direction: column; overflow: hidden;",
                onclick: move |e: MouseEvent| e.stop_propagation(),

                // Header
                div {
                    style: "padding: 20px 24px; border-bottom: 1px solid var(--border-glass); display: flex; justify-content: space-between; align-items: center;",
                    div {
                        h2 { style: "font-size: 20px; font-weight: 700; margin: 0; display: flex; align-items: center; gap: 8px;",
                            span { class: "material-icons md-22", style: "color: var(--accent-mint);", "image_search" }
                            if is_album_level { "Select Album Artwork" } else { "Select Song Artwork" }
                        }
                        p { style: "font-size: 12.5px; color: var(--text-muted); margin: 4px 0 0 0;",
                            if is_album_level {
                                "Pick or fetch high-resolution cover for album \"{album}\" by {artist}"
                            } else {
                                "Correct or replace cover for \"{title}\" by {artist}"
                            }
                        }
                    }

                    button {
                        class: "btn-pill",
                        style: "padding: 6px 14px; font-size: 12px;",
                        onclick: move |_| on_close.call(()),
                        span { class: "material-icons md-16", "close" }
                        "Close"
                    }
                }

                // Source Tabs Bar
                div {
                    style: "display: flex; gap: 8px; padding: 12px 24px; background: rgba(0,0,0,0.25); border-bottom: 1px solid var(--border-glass); align-items: center;",
                    button {
                        class: if *active_tab.read() == ArtworkSourceTab::YouTube { "btn-pill active" } else { "btn-pill" },
                        onclick: move |_| active_tab.set(ArtworkSourceTab::YouTube),
                        span { class: "material-icons md-16", "smart_display" }
                        "YouTube & YouTube Music (HQ)"
                    }
                    button {
                        class: if *active_tab.read() == ArtworkSourceTab::LocalFolder { "btn-pill active" } else { "btn-pill" },
                        onclick: move |_| active_tab.set(ArtworkSourceTab::LocalFolder),
                        span { class: "material-icons md-16", "folder" }
                        "Local Folder Images ({local_count})"
                    }
                    button {
                        class: if *active_tab.read() == ArtworkSourceTab::BrowseFile { "btn-pill active" } else { "btn-pill" },
                        onclick: move |_| {
                            active_tab.set(ArtworkSourceTab::BrowseFile);
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                                .pick_file()
                            {
                                selected_local_path.set(Some(path.clone()));
                                selected_yt.set(None);
                                if let Some(url) = resolve_thumbnail_data_url(&path) {
                                    preview_url.set(Some(url));
                                }
                            }
                        },
                        span { class: "material-icons md-16", "file_open" }
                        "Browse Device File..."
                    }
                }

                // Main Content Body: Split Left (Picker) and Right (Live Preview Card)
                div {
                    style: "display: grid; grid-template-columns: 1fr 280px; flex: 1; min-height: 0; overflow: hidden;",

                    // Left Column: Candidate List / Search
                    div {
                        style: "padding: 20px 24px; overflow-y: auto; display: flex; flex-direction: column; gap: 16px;",

                        match *active_tab.read() {
                            ArtworkSourceTab::YouTube => rsx! {
                                // Search Input Box
                                div { style: "display: flex; gap: 10px;",
                                    input {
                                        class: "search-bar",
                                        style: "flex: 1; padding: 9px 14px; border-radius: 10px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff; font-size: 13.5px;",
                                        placeholder: "Search artist, song, or album on YouTube...",
                                        value: "{search_query}",
                                        oninput: move |e| search_query.set(e.value()),
                                        onkeydown: {
                                            let mut on_enter = trigger_search.clone();
                                            move |e: KeyboardEvent| {
                                                if e.key() == dioxus::prelude::Key::Enter {
                                                    on_enter();
                                                }
                                            }
                                        },
                                    }
                                    button {
                                        class: "btn-pill active",
                                        style: "padding: 9px 18px; font-weight: 600; display: inline-flex; align-items: center; gap: 6px;",
                                        disabled: *is_searching.read(),
                                        onclick: {
                                            let mut on_click = trigger_search.clone();
                                            move |_| on_click()
                                        },
                                        if *is_searching.read() {
                                            span { class: "material-icons md-16", "sync" }
                                            "Searching..."
                                        } else {
                                            span { class: "material-icons md-16", "search" }
                                            "Search"
                                        }
                                    }
                                }

                                // Candidates Grid
                                if *is_searching.read() {
                                    div { style: "padding: 40px 0; text-align: center; color: var(--text-muted);",
                                        span { class: "material-icons md-36", style: "animation: spin 1s infinite linear; margin-bottom: 8px;", "refresh" }
                                        p { "Querying YouTube Music & YouTube for high-resolution artwork..." }
                                    }
                                } else if yt_candidates.read().is_empty() {
                                    div { style: "padding: 40px 0; text-align: center; color: var(--text-muted);",
                                        span { class: "material-icons md-36", style: "opacity: 0.5; margin-bottom: 8px;", "image_not_supported" }
                                        p { "No candidate thumbnails found. Try refining the search query above." }
                                    }
                                } else {
                                    div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: 14px;",
                                        for cand in yt_candidates.read().iter() {
                                            {
                                                let c = cand.clone();
                                                let is_sel = selected_yt.read().as_ref().map(|s| s.thumbnail_url == c.thumbnail_url).unwrap_or(false);
                                                rsx! {
                                                    div {
                                                        key: "{c.thumbnail_url}",
                                                        style: if is_sel {
                                                            "background: rgba(45, 212, 191, 0.12); border: 2px solid var(--accent-mint); border-radius: 12px; padding: 10px; cursor: pointer; display: flex; flex-direction: column; gap: 8px; transition: all 0.15s ease;"
                                                        } else {
                                                            "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 12px; padding: 10px; cursor: pointer; display: flex; flex-direction: column; gap: 8px; transition: all 0.15s ease;"
                                                        },
                                                        onclick: {
                                                            let c_click = c.clone();
                                                            move |_| {
                                                                preview_url.set(Some(c_click.preview_url.clone()));
                                                                selected_yt.set(Some(c_click.clone()));
                                                                selected_local_path.set(None);
                                                                status_msg.set(None);
                                                            }
                                                        },

                                                        div { style: "position: relative; width: 100%; aspect-ratio: 1/1; border-radius: 8px; overflow: hidden; background: #000;",
                                                            img {
                                                                src: "{c.preview_url}",
                                                                alt: "{c.title}",
                                                                style: "width: 100%; height: 100%; object-fit: cover;",
                                                            }
                                                            span {
                                                                class: "badge-pill",
                                                                style: "position: absolute; bottom: 6px; left: 6px; font-size: 10px; padding: 2px 6px; background: rgba(0,0,0,0.75); border: none;",
                                                                if c.source == "youtube_music" { "YTM 1024px" } else { "YT 1080p HD" }
                                                            }
                                                        }

                                                        div { style: "overflow: hidden;",
                                                            div { style: "font-size: 12.5px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: #fff;", "{c.title}" }
                                                            div { style: "font-size: 11px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px;", "{c.author}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },

                            ArtworkSourceTab::LocalFolder => rsx! {
                                if local_folder_images.read().is_empty() {
                                    div { style: "padding: 40px 0; text-align: center; color: var(--text-muted);",
                                        span { class: "material-icons md-36", style: "opacity: 0.5; margin-bottom: 8px;", "folder_off" }
                                        p { "No image files (.jpg, .png, .webp) found in the track's directory." }
                                    }
                                } else {
                                    div {
                                        p { style: "font-size: 12.5px; color: var(--text-secondary); margin-bottom: 12px;",
                                            "Found {local_folder_images.read().len()} local images in directory. Click an image to select it as the album cover:"
                                        }
                                        div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: 14px;",
                                            for path in local_folder_images.read().iter() {
                                                {
                                                    let p = path.clone();
                                                    let file_name = p.file_name().and_then(|f| f.to_str()).unwrap_or("image").to_string();
                                                    let is_sel = selected_local_path.read().as_ref().map(|s| s == &p).unwrap_or(false);
                                                    let thumb_url = resolve_thumbnail_data_url(&p).unwrap_or_default();
                                                    rsx! {
                                                        div {
                                                            key: "{p.display()}",
                                                            style: if is_sel {
                                                                "background: rgba(45, 212, 191, 0.12); border: 2px solid var(--accent-mint); border-radius: 12px; padding: 10px; cursor: pointer; display: flex; flex-direction: column; gap: 8px; transition: all 0.15s ease;"
                                                            } else {
                                                                "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 12px; padding: 10px; cursor: pointer; display: flex; flex-direction: column; gap: 8px; transition: all 0.15s ease;"
                                                            },
                                                            onclick: {
                                                                let p_click = p.clone();
                                                                let url_click = thumb_url.clone();
                                                                move |_| {
                                                                    preview_url.set(Some(url_click.clone()));
                                                                    selected_local_path.set(Some(p_click.clone()));
                                                                    selected_yt.set(None);
                                                                    status_msg.set(None);
                                                                }
                                                            },

                                                            div { style: "width: 100%; aspect-ratio: 1/1; border-radius: 8px; overflow: hidden; background: #000;",
                                                                if !thumb_url.is_empty() {
                                                                    img {
                                                                        src: "{thumb_url}",
                                                                        alt: "{file_name}",
                                                                        style: "width: 100%; height: 100%; object-fit: cover;",
                                                                    }
                                                                } else {
                                                                    div { style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;",
                                                                        span { class: "material-icons md-32", "image" }
                                                                    }
                                                                }
                                                            }

                                                            div { style: "font-size: 12px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: #fff;",
                                                                "{file_name}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },

                            ArtworkSourceTab::BrowseFile => rsx! {
                                div { style: "padding: 30px; text-align: center; background: rgba(255,255,255,0.02); border-radius: 14px; border: 1px dashed var(--border-glass); display: flex; flex-direction: column; align-items: center; gap: 14px;",
                                    span { class: "material-icons md-48", style: "color: var(--accent-mint);", "file_upload" }
                                    h3 { style: "font-size: 16px; font-weight: 600;", "Choose Custom Image From Computer" }
                                    p { style: "font-size: 13px; color: var(--text-muted); max-width: 420px;",
                                        "Supports PNG, JPG, JPEG, and WebP format. High-resolution 1000x1000 or higher images are recommended for crisp rendering."
                                    }
                                    button {
                                        class: "btn-pill active",
                                        style: "padding: 10px 24px; font-weight: 600; font-size: 13.5px;",
                                        onclick: move |_| {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                                                .pick_file()
                                            {
                                                selected_local_path.set(Some(path.clone()));
                                                selected_yt.set(None);
                                                if let Some(url) = resolve_thumbnail_data_url(&path) {
                                                    preview_url.set(Some(url));
                                                }
                                            }
                                        },
                                        span { class: "material-icons md-18", "folder_open" }
                                        "Browse Image File..."
                                    }
                                    if let Some(ref p) = *selected_local_path.read() {
                                        div { style: "margin-top: 10px; font-size: 12px; color: var(--accent-mint); font-family: monospace;",
                                            "Selected: {p.display()}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Right Column: Live Artwork Preview & Confirmation Actions
                    div {
                        style: "padding: 20px 24px; border-left: 1px solid var(--border-glass); background: rgba(0,0,0,0.18); display: flex; flex-direction: column; justify-content: space-between; align-items: center; gap: 16px;",

                        div { style: "width: 100%; display: flex; flex-direction: column; align-items: center; gap: 12px;",
                            div { style: "font-size: 12px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px;",
                                "Artwork Preview"
                            }

                            div { style: "width: 210px; height: 210px; border-radius: 14px; overflow: hidden; background: #000; box-shadow: 0 12px 30px rgba(0,0,0,0.6); border: 1px solid var(--border-glass); display: flex; align-items: center; justify-content: center;",
                                if let Some(ref art) = *preview_url.read() {
                                    img {
                                        src: "{art}",
                                        alt: "Artwork Preview",
                                        style: "width: 100%; height: 100%; object-fit: cover;",
                                    }
                                } else {
                                    div { style: "color: var(--text-muted); text-align: center; padding: 20px;",
                                        span { class: "material-icons md-48", style: "opacity: 0.4;", "album" }
                                        p { style: "font-size: 12px; margin-top: 8px;", "No artwork selected" }
                                    }
                                }
                            }

                            if let Some(ref yt) = *selected_yt.read() {
                                div { style: "width: 100%; background: rgba(255,255,255,0.03); border-radius: 8px; padding: 8px 10px; border: 1px solid var(--border-glass);",
                                    div { style: "font-size: 11px; color: var(--accent-mint); font-weight: 600;", "Source: {yt.source}" }
                                    div { style: "font-size: 11.5px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px;", "{yt.title}" }
                                    div { style: "font-size: 10.5px; color: var(--text-muted);", "{yt.width} x {yt.height} HD" }
                                }
                            } else if let Some(ref p) = *selected_local_path.read() {
                                div { style: "width: 100%; background: rgba(255,255,255,0.03); border-radius: 8px; padding: 8px 10px; border: 1px solid var(--border-glass);",
                                    div { style: "font-size: 11px; color: var(--accent-mint); font-weight: 600;", "Source: Local File" }
                                    div { style: "font-size: 11.5px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px;",
                                        "{p.file_name().and_then(|f| f.to_str()).unwrap_or(\"\")}"
                                    }
                                }
                            }
                        }

                        div { style: "width: 100%; display: flex; flex-direction: column; gap: 10px;",
                            if let Some((ok, msg)) = status_msg.read().as_ref() {
                                div {
                                    style: if *ok { "font-size: 11.5px; color: var(--accent-mint); text-align: center; font-weight: 600;" } else { "font-size: 11.5px; color: #fb7185; text-align: center; font-weight: 600;" },
                                    "{msg}"
                                }
                            }

                            button {
                                class: "btn-pill active",
                                style: "width: 100%; padding: 12px 18px; font-size: 13.5px; font-weight: 700; display: inline-flex; align-items: center; justify-content: center; gap: 8px;",
                                disabled: *is_applying.read() || (selected_yt.read().is_none() && selected_local_path.read().is_none()),
                                onclick: {
                                    let sel_yt_opt = selected_yt.read().clone();
                                    let sel_local_opt = selected_local_path.read().clone();
                                    let path_context = track_path.clone();
                                    let title_context = title.clone();
                                    let artist_context = artist.clone();

                                    move |_| {
                                        is_applying.set(true);
                                        status_msg.set(Some((true, "Applying album artwork...".to_string())));

                                        let sel_yt = sel_yt_opt.clone();
                                        let sel_local = sel_local_opt.clone();
                                        let track_p = path_context.clone();
                                        let t_name = title_context.clone();
                                        let a_name = artist_context.clone();

                                        spawn(async move {
                                            if let Some(yt) = sel_yt {
                                                // Determine target destination path
                                                let dest_path = if let Some(ref p) = track_p {
                                                    if let Some(parent) = p.parent() {
                                                        parent.join("cover.jpg")
                                                    } else {
                                                        dirs::cache_dir()
                                                            .unwrap_or_else(|| PathBuf::from("/tmp"))
                                                            .join("attune-desktop")
                                                            .join("artwork")
                                                            .join(format!("{}_{}.jpg", a_name, t_name))
                                                    }
                                                } else {
                                                    dirs::cache_dir()
                                                        .unwrap_or_else(|| PathBuf::from("/tmp"))
                                                        .join("attune-desktop")
                                                        .join("artwork")
                                                        .join(format!("{}_{}.jpg", a_name, t_name))
                                                };

                                                match download_youtube_thumbnail(&yt.thumbnail_url, &dest_path) {
                                                    Ok(()) => {
                                                        crate::library::clear_thumbnail_cache();
                                                        let data_url = resolve_thumbnail_data_url(&dest_path).unwrap_or_default();
                                                        is_applying.set(false);
                                                        on_artwork_applied.call((dest_path, data_url));
                                                    }
                                                    Err(e) => {
                                                        status_msg.set(Some((false, format!("Failed downloading thumbnail: {e}"))));
                                                        is_applying.set(false);
                                                    }
                                                }
                                            } else if let Some(local_p) = sel_local {
                                                // If track has a directory, copy chosen image to cover.<ext>
                                                let target_dest = if let Some(ref p) = track_p {
                                                    if let Some(parent) = p.parent() {
                                                        let ext = local_p.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
                                                        let d = parent.join(format!("cover.{}", ext));
                                                        if local_p != d {
                                                            let _ = std::fs::copy(&local_p, &d);
                                                        }
                                                        d
                                                    } else {
                                                        local_p.clone()
                                                    }
                                                } else {
                                                    local_p.clone()
                                                };

                                                crate::library::clear_thumbnail_cache();
                                                let data_url = resolve_thumbnail_data_url(&target_dest).unwrap_or_default();
                                                is_applying.set(false);
                                                on_artwork_applied.call((target_dest, data_url));
                                            } else {
                                                is_applying.set(false);
                                            }
                                        });
                                    }
                                },
                                if *is_applying.read() {
                                    span { class: "material-icons md-18", "sync" }
                                    "Saving..."
                                } else {
                                    span { class: "material-icons md-18", "check" }
                                    "Apply Album Art"
                                }
                            }

                            button {
                                class: "btn-pill",
                                style: "width: 100%; padding: 8px 14px; font-size: 12.5px; justify-content: center;",
                                onclick: move |_| on_close.call(()),
                                "Cancel"
                            }
                        }
                    }
                }
            }
        }
    }
}
