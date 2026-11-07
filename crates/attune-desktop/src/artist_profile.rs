use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

/// Cached artist image URLs from Wikipedia REST API / local resolution.
static ARTIST_IMAGE_CACHE: LazyLock<RwLock<HashMap<String, Option<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone, PartialEq)]
pub struct FavoriteArtistCatalog {
    pub artist: String,
    pub tracks: Vec<TrackEntry>,
    pub total_duration_str: String,
    pub fallback_artwork_url: Option<String>,
    pub rationale: String,
}

/// Computes the primary artist name, stripping collaborative "feat." or "ft." clauses
/// while respecting comma-separated multi-artists.
pub fn clean_primary_artist(raw_artist: &str) -> String {
    let trimmed = raw_artist.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Split on " feat. ", " ft. ", " featuring "
    let lower = trimmed.to_lowercase();
    let cut_pos = if let Some(pos) = lower.find(" feat. ") {
        Some(pos)
    } else if let Some(pos) = lower.find(" ft. ") {
        Some(pos)
    } else if let Some(pos) = lower.find(" featuring ") {
        Some(pos)
    } else if let Some(pos) = lower.find(',') {
        Some(pos)
    } else {
        None
    };

    if let Some(pos) = cut_pos {
        trimmed[..pos].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Identifies the user's favorite artist based on listening history (`recent_tracks`)
/// and library footprint (`library_tracks`), assembling their songs into a bundled catalog.
pub fn find_favorite_artist(
    recent_tracks: &[TrackEntry],
    library_tracks: &[TrackEntry],
) -> Option<FavoriteArtistCatalog> {
    if library_tracks.is_empty() && recent_tracks.is_empty() {
        return None;
    }

    let mut artist_scores: HashMap<String, u32> = HashMap::new();

    // 1. Score from recent plays (high weight for listening affinity)
    for (idx, track) in recent_tracks.iter().enumerate() {
        let artist = clean_primary_artist(&track.artist);
        if is_ignorable_artist(&artist) {
            continue;
        }
        // Recent tracks earlier in the list are played more recently
        let recency_weight = (recent_tracks.len().saturating_sub(idx) as u32) * 5 + 10;
        *artist_scores.entry(artist).or_insert(0) += recency_weight;
    }

    // 2. Score from library presence (baseline affinity)
    for track in library_tracks.iter() {
        let artist = clean_primary_artist(&track.artist);
        if is_ignorable_artist(&artist) {
            continue;
        }
        *artist_scores.entry(artist).or_insert(0) += 2;
    }

    // Select artist with the highest composite score
    let (top_artist, _) = artist_scores.into_iter().max_by_key(|(_, score)| *score)?;

    // Bundle all library tracks by this artist
    let top_lower = top_artist.to_lowercase();
    let mut bundled: Vec<TrackEntry> = library_tracks
        .iter()
        .filter(|t| {
            let t_clean = clean_primary_artist(&t.artist).to_lowercase();
            t_clean == top_lower || t.artist.to_lowercase().contains(&top_lower)
        })
        .cloned()
        .collect();

    // If library tracks was empty or didn't contain all, also check recents
    if bundled.is_empty() {
        for t in recent_tracks {
            let t_clean = clean_primary_artist(&t.artist).to_lowercase();
            if (t_clean == top_lower || t.artist.to_lowercase().contains(&top_lower))
                && !bundled.iter().any(|b| b.path == t.path)
            {
                bundled.push(t.clone());
            }
        }
    }

    if bundled.is_empty() {
        return None;
    }

    // Sort bundled tracks alphabetically by album and title for a coherent catalog flow
    bundled.sort_by(|a, b| {
        a.album
            .cmp(&b.album)
            .then_with(|| a.title.cmp(&b.title))
    });

    let total_duration_ms: u64 = bundled.iter().map(|t| t.duration_ms).sum();
    let total_duration_str = fmt_ms(total_duration_ms);

    // Pick first available local artwork data url
    let fallback_artwork_url = bundled
        .iter()
        .find_map(|t| t.artwork_data_url.clone());

    let rationale =
        "The local algorithm thinks that since you always listen to these songs with this artist, then it bundled these songs into one."
            .to_string();

    Some(FavoriteArtistCatalog {
        artist: top_artist,
        tracks: bundled,
        total_duration_str,
        fallback_artwork_url,
        rationale,
    })
}

fn is_ignorable_artist(artist: &str) -> bool {
    let lower = artist.trim().to_lowercase();
    lower.is_empty()
        || lower == "unknown artist"
        || lower == "<unknown>"
        || lower == "unknown"
        || lower == "various artists"
        || lower == "various"
}

/// Asynchronously fetches the artist portrait image from Wikipedia REST API.
/// If cached, returns immediately. If network fails, falls back gracefully.
pub fn fetch_wikipedia_artist_image(artist: &str) -> Option<String> {
    let clean = clean_primary_artist(artist);
    if clean.is_empty() || is_ignorable_artist(&clean) {
        return None;
    }

    // Check memory cache
    if let Ok(cache) = ARTIST_IMAGE_CACHE.read() {
        if let Some(entry) = cache.get(&clean) {
            return entry.clone();
        }
    }

    // Query Wikipedia REST API in background
    let url = format!(
        "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
        urlencoding(&clean)
    );

    let res = ureq::get(&url)
        .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
        .set("Accept", "application/json")
        .timeout(std::time::Duration::from_millis(1500))
        .call();

    let img_url = match res {
        Ok(response) => {
            if response.status() == 200 {
                if let Ok(val) = response.into_json::<serde_json::Value>() {
                    val.get("thumbnail")
                        .and_then(|t| t.get("source"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            val.get("originalimage")
                                .and_then(|o| o.get("source"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string())
                        })
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(_) => None,
    };

    if let Ok(mut cache) = ARTIST_IMAGE_CACHE.write() {
        cache.insert(clean, img_url.clone());
    }

    img_url
}

fn urlencoding(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('_'),
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

/// Generates procedural SVG vector artwork with dynamic color gradients,
/// angular geometric shards, and the artist profile/cover image at half opacity.
pub fn render_favorite_artist_svg(artist: &str, image_url: Option<&str>) -> Element {
    // Generate deterministic colors from artist name
    let seed = artist.bytes().fold(5381u32, |acc, b| acc.wrapping_mul(33).wrapping_add(b as u32));
    let palette_idx = (seed % 5) as usize;

    let (c1, c2, c3, accent) = match palette_idx {
        0 => ("#06b6d4", "#2dd4bf", "#0f172a", "#38bdf8"), // Mint / Cyan
        1 => ("#ec4899", "#8b5cf6", "#1e1b4b", "#f43f5e"), // Neon Rose / Violet
        2 => ("#f97316", "#e11d48", "#311042", "#fbbf24"), // Sunset Amber / Crimson
        3 => ("#10b981", "#0284c7", "#064e3b", "#34d399"), // Emerald / Sky
        _ => ("#8b5cf6", "#3b82f6", "#172554", "#c084fc"), // Electric Purple / Indigo
    };

    let initial = artist
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "A".to_string());

    rsx! {
        svg {
            view_box: "0 0 240 240",
            style: "width: 100%; height: 100%; display: block; border-radius: 12px; overflow: hidden;",
            defs {
                linearGradient { id: "bg-grad-{palette_idx}", x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                    stop { offset: "0%", stop_color: "{c1}" }
                    stop { offset: "50%", stop_color: "{c2}" }
                    stop { offset: "100%", stop_color: "{c3}" }
                }
                linearGradient { id: "shard-grad-{palette_idx}", x1: "100%", y1: "0%", x2: "0%", y2: "100%",
                    stop { offset: "0%", stop_color: "{accent}", stop_opacity: "0.45" }
                    stop { offset: "100%", stop_color: "{c1}", stop_opacity: "0.10" }
                }
                radialGradient { id: "glow-radial-{palette_idx}", cx: "50%", cy: "35%", r: "65%",
                    stop { offset: "0%", stop_color: "{accent}", stop_opacity: "0.55" }
                    stop { offset: "70%", stop_color: "{c2}", stop_opacity: "0.20" }
                    stop { offset: "100%", stop_color: "{c3}", stop_opacity: "0.0" }
                }
                linearGradient { id: "vignette-scrim", x1: "0%", y1: "0%", x2: "0%", y2: "100%",
                    stop { offset: "0%", stop_color: "#000000", stop_opacity: "0.10" }
                    stop { offset: "60%", stop_color: "#000000", stop_opacity: "0.35" }
                    stop { offset: "100%", stop_color: "#000000", stop_opacity: "0.75" }
                }
            }

            // Layer 1: Procedural Dynamic Geometric & Gradient Mesh
            rect { width: "100%", height: "100%", fill: "url(#bg-grad-{palette_idx})" }
            circle { cx: "120", cy: "80", r: "100", fill: "url(#glow-radial-{palette_idx})" }

            // Angular geometric shards (Apple Music / Spotify editorial polygon style)
            polygon {
                points: "0,0 240,60 180,240 0,160",
                fill: "url(#shard-grad-{palette_idx})"
            }
            polygon {
                points: "60,0 240,0 240,180 120,240",
                fill: "rgba(255, 255, 255, 0.08)"
            }
            polygon {
                points: "0,140 160,240 0,240",
                fill: "rgba(0, 0, 0, 0.25)"
            }

            // Layer 2: Artist Profile / Album Cover Image at HALF OPACITY (0.5)
            if let Some(img_src) = image_url {
                image {
                    href: "{img_src}",
                    x: "0",
                    y: "0",
                    width: "100%",
                    height: "100%",
                    preserve_aspect_ratio: "xMidYMid slice",
                    opacity: "0.50",
                }
            } else {
                // Typographic Monogram when image is pending or unavailable
                circle { cx: "120", cy: "120", r: "46", fill: "rgba(255, 255, 255, 0.12)", stroke: "rgba(255, 255, 255, 0.25)", stroke_width: "1.5" }
                text {
                    x: "120",
                    y: "135",
                    text_anchor: "middle",
                    font_size: "44",
                    font_weight: "700",
                    fill: "#ffffff",
                    opacity: "0.75",
                    font_family: "system-ui, sans-serif",
                    "{initial}"
                }
            }

            // Layer 3: Protective Liquid Glass Vignette Scrim
            rect { width: "100%", height: "100%", fill: "url(#vignette-scrim)" }

            // Subtle top-right music badge
            circle { cx: "216", cy: "24", r: "12", fill: "rgba(255, 255, 255, 0.18)" }
            text {
                x: "216",
                y: "28",
                text_anchor: "middle",
                font_size: "12",
                fill: "#ffffff",
                font_weight: "bold",
                "✦"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_track(title: &str, artist: &str, album: &str, dur_ms: u64) -> TrackEntry {
        TrackEntry {
            path: PathBuf::from(format!("/music/{title}.flac")),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            genre: Some("Pop".into()),
            duration_ms: dur_ms,
            bpm: Some(128.0),
            key: Some("8B".into()),
            format: "FLAC".into(),
            sample_rate: 44100,
            bit_depth: 16,
            artwork_path: None,
            artwork_data_url: None,
            aggression: None,
            vibe_family: None,
        }
    }

    #[test]
    fn test_clean_primary_artist() {
        assert_eq!(clean_primary_artist("DECO*27 feat. Hatsune Miku"), "DECO*27");
        assert_eq!(clean_primary_artist("OSTER project ft. KAITO"), "OSTER project");
        assert_eq!(clean_primary_artist("PinocchioP, Hatsune Miku"), "PinocchioP");
        assert_eq!(clean_primary_artist("ABBA"), "ABBA");
    }

    #[test]
    fn test_find_favorite_artist_from_recents() {
        let recents = vec![
            make_test_track("Song 1", "DECO*27", "Album A", 180000),
            make_test_track("Song 2", "DECO*27", "Album B", 200000),
            make_test_track("Song 3", "Other Artist", "Album C", 150000),
        ];

        let library = vec![
            make_test_track("Song 1", "DECO*27", "Album A", 180000),
            make_test_track("Song 2", "DECO*27", "Album B", 200000),
            make_test_track("Song 4", "DECO*27", "Album B", 210000),
            make_test_track("Song 3", "Other Artist", "Album C", 150000),
        ];

        let fav = find_favorite_artist(&recents, &library).expect("should find favorite artist");
        assert_eq!(fav.artist, "DECO*27");
        assert_eq!(fav.tracks.len(), 3);
        assert!(fav.rationale.contains("The local algorithm thinks that since you always listen to these songs with this artist, then it bundled these songs into one."));
    }

    #[test]
    fn test_find_favorite_artist_fallback_to_library_when_no_recents() {
        let recents = vec![];
        let library = vec![
            make_test_track("Track 1", "Miku", "Album 1", 120000),
            make_test_track("Track 2", "Miku", "Album 2", 130000),
            make_test_track("Track 3", "Solo", "Album 3", 140000),
        ];

        let fav = find_favorite_artist(&recents, &library).expect("should resolve library most frequent");
        assert_eq!(fav.artist, "Miku");
        assert_eq!(fav.tracks.len(), 2);
    }
}
