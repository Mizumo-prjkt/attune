use dioxus::prelude::*;

#[component]
pub fn About() -> Element {
    let strings = crate::i18n::current_strings();
    let pkg_version = crate::config::app_version();
    let target_os = if cfg!(target_os = "windows") { "Windows (WebView2)" } else { "Linux (WebKitGTK)" };

    rsx! {
        div { class: "about-stage",
            style: "max-width: 720px; margin: 0 auto; padding-top: 10px;",

            // Header Banner
            div { style: "display: flex; align-items: center; gap: 20px; background: var(--bg-card); padding: 24px; border-radius: 18px; border: 1px solid var(--border-glass); margin-bottom: 24px;",
                img {
                    src: "{crate::app_icon::app_icon_tile_data_url()}",
                    style: "width: 72px; height: 72px; border-radius: 16px; object-fit: cover; flex-shrink: 0; box-shadow: 0 8px 24px rgba(0,0,0,0.4), 0 0 20px rgba(45,212,191,0.2);",
                    alt: "Attune Official Logo",
                }
                div {
                    h1 { style: "font-size: 26px; font-weight: 700;", "{strings.about_attune}" }
                    p { style: "color: var(--accent-mint); font-weight: 600; font-size: 14px; margin-top: 2px;",
                        "Version {pkg_version} • {target_os}"
                    }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin-top: 4px;",
                        "Intelligent DJ-grade music player with audio-profile-driven transition & remixing engine"
                    }
                }
            }

            // Architecture Stack
            div { style: "background: var(--bg-card); padding: 24px; border-radius: 16px; border: 1px solid var(--border-glass); margin-bottom: 20px;",
                h2 { style: "font-size: 18px; font-weight: 700; margin-bottom: 14px;", "{strings.architecture_and_technology}" }

                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 14px; font-size: 13px;",
                    div { style: "background: rgba(255,255,255,0.04); padding: 12px; border-radius: 8px; border: 1px solid var(--border-glass);",
                        span { style: "color: var(--accent-mint); font-weight: 600;", "Headless Audio Engine" }
                        p { style: "color: var(--text-secondary); margin-top: 4px;", "100% Rust in-process pipeline via attune-core, DualDeckMixer, and cpal with zero IPC latency." }
                    }
                    div { style: "background: rgba(255,255,255,0.04); padding: 12px; border-radius: 8px; border: 1px solid var(--border-glass);",
                        span { style: "color: var(--accent-purple); font-weight: 600;", "Desktop Presentation Layer" }
                        p { style: "color: var(--text-secondary); margin-top: 4px;", "Dioxus 0.6 Desktop with native WebKitGTK / WebView2, hardware accelerated CSS3 Liquid Glass." }
                    }
                    div { style: "background: rgba(255,255,255,0.04); padding: 12px; border-radius: 8px; border: 1px solid var(--border-glass);",
                        span { style: "color: var(--accent-rose); font-weight: 600;", "JIT Transition Planner" }
                        p { style: "color: var(--text-secondary); margin-top: 4px;", "Dynamic 5-band audio profiling, Camelot wheel key progression, and 6 DJ transition archetypes." }
                    }
                    div { style: "background: rgba(255,255,255,0.04); padding: 12px; border-radius: 8px; border: 1px solid var(--border-glass);",
                        span { style: "color: #eab308; font-weight: 600;", "Modular C-ABI Shared Libraries" }
                        p { style: "color: var(--text-secondary); margin-top: 4px;", "Runtime-certified libattune_procmgr, libattune_watchguard, and libattune_music_metadata_changer." }
                    }
                }
            }

            // Special Thanks & Open Source Credits
            div { style: "background: var(--bg-card); padding: 24px; border-radius: 16px; border: 1px solid var(--border-glass); margin-bottom: 20px;",
                h2 { style: "font-size: 18px; font-weight: 700; margin-bottom: 12px;", "{strings.special_thanks_and_credits}" }
                p { style: "color: var(--text-secondary); font-size: 13px; margin-bottom: 16px; line-height: 1.5;",
                    "Attune is crafted on the shoulders of remarkable open-source projects, community innovators, and audio pioneers:"
                }
                div { style: "display: flex; flex-direction: column; gap: 10px;",
                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 10px; padding: 12px 16px; display: flex; justify-content: space-between; align-items: center;",
                        div {
                            div { style: "font-weight: 600; font-size: 14px; color: #fff; display: flex; align-items: center; gap: 6px;",
                                span { class: "material-icons md-16", style: "color: var(--accent-mint);", "stars" }
                                "Better Lyrics"
                            }
                            div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                "Lyrics animation techniques, fluid cubic-bezier easing curves, and custom CSS theme architecture."
                            }
                        }
                        a {
                            class: "btn-pill",
                            style: "font-size: 11.5px; flex-shrink: 0; margin-left: 12px;",
                            href: "https://github.com/better-lyrics/better-lyrics",
                            target: "_blank",
                            "better-lyrics/better-lyrics ↗"
                        }
                    }
                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 10px; padding: 12px 16px; display: flex; justify-content: space-between; align-items: center;",
                        div {
                            div { style: "font-weight: 600; font-size: 14px; color: #fff; display: flex; align-items: center; gap: 6px;",
                                span { class: "material-icons md-16", style: "color: var(--accent-mint);", "subtitles" }
                                "BiniLyrics (<am-lyrics>)"
                            }
                            div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                "Apple Music TTML syllable parser & rich sync components under MPL-2.0."
                            }
                        }
                        a {
                            class: "btn-pill",
                            style: "font-size: 11.5px; flex-shrink: 0; margin-left: 12px;",
                            href: "https://github.com/binimum/am-lyrics",
                            target: "_blank",
                            "am-lyrics ↗"
                        }
                    }
                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 10px; padding: 12px 16px; display: flex; justify-content: space-between; align-items: center;",
                        div {
                            div { style: "font-weight: 600; font-size: 14px; color: #fff; display: flex; align-items: center; gap: 6px;",
                                span { class: "material-icons md-16", style: "color: var(--accent-mint);", "cloud_queue" }
                                "LRCLIB"
                            }
                            div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                "Community-driven synchronized lyrics database and open REST API."
                            }
                        }
                        a {
                            class: "btn-pill",
                            style: "font-size: 11.5px; flex-shrink: 0; margin-left: 12px;",
                            href: "https://lrclib.net",
                            target: "_blank",
                            "lrclib.net ↗"
                        }
                    }
                }
            }

            // Ecosystem & Updates
            div { style: "background: var(--bg-card); padding: 24px; border-radius: 16px; border: 1px solid var(--border-glass);",
                h2 { style: "font-size: 18px; font-weight: 700; margin-bottom: 12px;", "{strings.open_source_distribution}" }
                p { style: "color: var(--text-secondary); font-size: 13.5px; line-height: 1.5; margin-bottom: 16px;",
                    "Attune is completely open-source. Desktop builds run native Rust for Linux and Windows, while Android companion builds run Flutter with the attune-core FFI engine."
                }

                div { style: "display: flex; gap: 12px;",
                    a {
                        class: "btn-pill",
                        href: "https://github.com/Mizumo-prjkt/attune",
                        target: "_blank",
                        "GitHub Repository ↗"
                    }
                    a {
                        class: "btn-pill",
                        href: "https://github.com/Mizumo-prjkt/attune/releases",
                        target: "_blank",
                        "Obtainium / Releases ↗"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_credits_contain_better_lyrics() {
        let repo_url = "https://github.com/better-lyrics/better-lyrics";
        assert!(repo_url.contains("better-lyrics/better-lyrics"));
    }
}
