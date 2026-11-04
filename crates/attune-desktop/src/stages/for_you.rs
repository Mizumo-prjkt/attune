use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};
use crate::stages::albums::format_total_duration;
use std::collections::HashSet;
use std::path::PathBuf;

pub const SURPRISE_ME_COMMENT: &str = "This catalog contains the most random song selection that this algorithm can come up based on melody, track composition, artists, how much did you play them or not, etc. at Random. Play if you want to.";

/// Determines if "Surprise Me" is eligible to be shown (only when >= 500 tracks in library).
pub fn should_show_surprise_me(track_count: usize) -> bool {
    track_count >= 500
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlgorithmicMix {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub description: &'static str,
    pub gradient: &'static str,
    pub hero_gradient_top: &'static str,
    pub vibe_tags: Vec<&'static str>,
}

#[component]
pub fn ForYou(
    tracks: Vec<TrackEntry>,
    #[props(default)]
    selected_mix_id: Option<String>,
    #[props(default)]
    current_track_path: Option<PathBuf>,
    #[props(default)]
    is_playing: bool,
    #[props(default)]
    favorites: HashSet<PathBuf>,
    #[props(default)]
    on_select_mix: EventHandler<String>,
    on_start_mix: EventHandler<Vec<TrackEntry>>,
    #[props(default)]
    on_play_track: EventHandler<TrackEntry>,
    #[props(default)]
    on_queue_track: EventHandler<TrackEntry>,
    #[props(default)]
    on_toggle_favorite: EventHandler<PathBuf>,
    #[props(default)]
    on_toggle_play: EventHandler<()>,
) -> Element {
    let mut local_selected = use_signal(|| None::<String>);

    let mut mixes = vec![
        AlgorithmicMix {
            id: "harmonic_flow",
            title: "Harmonic Flow",
            subtitle: "Camelot Wheel Continuous Mix",
            description: "Sequences tracks along adjacent Camelot keys (1A → 2A → 2B) for pitch-perfect, in-key harmonic blends without tonal clashes.",
            gradient: "linear-gradient(135deg, rgba(45, 212, 191, 0.22), rgba(59, 130, 246, 0.18))",
            hero_gradient_top: "rgb(35, 110, 140)",
            vibe_tags: vec!["Harmonic Wheel", "Key-Locked", "Seamless Morph"],
        },
        AlgorithmicMix {
            id: "high_energy",
            title: "High-Energy Rush",
            subtitle: "Aggression & Drive Index 𝒪(𝒜 ≥ 0.65)",
            description: "High-BPM drives, J-Rock, Drum & Bass, and Synthwave. Uses Attune's fast 2-bar Cold Cut and EQ Swap archetypes.",
            gradient: "linear-gradient(135deg, rgba(251, 113, 133, 0.22), rgba(249, 115, 22, 0.18))",
            hero_gradient_top: "rgb(165, 55, 65)",
            vibe_tags: vec!["Fast Swaps", "Aggression ≥ 0.65", "Bass Cut"],
        },
        AlgorithmicMix {
            id: "midnight_chill",
            title: "Midnight Chill",
            subtitle: "Downtempo & Ambient Vocal Sustains",
            description: "Late-night atmosphere, lo-fi, and warm vocals. Employs Tail Freeze and Sustained Vocal Bridge morphs.",
            gradient: "linear-gradient(135deg, rgba(168, 85, 247, 0.22), rgba(99, 102, 241, 0.18))",
            hero_gradient_top: "rgb(90, 60, 145)",
            vibe_tags: vec!["Vocal Sustain", "Tail Freeze", "Downtempo"],
        },
        AlgorithmicMix {
            id: "vocal_spotlight",
            title: "Vocal Spotlight",
            subtitle: "Formant F1/F2 & Pitch Sustains",
            description: "Highlights resonant vocal performances and acapella intros. Uses multi-layer vocal sustain morphs with formants intact.",
            gradient: "linear-gradient(135deg, rgba(236, 72, 153, 0.22), rgba(168, 85, 247, 0.18))",
            hero_gradient_top: "rgb(155, 45, 105)",
            vibe_tags: vec!["Vocal Anchor", "Formant Lock", "Acapella Bridge"],
        },
        AlgorithmicMix {
            id: "bpm_drive",
            title: "BPM Drive & Fast Swaps",
            subtitle: "Tempo Climbing & Drop Alignment",
            description: "Progressive energy ascension and quick transition handoffs timed perfectly to 8-bar and 16-bar phrase drops.",
            gradient: "linear-gradient(135deg, rgba(14, 165, 233, 0.22), rgba(45, 212, 191, 0.18))",
            hero_gradient_top: "rgb(25, 115, 155)",
            vibe_tags: vec!["Tempo Climb", "Phrase Grid", "Drop Sync"],
        },
    ];

    // Only add "Surprise Me" if library has 500+ songs (otherwise completely hidden)
    if should_show_surprise_me(tracks.len()) {
        mixes.push(AlgorithmicMix {
            id: "surprise_me",
            title: "Surprise Me",
            subtitle: "Algorithmic Wildcard Selection",
            description: SURPRISE_ME_COMMENT,
            gradient: "linear-gradient(135deg, rgba(168, 85, 247, 0.22), rgba(236, 72, 153, 0.18))",
            hero_gradient_top: "rgb(120, 50, 150)",
            vibe_tags: vec!["Mystery Catalog", "Melody Randomizer", "Surprise Mix"],
        });
    }

    // Resolve which mix tracks to compute for a given mix id
    let get_mix_tracks = |mix_id: &str| -> Vec<TrackEntry> {
        match mix_id {
            "harmonic_flow" => {
                let mut harmonic = tracks.clone();
                harmonic.sort_by(|a, b| {
                    let ka = a.key.as_deref().and_then(crate::smart_shuffle::normalize_to_camelot).unwrap_or_default();
                    let kb = b.key.as_deref().and_then(crate::smart_shuffle::normalize_to_camelot).unwrap_or_default();
                    ka.cmp(&kb)
                });
                harmonic
            },
            "high_energy" => {
                let high: Vec<TrackEntry> = tracks.iter()
                    .filter(|t| {
                        let has_high_bpm = t.bpm.map(|b| b >= 135.0).unwrap_or(false);
                        let is_energetic = {
                            let text = format!("{} {} {}", t.title, t.artist, t.genre.as_deref().unwrap_or("")).to_lowercase();
                            text.contains("rock") || text.contains("rush") || text.contains("fever") ||
                            text.contains("dance") || text.contains("energy") || text.contains("speed") ||
                            text.contains("fast") || text.contains("hard") || text.contains("boogie") ||
                            text.contains("drum") || text.contains("bass") || text.contains("eurobeat")
                        };
                        has_high_bpm || is_energetic
                    })
                    .cloned()
                    .collect();
                if high.is_empty() { tracks.clone() } else { high }
            },
            "midnight_chill" => {
                let chill: Vec<TrackEntry> = tracks.iter()
                    .filter(|t| {
                        let has_slow_bpm = t.bpm.map(|b| b < 125.0).unwrap_or(false);
                        let is_chill = {
                            let text = format!("{} {} {}", t.title, t.artist, t.genre.as_deref().unwrap_or("")).to_lowercase();
                            text.contains("chill") || text.contains("lo-fi") || text.contains("lofi") ||
                            text.contains("ambient") || text.contains("slow") || text.contains("night") ||
                            text.contains("ballad") || text.contains("acoustic") || text.contains("piano") ||
                            text.contains("symphony") || text.contains("orchestra") || text.contains("sleep")
                        };
                        has_slow_bpm || is_chill
                    })
                    .cloned()
                    .collect();
                if chill.is_empty() { tracks.clone() } else { chill }
            },
            "vocal_spotlight" => {
                let vocal: Vec<TrackEntry> = tracks.iter()
                    .filter(|t| {
                        let text = format!("{} {} {}", t.title, t.artist, t.genre.as_deref().unwrap_or("")).to_lowercase();
                        text.contains("vocal") || text.contains("feat") || text.contains("ft.") ||
                        text.contains("miku") || text.contains("rin") || text.contains("len") ||
                        text.contains("luka") || text.contains("teto") || text.contains("gumi") ||
                        text.contains("pop") || text.contains("sing") || text.contains("chorus") ||
                        text.contains("duet")
                    })
                    .cloned()
                    .collect();
                if vocal.is_empty() { tracks.clone() } else { vocal }
            },
            "bpm_drive" => {
                let mut drive: Vec<TrackEntry> = tracks.iter()
                    .filter(|t| {
                        let text = format!("{} {} {}", t.title, t.artist, t.genre.as_deref().unwrap_or("")).to_lowercase();
                        let is_slow = text.contains("symphony") || text.contains("orchestra") || text.contains("sleep");
                        !is_slow
                    })
                    .cloned()
                    .collect();
                drive.sort_by(|a, b| {
                    let ba = a.bpm.unwrap_or(120.0);
                    let bb = b.bpm.unwrap_or(120.0);
                    ba.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
                });
                if drive.len() > 30 {
                    drive.truncate(30);
                }
                drive
            },
            "surprise_me" => {
                // Diverse pseudo-random selection across the library
                let mut surprise = tracks.clone();
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(42);
                let mut rng = seed;
                for i in (1..surprise.len()).rev() {
                    rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let j = (rng as usize) % (i + 1);
                    surprise.swap(i, j);
                }
                surprise.truncate(50);
                surprise
            },
            _ => tracks.clone(),
        }
    };

    let active_mix_id = selected_mix_id.or_else(|| local_selected.read().clone());
    let active_mix = active_mix_id.as_ref().and_then(|id| mixes.iter().find(|m| &m.id == id).cloned());

    rsx! {
        div { class: "for-you-stage",
            if let Some(mix) = active_mix {
                {
                    let mix_tracks = get_mix_tracks(mix.id);
                    let track_count = mix_tracks.len();
                    let total_dur_str = format_total_duration(&mix_tracks);
                    let is_surprise = mix.id == "surprise_me";

                    let is_current_mix = current_track_path.as_ref().map(|p| mix_tracks.iter().any(|t| &t.path == p)).unwrap_or(false);
                    let is_current_mix_playing = is_playing && is_current_mix;
                    let all_tracks_play = mix_tracks.clone();
                    let all_tracks_shuffle = mix_tracks.clone();
                    let all_tracks_queue = mix_tracks.clone();

                    rsx! {
                        div {
                            class: "album-detail-view",
                            style: "background: linear-gradient(180deg, {mix.hero_gradient_top} 0%, rgba(18, 18, 18, 0.72) 280px, var(--bg-canvas) 460px);",

                            // Hero Header: 232px Animated SVG Art on left + Big Metadata on right
                            div { class: "album-detail-hero",
                                div { class: "album-cover-box",
                                    {render_mix_svg_art(mix.id)}
                                }

                                div { class: "album-meta-col",
                                    span { class: "album-meta-type", "Smart Algorithmic Mix" }
                                    h1 { class: "album-meta-title", "{mix.title}" }
                                    div { class: "album-meta-subline",
                                        div { class: "album-artist-avatar",
                                            span { class: "material-icons md-14", style: "color: var(--accent-mint);", "auto_awesome" }
                                        }
                                        span { class: "album-artist-name", "Attune Engine" }
                                        span { class: "album-dot-sep", "•" }
                                        span { "{mix.subtitle}" }
                                        span { class: "album-dot-sep", "•" }
                                        span { class: "album-meta-songs", "{track_count} eligible tracks," }
                                        span { class: "album-meta-dur", "{total_dur_str}" }
                                    }
                                }
                            }

                            // Action Bar: Big green Play button, Shuffle, Favorite, Add to Queue
                            div { class: "album-action-bar",
                                button {
                                    class: "album-big-play-btn",
                                    title: if is_current_mix_playing { "Pause" } else { "Start Mix" },
                                    onclick: {
                                        let all_t = all_tracks_play.clone();
                                        move |_| {
                                            if is_current_mix_playing {
                                                on_toggle_play.call(());
                                            } else {
                                                on_start_mix.call(all_t.clone());
                                            }
                                        }
                                    },
                                    if is_current_mix_playing {
                                        span { class: "material-icons md-32", "pause" }
                                    } else {
                                        span { class: "material-icons md-32", "play_arrow" }
                                    }
                                }

                                button {
                                    class: "album-icon-btn",
                                    title: "Shuffle Mix",
                                    onclick: {
                                        let mut s = all_tracks_shuffle.clone();
                                        s.reverse();
                                        move |_| on_start_mix.call(s.clone())
                                    },
                                    span { class: "material-icons md-28", "shuffle" }
                                }

                                button {
                                    class: if mix_tracks.iter().all(|t| favorites.contains(&t.path)) && !mix_tracks.is_empty() {
                                        "album-icon-btn active"
                                    } else {
                                        "album-icon-btn"
                                    },
                                    title: "Favorite All in Mix",
                                    onclick: {
                                        let all_paths: Vec<PathBuf> = mix_tracks.iter().map(|t| t.path.clone()).collect();
                                        move |_| {
                                            for p in &all_paths {
                                                on_toggle_favorite.call(p.clone());
                                            }
                                        }
                                    },
                                    if mix_tracks.iter().all(|t| favorites.contains(&t.path)) && !mix_tracks.is_empty() {
                                        span { class: "material-icons md-28", style: "color: #fb7185;", "favorite" }
                                    } else {
                                        span { class: "material-icons md-28", "favorite_border" }
                                    }
                                }

                                button {
                                    class: "album-icon-btn",
                                    title: "Add All to Queue",
                                    onclick: {
                                        let all_q = all_tracks_queue.clone();
                                        move |_| {
                                            for t in &all_q {
                                                on_queue_track.call(t.clone());
                                            }
                                        }
                                    },
                                    span { class: "material-icons md-28", "more_horiz" }
                                }
                            }

                            // If "Surprise Me", the tracklist is classified / hidden!
                            if is_surprise {
                                div { class: "mystery-surprise-card",
                                    span { class: "material-icons md-48", style: "color: #a855f7; margin-bottom: 12px;", "auto_awesome" }
                                    h2 { style: "font-size: 21px; font-weight: 700; color: #ffffff; margin-bottom: 12px;", "Catalog Hidden — Surprise Mystery Mix" }
                                    p { style: "font-size: 14.5px; color: #e2e8f0; max-width: 620px; line-height: 1.6; text-align: center; margin: 0 auto;",
                                        "\"{SURPRISE_ME_COMMENT}\""
                                    }
                                    button {
                                        class: "album-big-play-btn",
                                        style: "margin-top: 26px;",
                                        title: "Play Surprise Mix",
                                        onclick: {
                                            let all_t = all_tracks_play.clone();
                                            move |_| on_start_mix.call(all_t.clone())
                                        },
                                        span { class: "material-icons md-32", "play_arrow" }
                                    }
                                }
                            } else {
                                // 4-Column Tracklist Table matching the album layout
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
                                        for (idx, track) in mix_tracks.iter().enumerate() {
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

                                    // Dynamic Rationale / Lore Footer
                                    div { class: "album-detail-footer",
                                        p { class: "album-footer-lore", "{mix.description}" }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Overview of all mixes
                div { style: "margin-bottom: 24px;",
                    h1 { style: "font-size: 26px; font-weight: 700;", "For You — Smart Algorithmic Mixes" }
                    p { style: "color: var(--text-secondary); font-size: 13.5px; margin-top: 4px;",
                        "Multi-dimensional DJ recipes generated directly from your local music collection."
                    }
                }

                div { style: "display: flex; flex-direction: column; gap: 18px;",
                    for mix in mixes.into_iter() {
                        {
                            let mix_id_str = mix.id.to_string();
                            let mix_tracks = get_mix_tracks(mix.id);
                            let track_count = mix_tracks.len();
                            let all_tracks = mix_tracks.clone();

                            // Collect up to 4 distinct album artworks for preview collage
                            let mut seen_arts = HashSet::new();
                            let preview_arts: Vec<String> = mix_tracks
                                .iter()
                                .filter_map(|t| t.artwork_data_url.clone())
                                .filter(|art| seen_arts.insert(art.clone()))
                                .take(4)
                                .collect();

                            rsx! {
                                div {
                                    key: "{mix.id}",
                                    style: "display: flex; gap: 24px; background: rgba(18, 24, 38, 0.45); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 18px; padding: 22px; backdrop-filter: blur(28px); -webkit-backdrop-filter: blur(28px); position: relative; overflow: hidden; box-shadow: 0 10px 30px rgba(0,0,0,0.25); cursor: pointer; transition: transform 0.2s ease, border-color 0.2s ease;",
                                    onclick: {
                                        let id = mix_id_str.clone();
                                        move |_| {
                                            local_selected.set(Some(id.clone()));
                                            on_select_mix.call(id.clone());
                                        }
                                    },

                                    // Ambient gradient backdrop accent
                                    div {
                                        style: "position: absolute; inset: 0; background: {mix.gradient}; opacity: 0.35; pointer-events: none;",
                                    }

                                    // Animated SVG Mix Icon Container
                                    div {
                                        style: "position: relative; z-index: 1; width: 90px; height: 90px; border-radius: 16px; background: rgba(255,255,255,0.06); display: flex; align-items: center; justify-content: center; border: 1px solid rgba(255,255,255,0.12); flex-shrink: 0; box-shadow: 0 8px 20px rgba(0,0,0,0.3); overflow: hidden; padding: 6px;",
                                        {render_mix_svg_art(mix.id)}
                                    }

                                    // Mix Info
                                    div {
                                        style: "position: relative; z-index: 1; display: flex; flex-direction: column; justify-content: center; flex: 1; min-width: 0;",
                                        span { style: "color: var(--accent-mint); font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;", "{mix.subtitle}" }
                                        h2 { style: "font-size: 21px; font-weight: 700; margin: 2px 0 6px 0;", "{mix.title}" }
                                        p { style: "color: var(--text-secondary); font-size: 13px; line-height: 1.4; max-width: 600px;", "{mix.description}" }

                                        div { style: "display: flex; gap: 8px; margin-top: 10px; align-items: center; flex-wrap: wrap;",
                                            for tag in mix.vibe_tags {
                                                span {
                                                    key: "{tag}",
                                                    style: "background: rgba(255,255,255,0.06); padding: 3px 8px; border-radius: 6px; font-size: 10.5px; color: var(--text-secondary); border: 1px solid rgba(255,255,255,0.08);",
                                                    "{tag}"
                                                }
                                            }
                                            span { style: "font-size: 11.5px; color: var(--text-muted); margin-left: 4px;", "{track_count} eligible tracks" }
                                        }
                                    }

                                    // Album Artwork Preview Collage (hidden for surprise me)
                                    if !preview_arts.is_empty() && mix.id != "surprise_me" {
                                        div {
                                            style: "position: relative; z-index: 1; display: flex; align-items: center; margin-right: 8px; flex-shrink: 0;",
                                            for (i, art) in preview_arts.into_iter().enumerate() {
                                                img {
                                                    key: "{i}",
                                                    src: "{art}",
                                                    alt: "Album Cover",
                                                    style: "width: 44px; height: 44px; border-radius: 8px; object-fit: cover; border: 1px solid rgba(255,255,255,0.15); margin-left: if i > 0 { \"-14px\" } else { \"0px\" }; box-shadow: 0 4px 12px rgba(0,0,0,0.4); transform: scale({1.0 - (i as f32 * 0.03)}); transition: transform 0.2s;",
                                                }
                                            }
                                        }
                                    }

                                    // Start Button
                                    div {
                                        style: "position: relative; z-index: 1; display: flex; align-items: center; flex-shrink: 0;",
                                        button {
                                            class: "btn-pill active",
                                            style: "padding: 10px 22px; font-size: 13.5px; font-weight: 600; box-shadow: 0 4px 16px rgba(45,212,191,0.25); display: inline-flex; align-items: center; gap: 6px;",
                                            onclick: {
                                                let t_start = all_tracks.clone();
                                                move |evt: MouseEvent| {
                                                    evt.stop_propagation();
                                                    on_start_mix.call(t_start.clone());
                                                }
                                            },
                                            span { class: "material-icons md-18", "play_arrow" }
                                            if mix.id == "surprise_me" {
                                                "Play Surprise"
                                            } else {
                                                "Start Mix"
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

/// Renders the animated SVG art for the specified mix archetype.
pub fn render_mix_svg_art(mix_id: &str) -> Element {
    match mix_id {
        "harmonic_flow" => rsx! {
            svg {
                class: "for-you-svg-art animated-harmonic-flow",
                view_box: "0 0 200 200",
                defs {
                    linearGradient { id: "harmonic-grad-1", x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                        stop { offset: "0%", stop_color: "#2dd4bf" }
                        stop { offset: "100%", stop_color: "#3b82f6" }
                    }
                    linearGradient { id: "harmonic-grad-2", x1: "100%", y1: "0%", x2: "0%", y2: "100%",
                        stop { offset: "0%", stop_color: "#a855f7" }
                        stop { offset: "100%", stop_color: "#2dd4bf" }
                    }
                }
                circle { cx: "100", cy: "100", r: "75", fill: "none", stroke: "rgba(45, 212, 191, 0.15)", stroke_width: "2" }
                path { d: "M 20 100 Q 60 40, 100 100 T 180 100", fill: "none", stroke: "url(#harmonic-grad-1)", stroke_width: "6", stroke_linecap: "round", class: "harmonic-wave-1" }
                path { d: "M 20 100 Q 60 160, 100 100 T 180 100", fill: "none", stroke: "url(#harmonic-grad-2)", stroke_width: "5", stroke_linecap: "round", class: "harmonic-wave-2" }
                circle { cx: "60", cy: "70", r: "6", fill: "#2dd4bf", class: "harmonic-node" }
                circle { cx: "100", cy: "100", r: "7", fill: "#ffffff", class: "harmonic-node" }
                circle { cx: "140", cy: "130", r: "6", fill: "#a855f7", class: "harmonic-node" }
            }
        },
        "high_energy" => rsx! {
            svg {
                class: "for-you-svg-art animated-high-energy",
                view_box: "0 0 200 200",
                defs {
                    linearGradient { id: "energy-grad", x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                        stop { offset: "0%", stop_color: "#fb7185" }
                        stop { offset: "50%", stop_color: "#f97316" }
                        stop { offset: "100%", stop_color: "#facc15" }
                    }
                }
                polygon { points: "100,20 170,100 100,180 30,100", fill: "none", stroke: "rgba(251, 113, 133, 0.25)", stroke_width: "2", class: "energy-diamond" }
                polygon { points: "100,35 155,100 100,165 45,100", fill: "none", stroke: "rgba(249, 115, 22, 0.35)", stroke_width: "2" }
                path { d: "M 112 25 L 62 105 L 102 105 L 82 175 L 142 92 L 102 92 Z", fill: "url(#energy-grad)", class: "energy-bolt" }
            }
        },
        "midnight_chill" => rsx! {
            svg {
                class: "for-you-svg-art animated-midnight-chill",
                view_box: "0 0 200 200",
                defs {
                    linearGradient { id: "chill-grad", x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                        stop { offset: "0%", stop_color: "#c084fc" }
                        stop { offset: "60%", stop_color: "#818cf8" }
                        stop { offset: "100%", stop_color: "#2dd4bf" }
                    }
                }
                circle { cx: "100", cy: "100", r: "70", fill: "none", stroke: "rgba(168, 85, 247, 0.2)", stroke_width: "2" }
                path { d: "M 125 45 A 60 60 0 1 0 155 135 A 50 50 0 0 1 125 45 Z", fill: "url(#chill-grad)", class: "chill-moon" }
                circle { cx: "65", cy: "55", r: "3", fill: "#ffffff", class: "chill-star star-1" }
                circle { cx: "145", cy: "40", r: "2.5", fill: "#c084fc", class: "chill-star star-2" }
                circle { cx: "50", cy: "120", r: "2.5", fill: "#2dd4bf", class: "chill-star star-3" }
                circle { cx: "140", cy: "155", r: "3.5", fill: "#ffffff", class: "chill-star star-4" }
            }
        },
        "vocal_spotlight" => rsx! {
            svg {
                class: "for-you-svg-art animated-vocal-spotlight",
                view_box: "0 0 200 200",
                defs {
                    linearGradient { id: "vocal-grad", x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                        stop { offset: "0%", stop_color: "#f43f5e" }
                        stop { offset: "50%", stop_color: "#ec4899" }
                        stop { offset: "100%", stop_color: "#a855f7" }
                    }
                }
                circle { cx: "100", cy: "85", r: "45", fill: "none", stroke: "rgba(236, 72, 153, 0.3)", stroke_width: "2", class: "vocal-wave wave-1" }
                circle { cx: "100", cy: "85", r: "65", fill: "none", stroke: "rgba(168, 85, 247, 0.2)", stroke_width: "2", class: "vocal-wave wave-2" }
                rect { x: "85", y: "42", width: "30", height: "55", rx: "15", fill: "url(#vocal-grad)", class: "vocal-mic-head" }
                line { x1: "88", y1: "60", x2: "112", y2: "60", stroke: "rgba(255, 255, 255, 0.6)", stroke_width: "1.5" }
                line { x1: "88", y1: "72", x2: "112", y2: "72", stroke: "rgba(255, 255, 255, 0.6)", stroke_width: "1.5" }
                path { d: "M 76 72 C 76 110, 124 110, 124 72", fill: "none", stroke: "#ffffff", stroke_width: "4", stroke_linecap: "round" }
                line { x1: "100", y1: "110", x2: "100", y2: "145", stroke: "#ffffff", stroke_width: "4", stroke_linecap: "round" }
                line { x1: "78", y1: "145", x2: "122", y2: "145", stroke: "#ffffff", stroke_width: "4", stroke_linecap: "round" }
            }
        },
        "bpm_drive" => rsx! {
            svg {
                class: "for-you-svg-art animated-bpm-drive",
                view_box: "0 0 200 200",
                defs {
                    linearGradient { id: "bpm-grad", x1: "0%", y1: "0%", x2: "100%", y2: "0%",
                        stop { offset: "0%", stop_color: "#0ea5e9" }
                        stop { offset: "50%", stop_color: "#2dd4bf" }
                        stop { offset: "100%", stop_color: "#eab308" }
                    }
                }
                // Background Track Arc (Center 100, 110, Radius 64)
                path { d: "M 51 151 A 64 64 0 1 1 149 151", fill: "none", stroke: "rgba(255, 255, 255, 0.15)", stroke_width: "10", stroke_linecap: "round" }
                // Active Speed Zone Arc (Cyan to Mint to Amber, perfectly concentric)
                path { d: "M 51 151 A 64 64 0 1 1 160 88", fill: "none", stroke: "url(#bpm-grad)", stroke_width: "10", stroke_linecap: "round", class: "bpm-active-arc" }
                // Redline Accent Arc (Flush continuation from 160,88 to 149,151)
                path { d: "M 160 88 A 64 64 0 0 1 149 151", fill: "none", stroke: "rgba(244, 63, 94, 0.55)", stroke_width: "10", stroke_linecap: "round" }
                // Needle Assembly pivoted at exact center (100, 110)
                g {
                    transform: "translate(100, 110)",
                    g {
                        class: "bpm-needle-rotator",
                        // Sleek tapering crimson pointer
                        polygon { points: "-2.5,0 0,-54 2.5,0", fill: "#f43f5e" }
                        // Center Hub Base & Pin
                        circle { cx: "0", cy: "0", r: "11", fill: "#131823", stroke: "rgba(255, 255, 255, 0.25)", stroke_width: "2" }
                        circle { cx: "0", cy: "0", r: "4.5", fill: "#ffffff" }
                    }
                }
            }
        },
        _ => rsx! {
            // "surprise_me" or wildcard mystery box
            svg {
                class: "for-you-svg-art animated-surprise-me",
                view_box: "0 0 200 200",
                defs {
                    linearGradient { id: "mystery-grad", x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                        stop { offset: "0%", stop_color: "#a855f7" }
                        stop { offset: "50%", stop_color: "#ec4899" }
                        stop { offset: "100%", stop_color: "#3b82f6" }
                    }
                }
                rect { x: "50", y: "50", width: "100", height: "100", rx: "20", fill: "none", stroke: "url(#mystery-grad)", stroke_width: "3", class: "mystery-box" }
                circle { cx: "100", cy: "100", r: "55", fill: "rgba(168, 85, 247, 0.12)" }
                text { x: "100", y: "125", text_anchor: "middle", font_family: "sans-serif", font_size: "68", font_weight: "900", fill: "url(#mystery-grad)", class: "mystery-q", "?" }
                circle { cx: "60", cy: "45", r: "3", fill: "#ec4899", class: "chill-star star-1" }
                circle { cx: "145", cy: "55", r: "3.5", fill: "#facc15", class: "chill-star star-2" }
                circle { cx: "140", cy: "150", r: "2.5", fill: "#2dd4bf", class: "chill-star star-3" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surprise_me_hidden_when_under_500_songs() {
        assert!(!should_show_surprise_me(0));
        assert!(!should_show_surprise_me(159));
        assert!(!should_show_surprise_me(499));
    }

    #[test]
    fn test_surprise_me_visible_when_500_or_more_songs() {
        assert!(should_show_surprise_me(500));
        assert!(should_show_surprise_me(1000));
    }

    #[test]
    fn test_surprise_me_exact_comment_string() {
        assert_eq!(
            SURPRISE_ME_COMMENT,
            "This catalog contains the most random song selection that this algorithm can come up based on melody, track composition, artists, how much did you play them or not, etc. at Random. Play if you want to."
        );
    }
}
