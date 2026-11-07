use dioxus::prelude::*;
use crate::stages::albums::AlbumGroup;
use crate::dynamic_components::music_metadata_changer::{
    read_track_metadata, update_track_tag, write_track_metadata, TrackMetadata,
};
use std::path::PathBuf;

#[component]
pub fn AlbumEditor(
    group: AlbumGroup,
    on_back: EventHandler<()>,
    on_saved: EventHandler<()>,
) -> Element {
    let mut album_name = use_signal(|| group.name.clone());
    let mut primary_artist = use_signal(|| group.artist.clone());
    let mut additional_artists_str = use_signal(String::new);
    let mut year_str = use_signal(String::new);
    let mut genre_str = use_signal(|| {
        group.tracks.first()
            .and_then(|t| t.genre.clone())
            .unwrap_or_default()
    });
    let mut custom_splash_path = use_signal(|| None::<PathBuf>);
    let mut custom_splash_data_url = use_signal(|| group.artwork_data_url.clone());

    // Online & Catalog fields
    let mut record_label = use_signal(String::new);
    let mut catalog_number = use_signal(String::new);
    let mut album_type = use_signal(|| "Album".to_string());
    let mut musicbrainz_id = use_signal(String::new);

    let mut status_msg = use_signal(|| None::<(bool, String)>);
    let mut is_saving = use_signal(|| false);

    // Initial load: read existing metadata from the first track via libattune_music_metadata_changer
    use_hook({
        let first_track_path = group.tracks.first().map(|t| t.path.clone());
        let artist_val = group.artist.clone();
        move || {
            if artist_val.contains(',') {
                let parts: Vec<&str> = artist_val.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                if !parts.is_empty() {
                    primary_artist.set(parts[0].to_string());
                    let extras = parts[1..].join(", ");
                    additional_artists_str.set(extras);
                }
            }

            if let Some(path) = first_track_path {
                match read_track_metadata(&path) {
                    Ok(meta) => {
                        if let Some(a) = meta.album.filter(|s| !s.trim().is_empty()) {
                            album_name.set(a);
                        }
                        if let Some(art) = meta.artist.filter(|s| !s.trim().is_empty()) {
                            primary_artist.set(art);
                        }
                        if !meta.additional_artists.is_empty() {
                            additional_artists_str.set(meta.additional_artists.join(", "));
                        }
                        if let Some(y) = meta.year {
                            year_str.set(y.to_string());
                        }
                        if let Some(g) = meta.genre.filter(|s| !s.trim().is_empty()) {
                            genre_str.set(g);
                        }
                        if let Some(lbl) = meta.record_label.filter(|s| !s.trim().is_empty()) {
                            record_label.set(lbl);
                        }
                        if let Some(cat) = meta.catalog_number.filter(|s| !s.trim().is_empty()) {
                            catalog_number.set(cat);
                        }
                        if let Some(at) = meta.album_type.filter(|s| !s.trim().is_empty()) {
                            album_type.set(at);
                        }
                        if let Some(mb) = meta.musicbrainz_id.filter(|s| !s.trim().is_empty()) {
                            musicbrainz_id.set(mb);
                        }
                    }
                    Err(err) => {
                        log::warn!("AlbumEditor: Failed to read initial track metadata for {}: {}", path.display(), err);
                        status_msg.set(Some((false, format!("Metadata read diagnostic: {err}"))));
                    }
                }
            }
        }
    });

    rsx! {
        div { class: "album-editor-stage",
            style: "display: flex; flex-direction: column; gap: 24px; max-width: 860px; margin: 0 auto; width: 100%; padding-bottom: 50px;",

            // Navigation Header
            div { style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-glass); padding-bottom: 16px;",
                div {
                    button {
                        class: "btn-pill",
                        style: "margin-bottom: 10px;",
                        onclick: move |_| on_back.call(()),
                        "← Cancel & Back"
                    }
                    h1 { style: "font-size: 26px; font-weight: 700;", "Edit Album Information" }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                        "Updates metadata tags across all {group.tracks.len()} tracks in this album via libattune_music_metadata_changer"
                    }
                }

                div { style: "display: flex; gap: 10px; align-items: center;",
                    if let Some((ok, msg)) = status_msg.read().as_ref() {
                        span {
                            style: if *ok { "font-size: 12.5px; color: var(--accent-mint); font-weight: 600;" } else { "font-size: 12.5px; color: #fb7185; font-weight: 600;" },
                            "{msg}"
                        }
                    }
                    button {
                        class: "btn-pill active",
                        style: "padding: 10px 24px; font-weight: 600;",
                        disabled: *is_saving.read(),
                        onclick: {
                            let tracks = group.tracks.clone();
                            let target_album = album_name.read().clone();
                            let target_artist = primary_artist.read().clone();
                            let extras = additional_artists_str.read().clone();
                            let target_year = year_str.read().trim().parse::<u32>().ok();
                            let target_genre = genre_str.read().trim().to_string();
                            let target_label = record_label.read().trim().to_string();
                            let target_catalog = catalog_number.read().trim().to_string();
                            let target_type = album_type.read().trim().to_string();
                            let target_mb = musicbrainz_id.read().trim().to_string();
                            let new_splash = custom_splash_path.read().clone();

                            move |_| {
                                is_saving.set(true);
                                status_msg.set(Some((true, "Saving changes to audio tags...".to_string())));

                                let parsed_extras: Vec<String> = extras
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();

                                // If new splash art was picked, copy it to the album directory as cover.<ext>
                                if let Some(splash_src) = new_splash.as_ref() {
                                    if let Some(first_track) = tracks.first() {
                                        if let Some(parent) = first_track.path.parent() {
                                            let ext = splash_src.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
                                            let dest = parent.join(format!("cover.{}", ext));
                                            if let Err(e) = std::fs::copy(splash_src, &dest) {
                                                log::error!("AlbumEditor: Failed to copy cover art to {}: {}", dest.display(), e);
                                            }
                                        }
                                    }
                                }

                                let mut saved_count = 0usize;
                                let mut errors = Vec::new();
                                for track in &tracks {
                                    let mut meta = read_track_metadata(&track.path).unwrap_or_else(|_| {
                                        let mut m = TrackMetadata::new();
                                        m.title = Some(track.title.clone());
                                        m
                                    });
                                    meta.album = Some(target_album.clone());
                                    meta.artist = Some(target_artist.clone());
                                    meta.additional_artists = parsed_extras.clone();
                                    meta.year = target_year;
                                    if !target_genre.is_empty() {
                                        meta.genre = Some(target_genre.clone());
                                    }
                                    if !target_label.is_empty() {
                                        meta.record_label = Some(target_label.clone());
                                    }
                                    if !target_catalog.is_empty() {
                                        meta.catalog_number = Some(target_catalog.clone());
                                    }
                                    if !target_type.is_empty() {
                                        meta.album_type = Some(target_type.clone());
                                    }
                                    if !target_mb.is_empty() {
                                        meta.musicbrainz_id = Some(target_mb.clone());
                                    }

                                    match write_track_metadata(&track.path, &meta) {
                                        Ok(()) => {
                                            if !target_genre.is_empty() {
                                                let _ = update_track_tag(&track.path, "GENRE", &target_genre);
                                            }
                                            saved_count += 1;
                                        }
                                        Err(err) => {
                                            log::error!("AlbumEditor: Failed to write metadata for {}: {}", track.path.display(), err);
                                            errors.push(format!("{}: {}", track.title, err));
                                        }
                                    }
                                }

                                is_saving.set(false);
                                if errors.is_empty() {
                                    status_msg.set(Some((true, format!("✓ Successfully updated {} tracks!", saved_count))));
                                    on_saved.call(());
                                } else {
                                    status_msg.set(Some((
                                        false,
                                        format!("✗ Updated {}/{} tracks ({} failed: {})", saved_count, tracks.len(), errors.len(), errors[0]),
                                    )));
                                }
                            }
                        },
                        if *is_saving.read() {
                            span { class: "material-icons md-16", style: "margin-right: 6px;", "sync" }
                            span { "Saving..." }
                        } else {
                            span { class: "material-icons md-16", style: "margin-right: 6px;", "save" }
                            span { "Save Album Info" }
                        }
                    }
                }
            }

            // Top Layout: Cover Art + Primary Album Fields
            div { style: "display: grid; grid-template-columns: 220px 1fr; gap: 24px; background: var(--bg-card); padding: 24px; border-radius: 18px; border: 1px solid var(--border-glass);",
                // Artwork Picker Card
                div { style: "display: flex; flex-direction: column; align-items: center; gap: 12px;",
                    if let Some(art) = custom_splash_data_url.read().as_ref() {
                        img {
                            src: "{art}",
                            alt: "Album Artwork",
                            style: "width: 190px; height: 190px; border-radius: 14px; object-fit: cover; box-shadow: 0 8px 24px rgba(0,0,0,0.5); border: 1px solid var(--border-glass);",
                        }
                    } else {
                        div {
                            style: "width: 190px; height: 190px; border-radius: 14px; background: rgba(255,255,255,0.05); display: flex; align-items: center; justify-content: center; color: var(--text-muted); border: 1px solid var(--border-glass);",
                            span { class: "material-icons md-48", "album" }
                        }
                    }

                    button {
                        class: "btn-pill",
                        style: "font-size: 12px; width: 100%; display: inline-flex; align-items: center; justify-content: center; gap: 6px;",
                        onclick: move |_| {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                                .pick_file()
                            {
                                custom_splash_path.set(Some(path.clone()));
                                // Convert to base64 Data URL for instant live preview
                                if let Ok(bytes) = std::fs::read(&path) {
                                    use base64::Engine;
                                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                    let mime = if path.extension().and_then(|e| e.to_str()).unwrap_or("").eq_ignore_ascii_case("png") {
                                        "image/png"
                                    } else if path.extension().and_then(|e| e.to_str()).unwrap_or("").eq_ignore_ascii_case("webp") {
                                        "image/webp"
                                    } else {
                                        "image/jpeg"
                                    };
                                    custom_splash_data_url.set(Some(format!("data:{};base64,{}", mime, b64)));
                                }
                            }
                        },
                        span { class: "material-icons md-16", "image" }
                        "Pick Splash Art..."
                    }
                }

                // Core Album Info Form
                div { style: "display: flex; flex-direction: column; gap: 14px;",
                    div {
                        label { style: "display: block; font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 6px;", "Album Title" }
                        input {
                            class: "search-bar",
                            style: "width: 100%; font-size: 14px; padding: 10px 14px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                            value: "{album_name}",
                            oninput: move |e| album_name.set(e.value()),
                        }
                    }

                    div {
                        label { style: "display: block; font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 6px;", "Primary Album Artist" }
                        input {
                            class: "search-bar",
                            style: "width: 100%; font-size: 14px; padding: 10px 14px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                            value: "{primary_artist}",
                            oninput: move |e| primary_artist.set(e.value()),
                        }
                    }

                    // Contributing / Additional Artists with comma-split
                    div {
                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;",
                            label { style: "font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;", "Contributing / Additional Artists" }
                            button {
                                class: "btn-pill",
                                style: "font-size: 11px; padding: 2px 8px; display: inline-flex; align-items: center; gap: 4px;",
                                onclick: move |_| {
                                    let curr = primary_artist.read().clone();
                                    if curr.contains(',') {
                                        let parts: Vec<&str> = curr.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                                        if !parts.is_empty() {
                                            primary_artist.set(parts[0].to_string());
                                            let extras = parts[1..].join(", ");
                                            additional_artists_str.set(extras);
                                        }
                                    }
                                },
                                span { class: "material-icons md-14", "call_split" }
                                "Split Artists (,)"
                            }
                        }
                        input {
                            class: "search-bar",
                            style: "width: 100%; font-size: 14px; padding: 10px 14px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                            placeholder: "e.g. Hatsune Miku, DECO*27, Sirup",
                            value: "{additional_artists_str}",
                            oninput: move |e| additional_artists_str.set(e.value()),
                        }
                    }

                    // Year & Genre
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 14px;",
                        div {
                            label { style: "display: block; font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 6px;", "Release Year" }
                            input {
                                class: "search-bar",
                                style: "width: 100%; font-size: 14px; padding: 10px 14px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                                placeholder: "e.g. 2024",
                                value: "{year_str}",
                                oninput: move |e| year_str.set(e.value()),
                            }
                        }
                        div {
                            label { style: "display: block; font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 6px;", "Genre" }
                            input {
                                class: "search-bar",
                                style: "width: 100%; font-size: 14px; padding: 10px 14px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                                placeholder: "e.g. J-Rock / Vocaloid",
                                value: "{genre_str}",
                                oninput: move |e| genre_str.set(e.value()),
                            }
                        }
                    }
                }
            }

            // Catalog & Online Identifiers Card
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 14px;",
                h3 { style: "font-size: 16px; font-weight: 600;", "Catalog & Online Identifiers" }
                p { style: "font-size: 12px; color: var(--text-muted);", "Prepares metadata tags for MusicBrainz and external catalog indexing." }

                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 14px;",
                    div {
                        label { style: "display: block; font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 6px;", "Record Label" }
                        input {
                            class: "search-bar",
                            style: "width: 100%; font-size: 13.5px; padding: 9px 12px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                            placeholder: "e.g. Victor Entertainment, Sony Music",
                            value: "{record_label}",
                            oninput: move |e| record_label.set(e.value()),
                        }
                    }
                    div {
                        label { style: "display: block; font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 6px;", "Catalog Number" }
                        input {
                            class: "search-bar",
                            style: "width: 100%; font-size: 13.5px; padding: 9px 12px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                            placeholder: "e.g. VICL-65800",
                            value: "{catalog_number}",
                            oninput: move |e| catalog_number.set(e.value()),
                        }
                    }
                }

                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 14px;",
                    div {
                        label { style: "display: block; font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 6px;", "Album Type" }
                        select {
                            class: "search-bar",
                            style: "width: 100%; font-size: 13.5px; padding: 9px 12px; border-radius: 8px; background: #161b26; border: 1px solid var(--border-glass); color: #fff; cursor: pointer; outline: none;",
                            value: "{album_type}",
                            onchange: move |e| album_type.set(e.value()),
                            option { value: "Album", selected: *album_type.read() == "Album", "Album (Studio Album)" }
                            option { value: "Single", selected: *album_type.read() == "Single", "Single" }
                            option { value: "EP", selected: *album_type.read() == "EP", "EP (Extended Play)" }
                            option { value: "Compilation", selected: *album_type.read() == "Compilation", "Compilation" }
                            option { value: "Soundtrack", selected: *album_type.read() == "Soundtrack", "Soundtrack / OST" }
                            option { value: "Remix", selected: *album_type.read() == "Remix", "Remix Album" }
                            option { value: "Live", selected: *album_type.read() == "Live", "Live Album" }
                            option { value: "Demo", selected: *album_type.read() == "Demo", "Demo / Mixtape" }
                        }
                    }
                    div {
                        label { style: "display: block; font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; margin-bottom: 6px;", "MusicBrainz Release ID" }
                        input {
                            class: "search-bar",
                            style: "width: 100%; font-size: 13.5px; padding: 9px 12px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                            placeholder: "UUID string",
                            value: "{musicbrainz_id}",
                            oninput: move |e| musicbrainz_id.set(e.value()),
                        }
                    }
                }
            }

            // Track List Preview
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass);",
                h3 { style: "font-size: 16px; font-weight: 600; margin-bottom: 12px;", "Tracks in this Album ({group.tracks.len()})" }
                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    for (i, t) in group.tracks.iter().enumerate() {
                        div {
                            key: "{t.path.display()}",
                            style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: rgba(255,255,255,0.03); border-radius: 8px;",
                            div { style: "display: flex; gap: 12px; align-items: center;",
                                span { style: "font-family: monospace; color: var(--text-muted); font-size: 12px;", "{i + 1}" }
                                span { style: "font-weight: 500; font-size: 13.5px;", "{t.title}" }
                            }
                            span { style: "font-size: 12px; color: var(--text-muted); font-family: monospace;", "{t.format}" }
                        }
                    }
                }
            }
        }
    }
}
