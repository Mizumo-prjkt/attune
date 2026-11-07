use dioxus::prelude::*;
use std::path::PathBuf;
use crate::library::{clear_thumbnail_cache, TrackEntry};
use crate::process_manager::{sample_metrics, trim_memory};
use crate::smart_shuffle::ShuffleMode;

#[derive(Debug, Clone, PartialEq)]
pub struct DirTreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_root: bool,
    pub track_count: usize,
    pub subdirs: Vec<DirTreeNode>,
}

/// Builds a structured directory tree from music roots and indexed library tracks,
/// sorting directories hierarchically with branch connectors.
pub fn build_directory_tree(roots: &[PathBuf], tracks: &[TrackEntry]) -> Vec<DirTreeNode> {
    let mut tree = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let root_tracks: Vec<&TrackEntry> = tracks.iter().filter(|t| t.path.starts_with(root)).collect();
        let total_root_tracks = root_tracks.len();

        let mut sub_groups: std::collections::BTreeMap<String, (PathBuf, usize)> = std::collections::BTreeMap::new();
        for t in &root_tracks {
            if let Ok(rel) = t.path.strip_prefix(root) {
                if let Some(first_comp) = rel.components().next() {
                    let sub_name = first_comp.as_os_str().to_string_lossy().to_string();
                    let sub_path = root.join(&sub_name);
                    let entry = sub_groups.entry(sub_name).or_insert((sub_path, 0));
                    entry.1 += 1;
                }
            }
        }

        let mut subdirs = Vec::new();
        for (name, (path, count)) in sub_groups {
            subdirs.push(DirTreeNode {
                name,
                path,
                is_root: false,
                track_count: count,
                subdirs: Vec::new(),
            });
        }

        subdirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        let root_name = root.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_else(|| root.to_str().unwrap_or("Root"))
            .to_string();

        tree.push(DirTreeNode {
            name: root_name,
            path: root.clone(),
            is_root: true,
            track_count: total_root_tracks,
            subdirs,
        });
    }

    tree.sort_by(|a, b| a.path.cmp(&b.path));
    tree
}

#[component]
pub fn Settings(
    music_dir: Option<PathBuf>,
    extra_music_dirs: Vec<PathBuf>,
    song_exclusions: Vec<String>,
    tracks: Vec<TrackEntry>,
    music_dir_source: String,
    has_hardware_output: bool,
    driver_error: Option<String>,
    mixer_sample_rate: u32,
    shuffle_mode: ShuffleMode,
    on_change_shuffle_mode: EventHandler<ShuffleMode>,
    on_choose_folder: EventHandler<()>,
    on_add_folder: EventHandler<()>,
    on_remove_folder: EventHandler<PathBuf>,
    on_rescan_library: EventHandler<()>,
    on_add_exclusion: EventHandler<String>,
    on_remove_exclusion: EventHandler<String>,
    on_open_equalizer: EventHandler<()>,
    on_open_process_manager: EventHandler<()>,
    on_open_setup: EventHandler<()>,
) -> Element {
    let mut username = use_signal(|| {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "Listener".to_string())
    });
    let mut camelot_lock = use_signal(|| true);
    let mut trim_feedback = use_signal(|| None::<(bool, String)>);
    let mut metrics_res = use_signal(sample_metrics);
    let mut view_as_tree = use_signal(|| true);
    let mut new_exclusion_input = use_signal(String::new);

    let curr_metrics = metrics_res.read().clone();

    let mut all_configured_roots = Vec::new();
    if let Some(ref d) = music_dir {
        if d.is_dir() && !all_configured_roots.contains(d) {
            all_configured_roots.push(d.clone());
        }
    }
    for d in &extra_music_dirs {
        if d.is_dir() && !all_configured_roots.contains(d) {
            all_configured_roots.push(d.clone());
        }
    }

    let tree_nodes = build_directory_tree(&all_configured_roots, &tracks);

    let output_driver_label = if has_hardware_output {
        if cfg!(target_os = "windows") {
            "WASAPI Hardware Stream (cpal)".to_string()
        } else {
            "ALSA / PipeWire Hardware Stream (cpal)".to_string()
        }
    } else if let Some(ref err) = driver_error {
        format!("Headless Fallback ({err})")
    } else {
        "Headless (No Hardware Audio Output)".to_string()
    };

    let sample_rate_label = if mixer_sample_rate > 0 {
        format!("{mixer_sample_rate} Hz • 32-bit Float")
    } else {
        "Unprobed".to_string()
    };

    rsx! {
        div { class: "settings-stage",
            style: "display: flex; flex-direction: column; gap: 24px; max-width: 860px; margin: 0 auto; width: 100%; padding-bottom: 50px;",

            // Header
            div { style: "border-bottom: 1px solid var(--border-glass); padding-bottom: 16px;",
                h1 { style: "font-size: 26px; font-weight: 700;", "Settings & Preferences" }
                p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                    "Attune Desktop Configuration, Audio Output, Library Management & System Diagnostics"
                }
            }

            // Section 1: General & Profile
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                h3 { style: "font-size: 16px; font-weight: 600;", "General & Listener Profile" }

                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    label { style: "font-size: 11.5px; font-weight: 700; color: var(--text-muted); text-transform: uppercase;", "Listener Display Name" }
                    input {
                        class: "search-bar",
                        style: "width: 100%; max-width: 400px; font-size: 14px; padding: 10px 14px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                        value: "{username}",
                        oninput: move |e| username.set(e.value()),
                    }
                    span { style: "font-size: 11.5px; color: var(--text-muted);", "Resolved from OS environment ($USER / %USERNAME%) for personalized greetings." }
                }

                div { style: "display: flex; justify-content: space-between; align-items: center; padding-top: 10px; border-top: 1px solid var(--border-glass);",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Interface Aesthetic" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "Hardware-accelerated CSS3 liquid glass with backdrop blur" }
                    }
                    span { class: "btn-pill active", style: "font-size: 12px;", "Liquid Material Glass (Active)" }
                }
            }

            // Section 2: Library & Music Folders
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    div {
                        h3 { style: "font-size: 16px; font-weight: 600;", "Music Folders & Directory Tree" }
                        p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;", "Add multiple music libraries, inspect their directory tree structure, and rescan audio." }
                    }
                    div { style: "display: flex; gap: 6px;",
                        button {
                            class: if *view_as_tree.read() { "btn-pill active" } else { "btn-pill" },
                            style: "font-size: 11px; padding: 3px 10px; display: inline-flex; align-items: center; gap: 4px;",
                            onclick: move |_| view_as_tree.set(true),
                            span { class: "material-icons md-14", "account_tree" }
                            "Directory Tree"
                        }
                        button {
                            class: if !*view_as_tree.read() { "btn-pill active" } else { "btn-pill" },
                            style: "font-size: 11px; padding: 3px 10px; display: inline-flex; align-items: center; gap: 4px;",
                            onclick: move |_| view_as_tree.set(false),
                            span { class: "material-icons md-14", "view_list" }
                            "Flat List"
                        }
                    }
                }

                // Action buttons: Add folder, choose primary, rescan
                div { style: "display: flex; gap: 10px; flex-wrap: wrap;",
                    button {
                        class: "btn-pill active",
                        style: "display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| on_add_folder.call(()),
                        span { class: "material-icons md-16", "create_new_folder" }
                        "Add Music Folder..."
                    }
                    button {
                        class: "btn-pill",
                        style: "display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| on_choose_folder.call(()),
                        span { class: "material-icons md-16", "folder_open" }
                        "Set Primary Folder"
                    }
                    button {
                        class: "btn-pill",
                        style: "display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| on_rescan_library.call(()),
                        span { class: "material-icons md-16", "sync" }
                        "Rescan All Folders"
                    }
                    button {
                        class: "btn-pill",
                        style: "display: inline-flex; align-items: center; gap: 6px; border: 1px solid rgba(45,212,191,0.4); background: rgba(45,212,191,0.08); color: var(--accent-mint); font-weight: 600;",
                        onclick: move |_| on_open_setup.call(()),
                        span { class: "material-icons md-16", "auto_awesome" }
                        "Re-run Setup Wizard"
                    }
                }

                // Directory tree / folders list
                div { style: "display: flex; flex-direction: column; gap: 10px; margin-top: 6px;",
                    if tree_nodes.is_empty() {
                        div { style: "padding: 16px; text-align: center; color: var(--text-muted); font-size: 13px; background: rgba(255,255,255,0.02); border-radius: 8px; border: 1px dashed var(--border-glass);",
                            "No music folders configured. Click 'Add Music Folder...' to scan your library."
                        }
                    } else {
                        for root_node in tree_nodes.iter() {
                            {
                                let root_p = root_node.path.clone();
                                let is_primary = music_dir.as_ref() == Some(&root_p);
                                let on_rem = on_remove_folder.clone();
                                let on_add_exc = on_add_exclusion.clone();

                                rsx! {
                                    div {
                                        key: "{root_p.display()}",
                                        style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 12px; padding: 12px 14px; display: flex; flex-direction: column; gap: 8px;",

                                        // Root header
                                        div { style: "display: flex; justify-content: space-between; align-items: center;",
                                            div { style: "display: flex; align-items: center; gap: 8px; overflow: hidden;",
                                                span { class: "material-icons md-18", style: "color: var(--accent-mint);", "folder_special" }
                                                span { style: "font-family: monospace; font-size: 13px; font-weight: 600; color: #fff; word-break: break-all;", "{root_p.display()}" }
                                                if is_primary {
                                                    span { style: "font-size: 10px; font-weight: 700; background: rgba(45,212,191,0.2); color: var(--accent-mint); padding: 2px 6px; border-radius: 4px; text-transform: uppercase;", "PRIMARY" }
                                                }
                                            }
                                            div { style: "display: flex; align-items: center; gap: 8px;",
                                                span { style: "font-size: 11.5px; color: var(--text-muted); font-family: monospace;", "{root_node.track_count} songs" }
                                                if all_configured_roots.len() > 1 {
                                                    button {
                                                        class: "btn-icon",
                                                        style: "width: 24px; height: 24px; color: var(--text-muted);",
                                                        title: "Remove this folder from library",
                                                        onclick: {
                                                            let p = root_p.clone();
                                                            move |_| on_rem.call(p.clone())
                                                        },
                                                        span { class: "material-icons md-16", "remove_circle_outline" }
                                                    }
                                                }
                                            }
                                        }

                                        // Directory Tree branch nodes
                                        if *view_as_tree.read() && !root_node.subdirs.is_empty() {
                                            div { style: "margin-left: 12px; padding-left: 10px; border-left: 1px solid rgba(255,255,255,0.12); display: flex; flex-direction: column; gap: 4px; margin-top: 4px;",
                                                for (s_idx, subdir) in root_node.subdirs.iter().enumerate() {
                                                    {
                                                        let is_last = s_idx == root_node.subdirs.len() - 1;
                                                        let branch_sym = if is_last { "└── " } else { "├── " };
                                                        let sub_p = subdir.path.clone();
                                                        let sub_name = subdir.name.clone();
                                                        let on_exc = on_add_exc.clone();

                                                        rsx! {
                                                            div {
                                                                key: "{sub_p.display()}",
                                                                style: "display: flex; justify-content: space-between; align-items: center; padding: 4px 6px; font-size: 12.5px; border-radius: 6px;",
                                                                div { style: "display: flex; align-items: center; gap: 6px; overflow: hidden;",
                                                                    span { style: "font-family: monospace; color: var(--accent-mint); font-weight: bold;", "{branch_sym}" }
                                                                    span { class: "material-icons md-14", style: "color: var(--text-muted);", "folder" }
                                                                    span { style: "color: var(--text-primary); font-family: monospace; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{sub_name}" }
                                                                }
                                                                div { style: "display: flex; align-items: center; gap: 8px;",
                                                                    span { style: "font-size: 11px; color: var(--text-muted);", "{subdir.track_count} songs" }
                                                                    button {
                                                                        class: "btn-icon",
                                                                        style: "width: 22px; height: 22px; color: var(--text-muted);",
                                                                        title: "Exclude this subdirectory from library",
                                                                        onclick: {
                                                                            let exc_str = sub_p.to_string_lossy().to_string();
                                                                            move |_| on_exc.call(exc_str.clone())
                                                                        },
                                                                        span { class: "material-icons md-14", "block" }
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
                    }
                }

                div { style: "display: flex; justify-content: space-between; align-items: center; padding-top: 12px; border-top: 1px solid var(--border-glass);",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Artwork Thumbnail Cache" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "Stores downsampled 160x160 JPEG thumbnails to keep memory footprint under 150 MB" }
                    }
                    button {
                        class: "btn-pill",
                        style: "font-size: 12px; display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| {
                            clear_thumbnail_cache();
                            trim_feedback.set(Some((true, "✓ Thumbnail cache cleared!".to_string())));
                        },
                        span { class: "material-icons md-16", "delete_sweep" }
                        "Clear Thumbnails"
                    }
                }
            }

            // Section 3: Song Exclusion Index
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    div {
                        h3 { style: "font-size: 16px; font-weight: 600;", "Song Exclusion Index" }
                        p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;",
                            "Exclude specific songs, filenames, or folders from library indexing, shuffle queues, and transitions."
                        }
                    }
                    span { class: "btn-pill", style: "font-size: 11px;", "{song_exclusions.len()} Excluded" }
                }

                // Add exclusion form
                div { style: "display: flex; gap: 8px; align-items: center;",
                    input {
                        class: "search-bar",
                        style: "flex: 1; font-size: 13.5px; padding: 8px 12px; border-radius: 8px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-glass); color: #fff;",
                        placeholder: "Enter song filename, title, or path to exclude (e.g. sample.wav, /interludes/, noise)...",
                        value: "{new_exclusion_input}",
                        oninput: move |e| new_exclusion_input.set(e.value()),
                        onkeydown: {
                            let on_add = on_add_exclusion.clone();
                            move |e: KeyboardEvent| {
                                if e.key() == Key::Enter {
                                    let val = new_exclusion_input.read().trim().to_string();
                                    if !val.is_empty() {
                                        on_add.call(val);
                                        new_exclusion_input.set(String::new());
                                    }
                                }
                            }
                        }
                    }
                    button {
                        class: "btn-pill active",
                        style: "display: inline-flex; align-items: center; gap: 6px; white-space: nowrap;",
                        onclick: {
                            let on_add = on_add_exclusion.clone();
                            move |_| {
                                let val = new_exclusion_input.read().trim().to_string();
                                if !val.is_empty() {
                                    on_add.call(val);
                                    new_exclusion_input.set(String::new());
                                }
                            }
                        },
                        span { class: "material-icons md-16", "add_circle_outline" }
                        "Add Exclusion"
                    }
                }

                // List of active exclusions
                div { style: "display: flex; flex-direction: column; gap: 8px; margin-top: 4px;",
                    if song_exclusions.is_empty() {
                        div { style: "padding: 16px; text-align: center; color: var(--text-muted); font-size: 13px; background: rgba(255,255,255,0.02); border-radius: 8px; border: 1px dashed var(--border-glass);",
                            "No exclusions defined. All discovered audio tracks in your folders are indexed and playable."
                        }
                    } else {
                        for exc in song_exclusions.iter() {
                            {
                                let exc_val = exc.clone();
                                let exc_val_click = exc_val.clone();
                                let on_rem = on_remove_exclusion.clone();
                                rsx! {
                                    div {
                                        key: "{exc_val}",
                                        style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: rgba(251, 113, 133, 0.08); border: 1px solid rgba(251, 113, 133, 0.25); border-radius: 8px;",
                                        div { style: "display: flex; align-items: center; gap: 8px; overflow: hidden;",
                                            span { class: "material-icons md-16", style: "color: #fb7185;", "do_not_disturb_on" }
                                            span { style: "font-family: monospace; font-size: 13px; color: #fb7185; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{exc_val}" }
                                        }
                                        button {
                                            class: "btn-icon",
                                            style: "width: 26px; height: 26px; color: var(--text-muted);",
                                            title: "Remove from exclusion index (restore song)",
                                            onclick: move |_| on_rem.call(exc_val_click.clone()),
                                            span { class: "material-icons md-16", "close" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Section 4: Audio Output & DSP
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                h3 { style: "font-size: 16px; font-weight: 600;", "Audio Engine & Studio Master FX" }

                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 14px;",
                    div { style: "background: rgba(255,255,255,0.03); padding: 12px; border-radius: 10px; border: 1px solid var(--border-glass);",
                        span { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; display: block; margin-bottom: 4px;", "Output Driver" }
                        span {
                            style: if has_hardware_output { "font-weight: 600; font-size: 13.5px; color: var(--accent-mint); font-family: monospace;" } else { "font-weight: 600; font-size: 13.5px; color: #fb7185; font-family: monospace;" },
                            "{output_driver_label}"
                        }
                    }
                    div { style: "background: rgba(255,255,255,0.03); padding: 12px; border-radius: 10px; border: 1px solid var(--border-glass);",
                        span { style: "font-size: 11px; font-weight: 700; color: var(--text-muted); text-transform: uppercase; display: block; margin-bottom: 4px;", "Mixer Sample Rate" }
                        span { style: "font-weight: 600; font-size: 13.5px; color: var(--accent-mint); font-family: monospace;", "{sample_rate_label}" }
                    }
                }

                div { style: "display: flex; justify-content: space-between; align-items: center; padding-top: 10px; border-top: 1px solid var(--border-glass);",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Master Equalizer & FX Rack" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "10-band IIR graphic EQ, Pure Bass+, Clarity exciter, Stereo Width, and Limiter" }
                    }
                    button {
                        class: "btn-pill active",
                        style: "display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| on_open_equalizer.call(()),
                        span { class: "material-icons md-16", "tune" }
                        "Open Equalizer Rack"
                    }
                }
            }

            // Section 5: Transitions & Smart Shuffle
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                h3 { style: "font-size: 16px; font-weight: 600;", "Smart DJ Transitions & Mixer Logic" }

                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Dynamic Smart DJ Shuffle" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "Calculates Camelot harmonic key affinity, tempo alignment, and recency penalties" }
                    }
                    button {
                        class: if shuffle_mode == ShuffleMode::Smart { "btn-pill active" } else { "btn-pill" },
                        onclick: move |_| {
                            let next = if shuffle_mode == ShuffleMode::Smart {
                                ShuffleMode::Off
                            } else {
                                ShuffleMode::Smart
                            };
                            on_change_shuffle_mode.call(next);
                        },
                        if shuffle_mode == ShuffleMode::Smart { "ENABLED (SMART DJ)" } else if shuffle_mode == ShuffleMode::Linear { "LINEAR SHUFFLE" } else { "DISABLED (SEQUENTIAL)" }
                    }
                }

                div { style: "display: flex; justify-content: space-between; align-items: center; padding-top: 12px; border-top: 1px solid var(--border-glass);",
                    div {
                        span { style: "font-weight: 500; font-size: 14px; display: block;", "Camelot Circle Harmonic Key Locking" }
                        span { style: "font-size: 12px; color: var(--text-muted);", "Prioritizes adjacent keys (e.g. 8A → 8B, 9A) to prevent dissonance during blending" }
                    }
                    button {
                        class: if *camelot_lock.read() { "btn-pill active" } else { "btn-pill" },
                        onclick: move |_| {
                            let curr = *camelot_lock.read();
                            camelot_lock.set(!curr);
                        },
                        if *camelot_lock.read() { "LOCKED" } else { "UNLOCKED" }
                    }
                }
            }

            // Section 6: Hardware & Process Resource Monitor
            div { style: "background: var(--bg-card); padding: 22px; border-radius: 18px; border: 1px solid var(--border-glass); display: flex; flex-direction: column; gap: 16px;",
                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    div {
                        h3 { style: "font-size: 16px; font-weight: 600;", "Process & Hardware Diagnostics" }
                        p { style: "font-size: 12px; color: var(--text-muted); margin-top: 2px;", "Inspect kernel CPU load, GPU clock, memory RSS, and audio IRQs via libattune_procmgr" }
                    }
                    div { style: "display: flex; gap: 10px; align-items: center;",
                        if let Some((ok, msg)) = trim_feedback.read().as_ref() {
                            span {
                                style: if *ok { "font-size: 11.5px; color: var(--accent-mint); font-weight: 600;" } else { "font-size: 11.5px; color: #fb7185; font-weight: 600;" },
                                "{msg}"
                            }
                        }
                        button {
                            class: "btn-pill",
                            style: "font-size: 12px; display: inline-flex; align-items: center; gap: 6px;",
                            onclick: move |_| {
                                clear_thumbnail_cache();
                                match trim_memory() {
                                    Ok((before, after)) => {
                                        metrics_res.set(sample_metrics());
                                        let freed_mb = before.saturating_sub(after) as f64 / (1024.0 * 1024.0);
                                        trim_feedback.set(Some((true, format!("✓ Freed {:.1} MB", freed_mb))));
                                    }
                                    Err(err) => {
                                        trim_feedback.set(Some((false, format!("✗ Trim failed: {err}"))));
                                    }
                                }
                            },
                            span { class: "material-icons md-16", "cleaning_services" }
                            "Trim Memory"
                        }
                        button {
                            class: "btn-pill active",
                            style: "display: inline-flex; align-items: center; gap: 6px;",
                            onclick: move |_| on_open_process_manager.call(()),
                            span { class: "material-icons md-16", "analytics" }
                            "Open Process Manager"
                        }
                    }
                }

                match curr_metrics {
                    Ok(m) => {
                        let rss_mb = m.rss_bytes as f64 / (1024.0 * 1024.0);
                        rsx! {
                            div { style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; padding-top: 8px;",
                                div { style: "background: rgba(255,255,255,0.03); padding: 10px; border-radius: 8px; text-align: center;",
                                    span { style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; display: block;", "Current RSS" }
                                    span { style: "font-family: monospace; font-size: 14px; font-weight: 600; color: var(--text-primary);", "{rss_mb:.1} MB" }
                                }
                                div { style: "background: rgba(255,255,255,0.03); padding: 10px; border-radius: 8px; text-align: center;",
                                    span { style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; display: block;", "CPU Usage" }
                                    span { style: "font-family: monospace; font-size: 14px; font-weight: 600; color: var(--accent-mint);", "{m.cpu_usage_pct:.1}%" }
                                }
                                div { style: "background: rgba(255,255,255,0.03); padding: 10px; border-radius: 8px; text-align: center;",
                                    span { style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; display: block;", "Threads" }
                                    span { style: "font-family: monospace; font-size: 14px; font-weight: 600; color: var(--accent-purple);", "{m.thread_count}" }
                                }
                                div { style: "background: rgba(255,255,255,0.03); padding: 10px; border-radius: 8px; text-align: center;",
                                    span { style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; display: block;", "Audio IRQs" }
                                    span { style: "font-family: monospace; font-size: 14px; font-weight: 600; color: var(--accent-mint);", "{m.audio_irq_rate_per_sec:.0}/s" }
                                }
                            }
                        }
                    }
                    Err(err_msg) => rsx! {
                        div { style: "background: rgba(251,113,133,0.10); border: 1px solid rgba(251,113,133,0.45); padding: 12px; border-radius: 8px; color: #fb7185; font-family: monospace; font-size: 12px;",
                            "Process Manager Telemetry Error: {err_msg}"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_track_with_path(p: PathBuf, title: &str) -> TrackEntry {
        TrackEntry {
            path: p,
            title: title.to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            duration_ms: 180000,
            bpm: Some(120.0),
            key: Some("8B".to_string()),
            format: "FLAC".to_string(),
            sample_rate: 44100,
            ..Default::default()
        }
    }

    #[test]
    fn test_build_directory_tree_hierarchy() {
        let tmp = std::env::temp_dir().join("attune_test_dir_tree");
        let root = tmp.join("Music");
        let sub1 = root.join("Rock");
        let sub2 = root.join("Electronic");
        let _ = std::fs::create_dir_all(&sub1);
        let _ = std::fs::create_dir_all(&sub2);

        let t1 = make_test_track_with_path(sub1.join("track1.flac"), "Rock Song 1");
        let t2 = make_test_track_with_path(sub1.join("track2.flac"), "Rock Song 2");
        let t3 = make_test_track_with_path(sub2.join("track3.flac"), "Electro Song");

        let tree = build_directory_tree(&[root.clone()], &[t1, t2, t3]);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].track_count, 3);
        assert_eq!(tree[0].subdirs.len(), 2);
        assert_eq!(tree[0].subdirs[0].name, "Electronic");
        assert_eq!(tree[0].subdirs[0].track_count, 1);
        assert_eq!(tree[0].subdirs[1].name, "Rock");
        assert_eq!(tree[0].subdirs[1].track_count, 2);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
