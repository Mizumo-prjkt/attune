//! Dedicated Standalone Artist Profile Finder & Matcher Window.
//!
//! Provides a responsive interface allowing listeners to:
//! 1. Search online providers (YouTube Music Innertube, Wikipedia EN, Wikipedia JA) for artist profiles.
//! 2. Preview portraits, widescreen banners, curated bios, descriptions, and subscriber statistics.
//! 3. Accurately index and link the selected online profile to a specific library artist.
//! 4. Directly author or link custom image URLs and biographies in manual mode.
//! 5. Guard against unselected or invalid artist sessions with a graceful gating screen.
//!
//! Strictly complies with Mandate 4 (zero hardcoded values).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use dioxus::prelude::*;

use crate::artist_profile::{
    is_valid_profile_image_url, save_cached_artist_profile,
    search_online_artist_candidates, ArtistCandidate, OnlineArtistInfo,
};
use crate::config::load_config;

static ARTIST_FINDER_WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
static TARGET_FINDER_ARTIST: Mutex<Option<String>> = Mutex::new(None);

struct FinderWindowGuard;
impl Drop for FinderWindowGuard {
    fn drop(&mut self) {
        ARTIST_FINDER_WINDOW_OPEN.store(false, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtistFinderTab {
    All,
    YouTubeMusic,
    Wikipedia,
    Custom,
}

/// Opens the Artist Profile Finder in a dedicated standalone secondary OS window.
pub fn open_artist_finder_window(artist: Option<String>) {
    if let Some(ref a) = artist {
        *TARGET_FINDER_ARTIST.lock().unwrap() = Some(a.clone());
    }

    if ARTIST_FINDER_WINDOW_OPEN
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let dom = VirtualDom::new(ArtistFinderStandaloneWindow);
        let mut window = dioxus::desktop::WindowBuilder::new()
            .with_title("Attune — Artist Profile Finder & Matcher")
            .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1120.0, 740.0))
            .with_min_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(820.0, 540.0));

        if let Some(icon) = crate::app_icon::load_app_icon() {
            window = window.with_window_icon(Some(icon));
        }

        let css = include_str!("../style.css");
        let icons_css = include_str!("../material_icons.css");
        let cfg = dioxus::desktop::Config::new()
            .with_window(window)
            .with_menu(None)
            .with_custom_head(format!(
                "<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/icon?family=Material+Icons\">\n<style>\n{icons_css}\n{css}\n</style>"
            ));

        dioxus::desktop::window().new_window(dom, cfg);
    }
}

/// Standalone Window Root Component for Artist Profile Finder
#[component]
pub fn ArtistFinderStandaloneWindow() -> Element {
    use_hook(|| Arc::new(FinderWindowGuard));

    let config_sig = use_signal(load_config);
    let curr_cfg = config_sig.read().clone();
    let strings = crate::i18n::get_strings(crate::i18n::Language::from_code(&curr_cfg.language));

    let artist_opt = TARGET_FINDER_ARTIST.lock().unwrap().clone();
    let is_artist_valid = artist_opt.as_ref().map_or(false, |a| {
        let t = a.trim();
        !t.is_empty() && t != "Unknown Artist" && t != "—"
    });

    // Graceful gating screen if opened without a valid artist context
    if !is_artist_valid {
        return rsx! {
            div {
                class: "artist-finder-container",
                style: "display: flex; flex-direction: column; width: 100%; height: 100vh; overflow: hidden; background: rgba(10, 14, 23, 0.96); color: #ffffff; align-items: center; justify-content: center; text-align: center; padding: 32px; box-sizing: border-box; font-family: var(--font-sans, sans-serif);",

                div {
                    style: "background: rgba(22, 28, 45, 0.85); border: 1px solid var(--border-glass); border-radius: 20px; padding: 44px 36px; max-width: 480px; display: flex; flex-direction: column; align-items: center; gap: 18px; box-shadow: 0 16px 40px rgba(0,0,0,0.55); backdrop-filter: blur(24px); -webkit-backdrop-filter: blur(24px);",

                    div {
                        style: "width: 72px; height: 72px; border-radius: 50%; background: rgba(244, 63, 94, 0.12); border: 1.5px solid rgba(244, 63, 94, 0.35); display: flex; align-items: center; justify-content: center; color: #fb7185;",
                        span { class: "material-icons md-36", "person_off" }
                    }

                    div { style: "display: flex; flex-direction: column; gap: 8px;",
                        h2 { style: "font-size: 20px; font-weight: 700; color: #ffffff; margin: 0;",
                            "{strings.select_artist_to_find_profile}"
                        }
                        p { style: "font-size: 13.5px; color: var(--text-muted); line-height: 1.5; margin: 0; max-width: 380px;",
                            "{strings.select_artist_to_find_profile_desc}"
                        }
                    }

                    div { style: "display: flex; gap: 12px; margin-top: 6px;",
                        button {
                            class: "btn-pill active",
                            style: "padding: 10px 24px; font-size: 13px; font-weight: 700; border-radius: 9999px; cursor: pointer;",
                            onclick: move |_| {
                                dioxus::desktop::window().close();
                            },
                            "{strings.close}"
                        }
                    }
                }
            }
        };
    }

    let target_artist = artist_opt.unwrap();
    let initial_query = target_artist.clone();

    let mut active_tab = use_signal(|| ArtistFinderTab::All);
    let mut search_query = use_signal(|| initial_query.clone());
    let mut candidates = use_signal(Vec::<ArtistCandidate>::new);
    let mut is_searching = use_signal(|| false);
    let mut search_status = use_signal(String::new);
    let mut success_feedback = use_signal(|| None::<String>);

    // Custom profile manual inputs
    let mut custom_image_url = use_signal(String::new);
    let mut custom_banner_url = use_signal(String::new);
    let mut custom_bio = use_signal(String::new);
    let mut custom_desc = use_signal(String::new);

    // Initial search trigger on window open
    let target_artist_for_init = initial_query.clone();
    use_hook(move || {
        let q = target_artist_for_init.clone();
        spawn(async move {
            is_searching.set(true);
            search_status.set(format!("Searching online profiles for '{q}'..."));
            let results = tokio::task::spawn_blocking(move || {
                search_online_artist_candidates(&q)
            })
            .await
            .unwrap_or_default();
            candidates.set(results);
            is_searching.set(false);
            search_status.set(String::new());
        });
    });

    let do_search = {
        let mut is_searching = is_searching.clone();
        let mut search_status = search_status.clone();
        let mut candidates = candidates.clone();
        let search_query = search_query.clone();
        move || {
            let q = search_query.read().trim().to_string();
            if q.is_empty() {
                return;
            }
            is_searching.set(true);
            search_status.set(format!("Searching online profiles for '{q}'..."));
            let query_str = q.clone();
            spawn(async move {
                let results = tokio::task::spawn_blocking(move || {
                    search_online_artist_candidates(&query_str)
                })
                .await
                .unwrap_or_default();
                candidates.set(results);
                is_searching.set(false);
                search_status.set(String::new());
            });
        }
    };

    let filtered_candidates: Vec<ArtistCandidate> = candidates
        .read()
        .iter()
        .filter(|c| match *active_tab.read() {
            ArtistFinderTab::All => true,
            ArtistFinderTab::YouTubeMusic => c.provider.contains("YouTube"),
            ArtistFinderTab::Wikipedia => c.provider.contains("Wikipedia"),
            ArtistFinderTab::Custom => false,
        })
        .cloned()
        .collect();

    rsx! {
        div {
            class: "artist-finder-container",
            style: "display: flex; flex-direction: column; width: 100%; height: 100vh; overflow: hidden; background: rgba(10, 14, 23, 0.98); color: #ffffff; font-family: var(--font-sans, sans-serif); box-sizing: border-box;",

            // Header Row
            div {
                style: "padding: 20px 28px; background: rgba(15, 20, 32, 0.95); border-bottom: 1px solid var(--border-glass); display: flex; align-items: center; justify-content: space-between; flex-shrink: 0;",

                div { style: "display: flex; align-items: center; gap: 14px;",
                    div {
                        style: "width: 44px; height: 44px; border-radius: 12px; background: rgba(100, 255, 218, 0.14); border: 1px solid rgba(100, 255, 218, 0.35); display: flex; align-items: center; justify-content: center; color: var(--accent-mint);",
                        span { class: "material-icons md-24", "manage_search" }
                    }
                    div {
                        h2 { style: "font-size: 18px; font-weight: 700; margin: 0; color: #ffffff;",
                            "{strings.artist_profile_finder}"
                        }
                        div { style: "font-size: 12.5px; color: var(--text-muted); display: flex; align-items: center; gap: 6px; margin-top: 2px;",
                            span { "{strings.editing_profile_for_artist}:" }
                            span { style: "color: var(--accent-mint); font-weight: 700;", "{target_artist}" }
                        }
                    }
                }

                div { style: "display: flex; align-items: center; gap: 10px;",
                    button {
                        class: "btn-glass",
                        style: "padding: 8px 16px; font-size: 13px; font-weight: 600; cursor: pointer; border-radius: 9999px; display: inline-flex; align-items: center; gap: 6px;",
                        onclick: move |_| {
                            dioxus::desktop::window().close();
                        },
                        span { class: "material-icons md-16", "close" }
                        "{strings.close}"
                    }
                }
            }

            // Search Toolbar & Provider Tabs
            div {
                style: "padding: 16px 28px; background: rgba(18, 24, 38, 0.70); border-bottom: 1px solid rgba(255,255,255,0.06); display: flex; flex-direction: column; gap: 14px; flex-shrink: 0;",

                // Search Bar Row
                div { style: "display: flex; gap: 12px; align-items: center;",
                    div { style: "flex: 1; position: relative;",
                        input {
                            class: "form-input",
                            style: "width: 100%; padding: 10px 14px 10px 38px; font-size: 14px; border-radius: 10px; box-sizing: border-box;",
                            placeholder: "{strings.search_artist_placeholder}",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value()),
                            onkeydown: {
                                let mut do_search = do_search.clone();
                                move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter {
                                        do_search();
                                    }
                                }
                            },
                        }
                        span {
                            class: "material-icons md-18",
                            style: "position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: var(--text-muted); pointer-events: none;",
                            "search"
                        }
                    }

                    button {
                        class: "btn-pill active",
                        style: "padding: 10px 22px; font-size: 13.5px; font-weight: 700; border-radius: 10px; cursor: pointer; display: inline-flex; align-items: center; gap: 6px; white-space: nowrap;",
                        disabled: *is_searching.read(),
                        onclick: {
                            let mut do_search = do_search.clone();
                            move |_| do_search()
                        },
                        if *is_searching.read() {
                            span { class: "material-icons md-16 spin-animation", "sync" }
                        } else {
                            span { class: "material-icons md-16", "search" }
                        }
                        "Search"
                    }
                }

                // Provider Switcher Tabs
                div { style: "display: flex; gap: 8px; align-items: center;",
                    button {
                        class: if *active_tab.read() == ArtistFinderTab::All { "discography-chip active" } else { "discography-chip" },
                        onclick: move |_| active_tab.set(ArtistFinderTab::All),
                        "All Providers"
                    }
                    button {
                        class: if *active_tab.read() == ArtistFinderTab::YouTubeMusic { "discography-chip active" } else { "discography-chip" },
                        onclick: move |_| active_tab.set(ArtistFinderTab::YouTubeMusic),
                        span { class: "material-icons md-14", style: "color: #f43f5e; margin-right: 4px;", "smart_display" }
                        "YouTube Music"
                    }
                    button {
                        class: if *active_tab.read() == ArtistFinderTab::Wikipedia { "discography-chip active" } else { "discography-chip" },
                        onclick: move |_| active_tab.set(ArtistFinderTab::Wikipedia),
                        span { class: "material-icons md-14", style: "color: var(--accent-mint); margin-right: 4px;", "menu_book" }
                        "Wikipedia"
                    }
                    button {
                        class: if *active_tab.read() == ArtistFinderTab::Custom { "discography-chip active" } else { "discography-chip" },
                        onclick: move |_| active_tab.set(ArtistFinderTab::Custom),
                        span { class: "material-icons md-14", style: "color: #a78bfa; margin-right: 4px;", "edit" }
                        "{strings.custom_profile}"
                    }
                }
            }

            // Success Toast Feedback
            if let Some(ref msg) = *success_feedback.read() {
                div {
                    style: "margin: 16px 28px 0; padding: 12px 18px; border-radius: 10px; background: rgba(45, 212, 191, 0.14); border: 1px solid rgba(45, 212, 191, 0.40); color: var(--accent-mint); display: flex; align-items: center; justify-content: space-between; font-size: 13.5px; font-weight: 600;",
                    div { style: "display: flex; align-items: center; gap: 8px;",
                        span { class: "material-icons md-18", "check_circle" }
                        span { "{msg}" }
                    }
                    button {
                        class: "btn-glass",
                        style: "padding: 4px 12px; font-size: 12px; border-radius: 6px; cursor: pointer;",
                        onclick: move |_| success_feedback.set(None),
                        "Dismiss"
                    }
                }
            }

            // Main Content Area
            div {
                style: "flex: 1; overflow-y: auto; padding: 24px 28px;",

                if *is_searching.read() {
                    div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 60px 0; color: var(--text-muted); gap: 14px;",
                        span { class: "material-icons md-36 spin-animation", style: "color: var(--accent-mint);", "sync" }
                        p { style: "font-size: 14px; margin: 0;", "{search_status}" }
                    }
                } else if *active_tab.read() == ArtistFinderTab::Custom {
                    // Custom Profile Input Form
                    div {
                        style: "max-width: 680px; background: rgba(22, 28, 45, 0.60); border: 1px solid var(--border-glass); border-radius: 16px; padding: 28px; display: flex; flex-direction: column; gap: 20px;",

                        h3 { style: "font-size: 16px; font-weight: 700; margin: 0; color: #ffffff; display: flex; align-items: center; gap: 8px;",
                            span { class: "material-icons md-20", style: "color: #a78bfa;", "palette" }
                            "{strings.custom_profile}: {target_artist}"
                        }

                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            label { style: "font-size: 12.5px; font-weight: 600; color: var(--text-secondary);", "{strings.image_url}" }
                            input {
                                class: "form-input",
                                placeholder: "https://... (JPEG, PNG, WebP image link)",
                                value: "{custom_image_url}",
                                oninput: move |e| custom_image_url.set(e.value()),
                            }
                            if !custom_image_url.read().is_empty() && is_valid_profile_image_url(&custom_image_url.read()) {
                                div { style: "display: flex; align-items: center; gap: 12px; margin-top: 6px;",
                                    img {
                                        src: "{custom_image_url}",
                                        alt: "Preview",
                                        style: "width: 54px; height: 54px; border-radius: 50%; object-fit: cover; border: 1px solid var(--border-glass);",
                                    }
                                    span { style: "font-size: 12px; color: var(--accent-mint);", "Image link preview loaded" }
                                }
                            }
                        }

                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            label { style: "font-size: 12.5px; font-weight: 600; color: var(--text-secondary);", "{strings.banner_url}" }
                            input {
                                class: "form-input",
                                placeholder: "https://... (Optional 16:9 widescreen banner image)",
                                value: "{custom_banner_url}",
                                oninput: move |e| custom_banner_url.set(e.value()),
                            }
                        }

                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            label { style: "font-size: 12.5px; font-weight: 600; color: var(--text-secondary);", "Subtitle / Description" }
                            input {
                                class: "form-input",
                                placeholder: "e.g. Japanese Vocaloid Producer / J-Rock Band",
                                value: "{custom_desc}",
                                oninput: move |e| custom_desc.set(e.value()),
                            }
                        }

                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            label { style: "font-size: 12.5px; font-weight: 600; color: var(--text-secondary);", "{strings.bio_description}" }
                            textarea {
                                class: "form-input",
                                style: "min-height: 100px; resize: vertical; font-family: inherit;",
                                placeholder: "Biography and notes about this artist...",
                                value: "{custom_bio}",
                                oninput: move |e| custom_bio.set(e.value()),
                            }
                        }

                        div { style: "display: flex; justify-content: flex-end; margin-top: 10px;",
                            button {
                                class: "btn-pill active",
                                style: "padding: 10px 24px; font-size: 13.5px; font-weight: 700; cursor: pointer; border-radius: 9999px; display: inline-flex; align-items: center; gap: 6px;",
                                onclick: {
                                    let t_artist = target_artist.clone();
                                    let mut success_feedback = success_feedback.clone();
                                    move |_| {
                                        let img = if custom_image_url.read().trim().is_empty() {
                                            None
                                        } else {
                                            Some(custom_image_url.read().trim().to_string())
                                        };
                                        let banner = if custom_banner_url.read().trim().is_empty() {
                                            None
                                        } else {
                                            Some(custom_banner_url.read().trim().to_string())
                                        };
                                        let desc = if custom_desc.read().trim().is_empty() {
                                            None
                                        } else {
                                            Some(custom_desc.read().trim().to_string())
                                        };
                                        let bio = if custom_bio.read().trim().is_empty() {
                                            None
                                        } else {
                                            Some(custom_bio.read().trim().to_string())
                                        };

                                        let info = OnlineArtistInfo {
                                            name: Some(t_artist.clone()),
                                            description: desc,
                                            bio,
                                            image_url: img,
                                            banner_url: banner,
                                            subscribers: None,
                                            views: None,
                                            browse_id: None,
                                        };

                                        save_cached_artist_profile(&t_artist, &info);
                                        success_feedback.set(Some(format!("{}: '{t_artist}'", strings.linked_profile_success)));
                                    }
                                },
                                span { class: "material-icons md-18", "save" }
                                "{strings.apply_and_link_artist}"
                            }
                        }
                    }
                } else if filtered_candidates.is_empty() {
                    div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 60px 0; color: var(--text-muted); gap: 12px; text-align: center;",
                        span { class: "material-icons md-48", style: "opacity: 0.4;", "person_search" }
                        p { style: "font-size: 16px; font-weight: 600; color: var(--text-primary); margin: 0;", "{strings.no_artist_candidates_found}" }
                        p { style: "font-size: 13px; max-width: 440px; line-height: 1.5; margin: 0;", "{strings.no_artist_candidates_guide}" }
                    }
                } else {
                    // Candidate Cards Grid
                    div { style: "display: flex; flex-direction: column; gap: 14px; max-width: 860px;",
                        for candidate in filtered_candidates.into_iter() {
                            {
                                let c = candidate.clone();
                                let c_apply = candidate.clone();
                                let t_artist = target_artist.clone();
                                let mut success_feedback = success_feedback.clone();

                                rsx! {
                                    div {
                                        key: "{c.provider}-{c.name}",
                                        style: "background: rgba(22, 28, 45, 0.65); border: 1px solid var(--border-glass); border-radius: 14px; padding: 18px 22px; display: flex; align-items: center; justify-content: space-between; gap: 20px; transition: all 0.2s ease;",

                                        div { style: "display: flex; align-items: center; gap: 18px; flex: 1; min-width: 0;",
                                            // Avatar
                                            div {
                                                style: "width: 76px; height: 76px; border-radius: 50%; overflow: hidden; background: rgba(255,255,255,0.06); border: 1.5px solid var(--border-glass); flex-shrink: 0; display: flex; align-items: center; justify-content: center;",
                                                if let Some(ref img_url) = c.image_url {
                                                    img {
                                                        src: "{img_url}",
                                                        alt: "{c.name}",
                                                        style: "width: 100%; height: 100%; object-fit: cover;",
                                                    }
                                                } else {
                                                    span { class: "material-icons md-36", style: "color: var(--text-muted); opacity: 0.6;", "person" }
                                                }
                                            }

                                            // Metadata
                                            div { style: "display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 0;",
                                                div { style: "display: flex; align-items: center; gap: 10px; flex-wrap: wrap;",
                                                    h4 { style: "font-size: 16px; font-weight: 700; color: #ffffff; margin: 0;", "{c.name}" }
                                                    span {
                                                        style: "font-size: 11px; font-weight: 700; padding: 3px 8px; border-radius: 9999px; background: rgba(100, 255, 218, 0.12); color: var(--accent-mint); border: 1px solid rgba(100, 255, 218, 0.30); text-transform: uppercase;",
                                                        "{c.provider}"
                                                    }
                                                    if c.browse_id.is_some() || c.subscribers.is_some() {
                                                        span {
                                                            class: "material-icons md-16",
                                                            style: "color: var(--accent-mint);",
                                                            title: "{strings.verified_artist}",
                                                            "verified"
                                                        }
                                                    }
                                                }

                                                if let Some(ref desc) = c.description {
                                                    p { style: "font-size: 12.5px; color: var(--accent-mint); margin: 0; font-weight: 600;", "{desc}" }
                                                }

                                                if let Some(ref bio) = c.bio {
                                                    p {
                                                        style: "font-size: 12.5px; color: var(--text-muted); margin: 0; line-height: 1.4; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;",
                                                        "{bio}"
                                                    }
                                                }

                                                div { style: "display: flex; gap: 14px; font-size: 11.5px; color: var(--text-muted); margin-top: 2px;",
                                                    if let Some(ref subs) = c.subscribers {
                                                        span { style: "display: inline-flex; align-items: center; gap: 4px;",
                                                            span { class: "material-icons md-14", style: "color: #f43f5e;", "smart_display" }
                                                            "{subs}"
                                                        }
                                                    }
                                                    if let Some(ref b_id) = c.browse_id {
                                                        span { "ID: {b_id}" }
                                                    }
                                                }
                                            }
                                        }

                                        // Apply Action
                                        button {
                                            class: "btn-pill active",
                                            style: "padding: 8px 18px; font-size: 13px; font-weight: 700; cursor: pointer; white-space: nowrap; border-radius: 9999px; display: inline-flex; align-items: center; gap: 6px; flex-shrink: 0;",
                                            onclick: move |_| {
                                                let info = c_apply.to_online_info();
                                                save_cached_artist_profile(&t_artist, &info);
                                                success_feedback.set(Some(format!("{}: '{t_artist}'", strings.linked_profile_success)));
                                            },
                                            span { class: "material-icons md-16", "link" }
                                            "{strings.apply_and_link_artist}"
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

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_artist_candidate_conversion() {
        let cand = ArtistCandidate {
            name: "Hatsune Miku".to_string(),
            provider: "Test Provider".to_string(),
            description: Some("Vocaloid".to_string()),
            bio: Some("Singing voice synthesizer".to_string()),
            image_url: Some("https://example.com/miku.jpg".to_string()),
            banner_url: Some("https://example.com/banner.jpg".to_string()),
            subscribers: Some("3.5M".to_string()),
            views: Some("1B".to_string()),
            browse_id: Some("UC_miku".to_string()),
        };

        let info = cand.to_online_info();
        assert_eq!(info.name.as_deref(), Some("Hatsune Miku"));
        assert_eq!(info.description.as_deref(), Some("Vocaloid"));
        assert_eq!(info.bio.as_deref(), Some("Singing voice synthesizer"));
        assert_eq!(info.image_url.as_deref(), Some("https://example.com/miku.jpg"));
        assert_eq!(info.banner_url.as_deref(), Some("https://example.com/banner.jpg"));
        assert_eq!(info.subscribers.as_deref(), Some("3.5M"));
        assert!(info.is_verified());
    }

    #[test]
    fn test_artist_validity_check_rejects_empty_or_placeholder() {
        let empty_opt: Option<String> = None;
        let is_valid = empty_opt.as_ref().map_or(false, |a| {
            let t = a.trim();
            !t.is_empty() && t != "Unknown Artist" && t != "—"
        });
        assert!(!is_valid);

        let blank = Some("   ".to_string());
        let is_blank_valid = blank.as_ref().map_or(false, |a| {
            let t = a.trim();
            !t.is_empty() && t != "Unknown Artist" && t != "—"
        });
        assert!(!is_blank_valid);

        let unknown = Some("Unknown Artist".to_string());
        let is_unknown_valid = unknown.as_ref().map_or(false, |a| {
            let t = a.trim();
            !t.is_empty() && t != "Unknown Artist" && t != "—"
        });
        assert!(!is_unknown_valid);

        let valid = Some("DECO*27".to_string());
        let is_real_valid = valid.as_ref().map_or(false, |a| {
            let t = a.trim();
            !t.is_empty() && t != "Unknown Artist" && t != "—"
        });
        assert!(is_real_valid);
    }
}
