use dioxus::prelude::*;

#[component]
pub fn About() -> Element {
    let pkg_version = env!("CARGO_PKG_VERSION");
    let target_os = if cfg!(target_os = "windows") { "Windows" } else { "Linux (WebKitGTK)" };

    rsx! {
        div { class: "about-stage",
            style: "max-width: 720px; margin: 0 auto; padding-top: 10px;",

            // Header Banner
            div { style: "display: flex; align-items: center; gap: 20px; background: var(--bg-card); padding: 24px; border-radius: 18px; border: 1px solid var(--border-glass); margin-bottom: 24px;",
                div {
                    style: "width: 72px; height: 72px; border-radius: 16px; background: linear-gradient(135deg, rgba(45,212,191,0.2), rgba(168,85,247,0.2)); display: flex; align-items: center; justify-content: center; border: 1px solid var(--border-glass); color: var(--accent-mint);",
                    span { class: "material-icons md-36", "tune" }
                }
                div {
                    h1 { style: "font-size: 26px; font-weight: 700;", "Attune Desktop" }
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
                h2 { style: "font-size: 18px; font-weight: 700; margin-bottom: 14px;", "Architecture & Technology" }
                
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
                        span { style: "color: #eab308; font-weight: 600;", "Synchronized Lyrics" }
                        p { style: "color: var(--text-secondary); margin-top: 4px;", "Real-time timestamped .lrc parsing with active glowing text shadow and sub-second seeking." }
                    }
                }
            }

            // Ecosystem & Updates
            div { style: "background: var(--bg-card); padding: 24px; border-radius: 16px; border: 1px solid var(--border-glass);",
                h2 { style: "font-size: 18px; font-weight: 700; margin-bottom: 12px;", "Open Source & Distribution" }
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
