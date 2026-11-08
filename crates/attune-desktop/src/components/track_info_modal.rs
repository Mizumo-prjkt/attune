use dioxus::prelude::*;
use std::io::Write;
use crate::library::{fmt_ms, TrackEntry};

/// Reusable modal presenting comprehensive track metadata, creative credits,
/// audio profile metrics, and file location details.
#[component]
pub fn TrackInfoModal(
    track: TrackEntry,
    on_close: EventHandler<()>,
) -> Element {
    let mut copied_feedback = use_signal(|| false);

    let dur_str = fmt_ms(track.duration_ms);
    let vocalist_str = track.vocalist.clone().unwrap_or_else(|| {
        if track.artist.contains("feat.") || track.artist.contains("ft.") {
            crate::artist_profile::clean_primary_artist(&track.artist)
        } else {
            "—".to_string()
        }
    });
    let writer_str = track
        .writer
        .clone()
        .or(track.composer.clone())
        .unwrap_or_else(|| "—".to_string());
    let producer_str = track.producer.clone().unwrap_or_else(|| "—".to_string());
    let sample_rate_str = format!(
        "{} Hz ({:.1} kHz)",
        track.sample_rate,
        track.sample_rate as f32 / 1000.0
    );
    let bit_depth_str = if track.bit_depth > 0 {
        format!("{}-bit", track.bit_depth)
    } else {
        "16-bit".to_string()
    };
    let bpm_str = track
        .bpm
        .map(|b| format!("{:.1} BPM", b))
        .unwrap_or_else(|| "—".to_string());
    let key_str = track.key.clone().unwrap_or_else(|| "—".to_string());
    let genre_str = track.genre.clone().unwrap_or_else(|| "—".to_string());
    let vibe_str = match (&track.vibe_family, track.aggression) {
        (Some(v), Some(a)) => format!("{} (Drive {:.0}%)", v, a * 100.0),
        (Some(v), None) => v.clone(),
        (None, Some(a)) => format!("Drive {:.0}%", a * 100.0),
        (None, None) => "—".to_string(),
    };
    let path_str = track.path.display().to_string();

    rsx! {
        div {
            class: "credits-modal-backdrop",
            onclick: move |_| on_close.call(()),

            div {
                class: "credits-modal-dialog",
                onclick: move |e: MouseEvent| e.stop_propagation(),

                // Header with Album Artwork Preview
                div { class: "credits-modal-header",
                    div { style: "display: flex; gap: 14px; align-items: center; min-width: 0; flex: 1;",
                        if let Some(ref art) = track.artwork_data_url {
                            img {
                                src: "{art}",
                                alt: "{track.title}",
                                style: "width: 52px; height: 52px; border-radius: 10px; object-fit: cover; box-shadow: 0 4px 12px rgba(0,0,0,0.5); border: 1px solid var(--border-glass); flex-shrink: 0;",
                            }
                        } else {
                            div {
                                style: "width: 52px; height: 52px; border-radius: 10px; background: rgba(255,255,255,0.05); display: flex; align-items: center; justify-content: center; color: var(--text-muted); border: 1px solid var(--border-glass); flex-shrink: 0;",
                                span { class: "material-icons md-24", "music_note" }
                            }
                        }
                        div { style: "min-width: 0; flex: 1; overflow: hidden;",
                            h2 { style: "font-size: 18px; font-weight: 700; margin: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{track.title}" }
                            p { style: "font-size: 12.5px; color: var(--text-muted); margin: 3px 0 0 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{track.artist} • {track.album}" }
                        }
                    }
                    button {
                        class: "modal-close-btn",
                        title: "Close",
                        onclick: move |_| on_close.call(()),
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
                                span { class: "credits-value", title: "{track.artist}", "{track.artist}" }
                            }
                            div { class: "credits-cell",
                                span { class: "credits-label", "Album" }
                                span { class: "credits-value", title: "{track.album}", "{track.album}" }
                            }
                            div { class: "credits-cell",
                                span { class: "credits-label", "Vocals" }
                                span { class: "credits-value", title: "{vocalist_str}", "{vocalist_str}" }
                            }
                            div { class: "credits-cell",
                                span { class: "credits-label", "Writers & Composers" }
                                span { class: "credits-value", title: "{writer_str}", "{writer_str}" }
                            }
                            div { class: "credits-cell",
                                span { class: "credits-label", "Producer / Arranger" }
                                span { class: "credits-value", title: "{producer_str}", "{producer_str}" }
                            }
                            div { class: "credits-cell",
                                span { class: "credits-label", "Genre" }
                                span { class: "credits-value", title: "{genre_str}", "{genre_str}" }
                            }
                        }
                    }

                    // Audio Fidelity & Profile
                    div { class: "credits-section",
                        h3 { class: "credits-section-title", "Audio Fidelity & Profile" }
                        div { class: "credits-grid",
                            div { class: "credits-cell",
                                span { class: "credits-label", "Format & Codec" }
                                span { class: "credits-value", "{track.format} ({bit_depth_str})" }
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
                                span { class: "credits-label", "Vibe & Drive" }
                                span { class: "credits-value", title: "{vibe_str}", "{vibe_str}" }
                            }
                        }
                    }

                    // Disk Location
                    div { class: "credits-section",
                        div { style: "display: flex; justify-content: space-between; align-items: center;",
                            h3 { class: "credits-section-title", "File Location" }
                            button {
                                class: "btn-pill",
                                style: "font-size: 11px; padding: 2px 8px; display: inline-flex; align-items: center; gap: 4px;",
                                onclick: {
                                    let p_to_copy = path_str.clone();
                                    move |_| {
                                        if let Ok(mut child) = std::process::Command::new("wl-copy")
                                            .stdin(std::process::Stdio::piped())
                                            .spawn()
                                        {
                                            if let Some(mut stdin) = child.stdin.take() {
                                                let _ = stdin.write_all(p_to_copy.as_bytes());
                                            }
                                        } else if let Ok(mut child) = std::process::Command::new("xclip")
                                            .args(["-selection", "clipboard"])
                                            .stdin(std::process::Stdio::piped())
                                            .spawn()
                                        {
                                            if let Some(mut stdin) = child.stdin.take() {
                                                let _ = stdin.write_all(p_to_copy.as_bytes());
                                            }
                                        }
                                        copied_feedback.set(true);
                                    }
                                },
                                span { class: "material-icons md-14", if *copied_feedback.read() { "check" } else { "content_copy" } }
                                if *copied_feedback.read() { "Copied!" } else { "Copy Path" }
                            }
                        }
                        div { class: "credits-path-box",
                            span { style: "font-family: monospace; font-size: 11.5px; color: var(--text-muted); word-break: break-all;",
                                "{path_str}"
                            }
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
    use std::path::PathBuf;

    #[test]
    fn test_track_info_modal_formatting() {
        let t = TrackEntry {
            path: PathBuf::from("/music/masarada/Liar Dancer/01 Liar Dancer.flac"),
            title: "Liar Dancer".into(),
            artist: "masarada".into(),
            album: "Liar Dancer".into(),
            genre: Some("J-Pop".into()),
            duration_ms: 184_000,
            bpm: Some(133.0),
            key: Some("F Major".into()),
            format: "FLAC".into(),
            sample_rate: 44_100,
            bit_depth: 16,
            artwork_path: None,
            artwork_data_url: None,
            aggression: Some(0.65),
            vibe_family: Some("UpbeatPop".into()),
            composer: Some("masarada".into()),
            writer: Some("masarada".into()),
            vocalist: Some("Kasane Teto".into()),
            producer: None,
        };

        assert_eq!(t.title, "Liar Dancer");
        assert_eq!(t.artist, "masarada");
        assert_eq!(fmt_ms(t.duration_ms), "3:04");
    }
}
