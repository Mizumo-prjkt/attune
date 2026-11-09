use dioxus::prelude::*;
use crate::library::{fmt_ms, TrackEntry};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, RwLock};

/// Monotonically increasing revision counter bumped whenever an artist profile is updated or saved.
pub static ARTIST_PROFILE_REVISION: AtomicU64 = AtomicU64::new(0);

pub fn bump_artist_profile_revision() {
    ARTIST_PROFILE_REVISION.fetch_add(1, Ordering::SeqCst);
}

pub fn artist_profile_revision() -> u64 {
    ARTIST_PROFILE_REVISION.load(Ordering::SeqCst)
}

/// Cached artist full profile info from Wikipedia REST API / local resolution.
static ARTIST_PROFILE_CACHE: LazyLock<RwLock<HashMap<String, OnlineArtistInfo>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OnlineArtistInfo {
    pub name: Option<String>,
    pub description: Option<String>,
    pub bio: Option<String>,
    pub image_url: Option<String>,
    pub banner_url: Option<String>,
    pub subscribers: Option<String>,
    pub views: Option<String>,
    pub browse_id: Option<String>,
}

impl OnlineArtistInfo {
    /// Returns true if this artist has a verified online profile resolved
    /// from YouTube Music Innertube or Wikipedia (e.g. browse ID, subscriber metrics,
    /// widescreen banner, curated bio/description, or verified online portrait).
    pub fn is_verified(&self) -> bool {
        self.browse_id.is_some()
            || self.subscribers.is_some()
            || self.views.is_some()
            || self.banner_url.is_some()
            || self.bio.is_some()
            || self.description.is_some()
            || self.image_url.is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistSummary {
    pub name: String,
    pub track_count: usize,
    pub album_count: usize,
    pub sample_track: TrackEntry,
    pub fallback_artwork_url: Option<String>,
    pub photo_url: Option<String>,
    pub is_verified: bool,
}

/// Validates an artist profile image URL, preventing empty strings, placeholders,
/// or bogus test URLs like example.com from poisoning disk or memory caches.
pub fn is_valid_profile_image_url(url: &str) -> bool {
    let u = url.trim();
    if u.is_empty() || u.contains("example.com") {
        return false;
    }
    if u.starts_with("http://") || u.starts_with("https://") || u.starts_with("data:image/") || u.starts_with("file://") {
        return true;
    }
    // Accept valid local image file paths
    let p = std::path::Path::new(u);
    p.is_file()
}

/// Unicode character altmap and normalization for artist names.
/// Maps fullwidth characters, variant asterisks, Japanese / CJK punctuation, dashes, tildes,
/// and spaces to clean standard representations, preventing broken lookups and split artist cards.
pub fn normalize_artist_characters(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut prev_is_space = false;

    for ch in raw.chars() {
        let mapped = match ch {
            // Fullwidth & variant asterisks -> '*'
            '＊' | '﹡' | '∗' | '✱' | '✲' | '✳' => Some('*'),

            // Fullwidth Latin uppercase -> ASCII uppercase
            'Ａ'..='Ｚ' => {
                let ascii = ((ch as u32 - 'Ａ' as u32) + 'A' as u32) as u8 as char;
                Some(ascii)
            }
            // Fullwidth Latin lowercase -> ASCII lowercase
            'ａ'..='ｚ' => {
                let ascii = ((ch as u32 - 'ａ' as u32) + 'a' as u32) as u8 as char;
                Some(ascii)
            }
            // Fullwidth digits -> ASCII digits
            '０'..='９' => {
                let ascii = ((ch as u32 - '０' as u32) + '0' as u32) as u8 as char;
                Some(ascii)
            }

            // Hyphens, dashes, minus signs -> '-'
            '－' | '‐' | '‑' | '–' | '—' | '―' | '−' => Some('-'),

            // Tildes & wave dashes -> '~'
            '～' | '〜' => Some('~'),

            // Slashes -> '/' and '\'
            '／' => Some('/'),
            '＼' => Some('\\'),

            // Colons & Semicolons
            '：' => Some(':'),
            '；' => Some(';'),

            // Exclamation & Question marks
            '！' => Some('!'),
            '？' => Some('?'),

            // Commercial at
            '＠' => Some('@'),

            // Ampersand
            '＆' => Some('&'),

            // Apostrophes & Single quotes -> '\''
            '’' | '‘' | '＇' | 'ʼ' | '｀' | '′' => Some('\''),

            // Double quotes -> '"'
            '”' | '“' | '＂' => Some('"'),

            // Brackets & Parentheses
            '（' => Some('('),
            '）' => Some(')'),
            '［' => Some('['),
            '］' => Some(']'),
            '｛' => Some('{'),
            '｝' => Some('}'),

            // Ideographic and non-breaking spaces -> ' '
            '　' | '\u{00A0}' => Some(' '),

            // Zero-width characters and BOM -> skip
            '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' => None,

            // Keep everything else as is
            other => Some(other),
        };

        if let Some(c) = mapped {
            if c.is_whitespace() {
                if !prev_is_space && !result.is_empty() {
                    result.push(' ');
                    prev_is_space = true;
                }
            } else {
                result.push(c);
                prev_is_space = false;
            }
        }
    }

    result.trim().to_string()
}

/// Generates a normalized, case-folded canonical lookup key for artist matching and deduplication.
pub fn canonical_artist_key(name: &str) -> String {
    normalize_artist_characters(name).to_lowercase()
}

/// Returns the cache file path for an artist profile (~/.cache/attune/artists/{safe_name}.json).
pub fn artist_cache_file_path(artist_name: &str) -> Option<std::path::PathBuf> {
    let clean = clean_primary_artist(artist_name);
    if clean.is_empty() {
        return None;
    }
    let cache_dir = dirs::cache_dir()?.join("attune").join("artists");
    let _ = std::fs::create_dir_all(&cache_dir);
    let safe_name: String = clean
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    Some(cache_dir.join(format!("{safe_name}.json")))
}

/// Loads a cached artist profile from memory or disk cache.
pub fn load_cached_artist_profile(artist_name: &str) -> Option<OnlineArtistInfo> {
    let key = canonical_artist_key(artist_name);
    if key.is_empty() {
        return None;
    }

    // 1. Memory cache (indexed canonically so case/accents/alt-characters hit consistently)
    if let Ok(cache) = ARTIST_PROFILE_CACHE.read() {
        if let Some(entry) = cache.get(&key) {
            let mut info = entry.clone();
            if let Some(ref img) = info.image_url {
                if !is_valid_profile_image_url(img) {
                    info.image_url = None;
                }
            }
            return Some(info);
        }
        let clean_key = clean_primary_artist(artist_name).to_lowercase();
        if !clean_key.is_empty() {
            if let Some(entry) = cache.get(&clean_key) {
                let mut info = entry.clone();
                if let Some(ref img) = info.image_url {
                    if !is_valid_profile_image_url(img) {
                        info.image_url = None;
                    }
                }
                return Some(info);
            }
        }
    }

    // 2. Disk cache
    let clean = clean_primary_artist(artist_name);
    if let Some(path) = artist_cache_file_path(&clean) {
        if path.is_file() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(mut info) = serde_json::from_str::<OnlineArtistInfo>(&data) {
                    if let Some(ref img) = info.image_url {
                        if !is_valid_profile_image_url(img) {
                            info.image_url = None;
                        }
                    }
                    if let Ok(mut cache) = ARTIST_PROFILE_CACHE.write() {
                        cache.insert(key, info.clone());
                    }
                    return Some(info);
                }
            }
        }
    }

    None
}

/// Persists an artist profile to both memory and disk cache.
pub fn save_cached_artist_profile(artist_name: &str, info: &OnlineArtistInfo) {
    let key = canonical_artist_key(artist_name);
    if key.is_empty() {
        return;
    }

    let mut sanitized = info.clone();

    // Ensure manual or custom linked profiles carry verified status
    if sanitized.browse_id.is_none() {
        sanitized.browse_id = Some("custom-manual-linked".to_string());
    }

    // If image_url points to a local file, downscale/compress it to cache and store as Data URL
    if let Some(ref img) = sanitized.image_url {
        let trimmed = img.trim();
        let local_path = if trimmed.starts_with("file://") {
            Some(std::path::PathBuf::from(&trimmed[7..]))
        } else if std::path::Path::new(trimmed).is_file() {
            Some(std::path::PathBuf::from(trimmed))
        } else {
            None
        };

        if let Some(src_p) = local_path {
            let clean = clean_primary_artist(artist_name);
            let safe_name: String = clean
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
                .collect();
            let img_dir = dirs::cache_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                .join("attune")
                .join("artists")
                .join("images");
            let dest_img = img_dir.join(format!("{safe_name}.jpg"));
            let max_res = crate::config::load_config().effective_artwork_resolution();

            if crate::library::save_compressed_image(&src_p, &dest_img, max_res).is_ok() {
                if let Ok(bytes) = std::fs::read(&dest_img) {
                    if let Some(data_url) = crate::library::compress_image_to_data_url(&bytes, max_res) {
                        sanitized.image_url = Some(data_url);
                    } else {
                        sanitized.image_url = Some(format!("file://{}", dest_img.to_string_lossy()));
                    }
                } else {
                    sanitized.image_url = Some(format!("file://{}", dest_img.to_string_lossy()));
                }
            }
        } else if !is_valid_profile_image_url(trimmed) {
            sanitized.image_url = None;
        }
    }

    let clean_key = clean_primary_artist(artist_name).to_lowercase();
    if let Ok(mut cache) = ARTIST_PROFILE_CACHE.write() {
        cache.insert(key, sanitized.clone());
        if !clean_key.is_empty() {
            cache.insert(clean_key, sanitized.clone());
        }
    }

    let clean = clean_primary_artist(artist_name);
    if let Some(path) = artist_cache_file_path(&clean) {
        if let Ok(json_str) = serde_json::to_string_pretty(&sanitized) {
            let _ = std::fs::write(path, json_str);
        }
    }

    bump_artist_profile_revision();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArtistCandidate {
    pub name: String,
    pub provider: String,
    pub description: Option<String>,
    pub bio: Option<String>,
    pub image_url: Option<String>,
    pub banner_url: Option<String>,
    pub subscribers: Option<String>,
    pub views: Option<String>,
    pub browse_id: Option<String>,
}

impl ArtistCandidate {
    pub fn to_online_info(&self) -> OnlineArtistInfo {
        OnlineArtistInfo {
            name: Some(self.name.clone()),
            description: self.description.clone(),
            bio: self.bio.clone(),
            image_url: self.image_url.clone(),
            banner_url: self.banner_url.clone(),
            subscribers: self.subscribers.clone(),
            views: self.views.clone(),
            browse_id: self.browse_id.clone(),
        }
    }
}

/// Queries online providers (YouTube Music Innertube, Wikipedia EN, Wikipedia JA) for artist profile candidates matching a search query.
pub fn search_online_artist_candidates(query: &str) -> Vec<ArtistCandidate> {
    let clean = clean_primary_artist(query);
    if clean.trim().is_empty() {
        return Vec::new();
    }

    let mut candidates: Vec<ArtistCandidate> = Vec::new();

    // 1. YouTube Music Innertube direct query
    let yt_info = query_innertube_artist_profile(&clean);
    if yt_info.is_verified() || yt_info.name.is_some() || yt_info.image_url.is_some() {
        candidates.push(ArtistCandidate {
            name: yt_info.name.clone().unwrap_or_else(|| clean.clone()),
            provider: "YouTube Music Innertube".to_string(),
            description: yt_info.description.clone(),
            bio: yt_info.bio.clone(),
            image_url: yt_info.image_url.clone(),
            banner_url: yt_info.banner_url.clone(),
            subscribers: yt_info.subscribers.clone(),
            views: yt_info.views.clone(),
            browse_id: yt_info.browse_id.clone(),
        });
    }

    // 2. English Wikipedia OpenSearch
    let en_encoded = urlencoding(&clean);
    let en_search_url = format!(
        "https://en.wikipedia.org/w/api.php?action=opensearch&search={}&limit=4&namespace=0&format=json",
        en_encoded
    );
    if let Ok(resp) = ureq::get(&en_search_url)
        .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
        .set("Accept", "application/json")
        .timeout(std::time::Duration::from_millis(3000))
        .call()
    {
        if let Ok(arr) = resp.into_json::<serde_json::Value>() {
            if let Some(titles) = arr.get(1).and_then(|v| v.as_array()) {
                for title_val in titles {
                    if let Some(title) = title_val.as_str() {
                        let sum_url = format!(
                            "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
                            urlencoding(title)
                        );
                        let wiki_info = query_wikipedia_summary(&sum_url);
                        if wiki_info.description.is_some() || wiki_info.bio.is_some() || wiki_info.image_url.is_some() {
                            let already_exists = candidates.iter().any(|c| c.name.eq_ignore_ascii_case(title));
                            if !already_exists {
                                candidates.push(ArtistCandidate {
                                    name: title.to_string(),
                                    provider: "Wikipedia (English)".to_string(),
                                    description: wiki_info.description,
                                    bio: wiki_info.bio,
                                    image_url: wiki_info.image_url,
                                    banner_url: None,
                                    subscribers: None,
                                    views: None,
                                    browse_id: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Japanese Wikipedia OpenSearch (crucial for Vocaloid / J-Pop / Anime artists)
    let ja_search_url = format!(
        "https://ja.wikipedia.org/w/api.php?action=opensearch&search={}&limit=4&namespace=0&format=json",
        en_encoded
    );
    if let Ok(resp) = ureq::get(&ja_search_url)
        .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
        .set("Accept", "application/json")
        .timeout(std::time::Duration::from_millis(3000))
        .call()
    {
        if let Ok(arr) = resp.into_json::<serde_json::Value>() {
            if let Some(titles) = arr.get(1).and_then(|v| v.as_array()) {
                for title_val in titles {
                    if let Some(title) = title_val.as_str() {
                        let sum_url = format!(
                            "https://ja.wikipedia.org/api/rest_v1/page/summary/{}",
                            urlencoding(title)
                        );
                        let wiki_info = query_wikipedia_summary(&sum_url);
                        if wiki_info.description.is_some() || wiki_info.bio.is_some() || wiki_info.image_url.is_some() {
                            let already_exists = candidates.iter().any(|c| c.name == title);
                            if !already_exists {
                                candidates.push(ArtistCandidate {
                                    name: title.to_string(),
                                    provider: "Wikipedia (Japanese)".to_string(),
                                    description: wiki_info.description,
                                    bio: wiki_info.bio,
                                    image_url: wiki_info.image_url,
                                    banner_url: None,
                                    subscribers: None,
                                    views: None,
                                    browse_id: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Musixmatch v1.1 `artist.search` (when enabled in Settings with a valid API key)
    let cfg = crate::config::load_config();
    if cfg.api_musixmatch_enabled && !cfg.musixmatch_api_key.trim().is_empty() {
        let mxm_url = format!(
            "https://api.musixmatch.com/ws/1.1/artist.search?q_artist={}&page_size=4&apikey={}",
            en_encoded,
            urlencoding(cfg.musixmatch_api_key.trim())
        );
        if let Ok(resp) = ureq::get(&mxm_url)
            .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
            .set("Accept", "application/json")
            .timeout(std::time::Duration::from_millis(3500))
            .call()
        {
            if let Ok(json) = resp.into_json::<serde_json::Value>() {
                for mxm_cand in parse_musixmatch_artist_candidates(&json) {
                    candidates.push(mxm_cand);
                }
            }
        }
    }

    candidates
}

/// Parses a Musixmatch v1.1 `artist.search` JSON response (`message.body.artist_list`)
/// into `ArtistCandidate` entries, extracting `artist_comment`, `artist_country`, `artist_rating`,
/// `artist_alias_list`, and `primary_genres` per the Musixmatch API specification.
pub fn parse_musixmatch_artist_candidates(json: &serde_json::Value) -> Vec<ArtistCandidate> {
    let mut out = Vec::new();
    let status = json
        .get("message")
        .and_then(|m| m.get("header"))
        .and_then(|h| h.get("status_code"))
        .and_then(|c| c.as_i64())
        .unwrap_or(0);
    if status != 200 {
        return out;
    }

    let Some(artist_list) = json
        .get("message")
        .and_then(|m| m.get("body"))
        .and_then(|b| b.get("artist_list"))
        .and_then(|al| al.as_array())
    else {
        return out;
    };

    for item in artist_list.iter().take(4) {
        let Some(artist) = item.get("artist") else {
            continue;
        };
        let artist_id = artist.get("artist_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let name = artist
            .get("artist_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }

        let comment = artist
            .get("artist_comment")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let country = artist
            .get("artist_country")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let rating = artist.get("artist_rating").and_then(|v| v.as_i64());

        let mut genres = Vec::new();
        if let Some(g_list) = artist
            .get("primary_genres")
            .and_then(|pg| pg.get("music_genre_list"))
            .and_then(|gl| gl.as_array())
        {
            for g_item in g_list {
                if let Some(g_name) = g_item
                    .get("music_genre")
                    .and_then(|mg| mg.get("music_genre_name_extended").or_else(|| mg.get("music_genre_name")))
                    .and_then(|v| v.as_str())
                {
                    let trimmed = g_name.trim();
                    if !trimmed.is_empty() && !genres.contains(&trimmed.to_string()) {
                        genres.push(trimmed.to_string());
                    }
                }
            }
        }

        let mut aliases = Vec::new();
        if let Some(a_list) = artist.get("artist_alias_list").and_then(|al| al.as_array()) {
            for a_item in a_list {
                if let Some(alias) = a_item.get("artist_alias").and_then(|v| v.as_str()) {
                    let trimmed = alias.trim();
                    if !trimmed.is_empty() && !aliases.contains(&trimmed.to_string()) {
                        aliases.push(trimmed.to_string());
                    }
                }
            }
        }

        let mut desc_parts = Vec::new();
        if let Some(c) = comment {
            desc_parts.push(c.to_string());
        }
        if let Some(cty) = country {
            desc_parts.push(format!("Country: {cty}"));
        }
        if !genres.is_empty() {
            desc_parts.push(genres.join(", "));
        }
        let description = if desc_parts.is_empty() {
            Some("Musixmatch Catalog Artist".to_string())
        } else {
            Some(desc_parts.join(" • "))
        };

        let mut bio_lines = Vec::new();
        if let Some(c) = comment {
            bio_lines.push(format!("{name} ({c})."));
        } else {
            bio_lines.push(format!("{name} is an indexed artist in the Musixmatch global catalog."));
        }
        if let Some(cty) = country {
            bio_lines.push(format!("Origin country: {cty}."));
        }
        if !genres.is_empty() {
            bio_lines.push(format!("Primary genres: {}.", genres.join(", ")));
        }
        if !aliases.is_empty() {
            bio_lines.push(format!("Also known as: {}.", aliases.join(", ")));
        }

        out.push(ArtistCandidate {
            name,
            provider: "Musixmatch".to_string(),
            description,
            bio: Some(bio_lines.join(" ")),
            image_url: None,
            banner_url: None,
            subscribers: rating.map(|r| format!("Rating {r}/100")),
            views: None,
            browse_id: if artist_id > 0 {
                Some(format!("mxm-artist-{artist_id}"))
            } else {
                None
            },
        });
    }

    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct FavoriteArtistCatalog {
    pub artist: String,
    pub tracks: Vec<TrackEntry>,
    pub total_duration_str: String,
    pub fallback_artwork_url: Option<String>,
    pub rationale: String,
}

/// Helper checking for opening parenthesis or bracket (supporting half-width and full-width/CJK characters)
fn is_open_bracket(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{' | '（' | '【' | '〔' | '［' | '｛')
}

/// Helper checking for closing parenthesis or bracket (supporting half-width and full-width/CJK characters)
fn is_close_bracket(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}' | '）' | '】' | '〕' | '］' | '｝')
}

/// Splits an artist string into distinct individual artists, recognizing
/// delimiters such as ',', ';', '/', '&', '&amp;', 'x', 'vs.', 'feat.', 'ft.', 'featuring', 'with',
/// and Japanese / East Asian punctuation: '＆', '，', '、', '・', '×', '✕', '／', '；', '＋'.
pub fn split_artist_string(raw: &str) -> Vec<String> {
    let normalized = normalize_artist_characters(raw);
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // 1. Separate bracketed/parenthetical feature clauses (e.g. "(feat. Kasane Teto & Ui)")
    let mut main_part = String::new();
    let mut extra_clauses = Vec::new();
    let mut in_paren = false;
    let mut current_paren = String::new();

    for ch in trimmed.chars() {
        if is_open_bracket(ch) {
            in_paren = true;
            current_paren.clear();
        } else if is_close_bracket(ch) {
            if in_paren {
                in_paren = false;
                let paren_lower = current_paren.trim().to_lowercase();
                if paren_lower.starts_with("feat.")
                    || paren_lower.starts_with("feat ")
                    || paren_lower.starts_with("ft.")
                    || paren_lower.starts_with("ft ")
                    || paren_lower.starts_with("featuring ")
                    || paren_lower.starts_with("with ")
                {
                    extra_clauses.push(current_paren.clone());
                } else {
                    main_part.push(' ');
                    main_part.push_str(&current_paren);
                }
            }
        } else if in_paren {
            current_paren.push(ch);
        } else {
            main_part.push(ch);
        }
    }

    let mut text_to_split = main_part;
    for clause in extra_clauses {
        text_to_split.push(' ');
        text_to_split.push_str(&clause);
    }

    // 2. Replace collaboration delimiters with a sentinel unit separator \x1f
    let chars: Vec<char> = text_to_split.chars().collect();
    let lower_chars: Vec<char> = text_to_split.to_lowercase().chars().collect();
    let len = chars.len();

    let mut output = String::new();
    let mut i = 0;

    while i < len {
        let remaining_lower: String = lower_chars[i..].iter().collect();

        if remaining_lower.starts_with(" featuring ") {
            output.push('\x1f');
            i += 11;
        } else if remaining_lower.starts_with(" feat. ") {
            output.push('\x1f');
            i += 7;
        } else if remaining_lower.starts_with(" feat.") {
            output.push('\x1f');
            i += 6;
        } else if remaining_lower.starts_with(" feat ") {
            output.push('\x1f');
            i += 6;
        } else if remaining_lower.starts_with(" ft. ") {
            output.push('\x1f');
            i += 5;
        } else if remaining_lower.starts_with(" ft.") {
            output.push('\x1f');
            i += 4;
        } else if remaining_lower.starts_with(" ft ") {
            output.push('\x1f');
            i += 4;
        } else if (i == 0 || chars[i - 1] == ' ' || chars[i - 1] == '\x1f' || !chars[i - 1].is_alphabetic())
            && (remaining_lower.starts_with("feat.") || remaining_lower.starts_with("ft."))
        {
            let adv = if remaining_lower.starts_with("feat.") { 5 } else { 3 };
            output.push('\x1f');
            i += adv;
        } else if remaining_lower.starts_with(" with ") {
            output.push('\x1f');
            i += 6;
        } else if remaining_lower.starts_with(" &amp; ") {
            output.push('\x1f');
            i += 7;
        } else if remaining_lower.starts_with(" & ") || remaining_lower.starts_with(" ＆ ") {
            output.push('\x1f');
            i += 3;
        } else if remaining_lower.starts_with(" x ")
            || remaining_lower.starts_with(" × ")
            || remaining_lower.starts_with(" ✕ ")
            || remaining_lower.starts_with(" vs ")
            || remaining_lower.starts_with(" + ")
            || remaining_lower.starts_with(" ＋ ")
        {
            output.push('\x1f');
            i += 3;
        } else if remaining_lower.starts_with(" vs. ") {
            output.push('\x1f');
            i += 5;
        } else if chars[i] == ','
            || chars[i] == ';'
            || chars[i] == '/'
            || chars[i] == '，'
            || chars[i] == '、'
            || chars[i] == '；'
            || chars[i] == '／'
            || chars[i] == '・'
        {
            output.push('\x1f');
            i += 1;
        } else if chars[i] == '&' || chars[i] == '＆' || chars[i] == '×' || chars[i] == '✕' {
            output.push('\x1f');
            i += 1;
        } else {
            output.push(chars[i]);
            i += 1;
        }
    }

    let mut results = Vec::new();
    for token in output.split('\x1f') {
        let mut t = token.trim();
        let t_lower = t.to_lowercase();
        if let Some(rest) = t_lower.strip_prefix("feat.") {
            t = t[t.len() - rest.len()..].trim();
        } else if let Some(rest) = t_lower.strip_prefix("feat ") {
            t = t[t.len() - rest.len()..].trim();
        } else if let Some(rest) = t_lower.strip_prefix("ft.") {
            t = t[t.len() - rest.len()..].trim();
        } else if let Some(rest) = t_lower.strip_prefix("ft ") {
            t = t[t.len() - rest.len()..].trim();
        } else if let Some(rest) = t_lower.strip_prefix("featuring ") {
            t = t[t.len() - rest.len()..].trim();
        } else if let Some(rest) = t_lower.strip_prefix("with ") {
            t = t[t.len() - rest.len()..].trim();
        }

        let cleaned = normalize_artist_characters(t.trim());
        if !cleaned.is_empty() && !is_ignorable_artist(&cleaned) {
            results.push(cleaned);
        }
    }

    results
}

/// Extracts featured artists from track titles, such as:
/// "Teto the 31st (feat. KASANE TETO & Ui)" -> ["KASANE TETO", "Ui"]
pub fn extract_featured_artists_from_title(title: &str) -> Vec<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut found_artists = Vec::new();

    // 1. Look for (feat. ...) or [ft. ...]
    let mut in_paren = false;
    let mut current_paren = String::new();

    for ch in trimmed.chars() {
        if is_open_bracket(ch) {
            in_paren = true;
            current_paren.clear();
        } else if is_close_bracket(ch) {
            if in_paren {
                in_paren = false;
                let paren_lower = current_paren.trim().to_lowercase();
                let feat_prefix = if paren_lower.starts_with("feat.") {
                    Some(5)
                } else if paren_lower.starts_with("feat ") {
                    Some(5)
                } else if paren_lower.starts_with("ft.") {
                    Some(3)
                } else if paren_lower.starts_with("ft ") {
                    Some(3)
                } else if paren_lower.starts_with("featuring ") {
                    Some(10)
                } else if paren_lower.starts_with("with ") {
                    Some(5)
                } else {
                    None
                };

                if let Some(prefix_len) = feat_prefix {
                    let artists_slice = &current_paren.trim()[prefix_len..];
                    for a in split_artist_string(artists_slice) {
                        found_artists.push(a);
                    }
                }
            }
        } else if in_paren {
            current_paren.push(ch);
        }
    }

    // 2. Also check if title has non-parenthesized " feat. " or " ft. "
    if found_artists.is_empty() {
        let lower = trimmed.to_lowercase();
        let feat_cut = if let Some(pos) = lower.find(" feat. ") {
            Some(pos + 7)
        } else if let Some(pos) = lower.find(" feat.") {
            Some(pos + 6)
        } else if let Some(pos) = lower.find(" feat ") {
            Some(pos + 6)
        } else if let Some(pos) = lower.find(" ft. ") {
            Some(pos + 5)
        } else if let Some(pos) = lower.find(" ft.") {
            Some(pos + 4)
        } else if let Some(pos) = lower.find(" ft ") {
            Some(pos + 4)
        } else if let Some(pos) = lower.find(" featuring ") {
            Some(pos + 11)
        } else {
            None
        };

        if let Some(start_idx) = feat_cut {
            let slice = &trimmed[start_idx..];
            for a in split_artist_string(slice) {
                found_artists.push(a);
            }
        }
    }

    found_artists
}

/// Returns true if `new_str` has preferable casing over `old_str`
/// (e.g. "Kasane Teto" over "KASANE TETO" or "kasane teto").
pub fn should_prefer_casing(new_str: &str, old_str: &str) -> bool {
    let new_is_all_upper = new_str.chars().any(|c| c.is_alphabetic())
        && new_str.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
    let old_is_all_upper = old_str.chars().any(|c| c.is_alphabetic())
        && old_str.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());

    let new_is_all_lower = new_str.chars().any(|c| c.is_alphabetic())
        && new_str.chars().all(|c| !c.is_alphabetic() || c.is_lowercase());
    let old_is_all_lower = old_str.chars().any(|c| c.is_alphabetic())
        && old_str.chars().all(|c| !c.is_alphabetic() || c.is_lowercase());

    // Prefer mixed/title case over ALL-CAPS
    if old_is_all_upper && !new_is_all_upper {
        return true;
    }
    // Prefer capitalized over all-lowercase
    if old_is_all_lower && !new_is_all_lower {
        return true;
    }
    false
}

/// Extracts all pinpointed artists from a track, combining artist field,
/// title featured clauses, and vocalist metadata with case-insensitive deduplication.
pub fn extract_all_artists_from_track(track: &TrackEntry) -> Vec<String> {
    let mut candidates = Vec::new();

    // From track.artist
    for a in split_artist_string(&track.artist) {
        candidates.push(a);
    }

    // From track.title
    for a in extract_featured_artists_from_title(&track.title) {
        candidates.push(a);
    }

    // From track.vocalist
    if let Some(ref voc) = track.vocalist {
        for a in split_artist_string(voc) {
            candidates.push(a);
        }
    }

    // Deduplicate case-insensitively with character altmap normalization, preferring mixed/title-casing over ALL-CAPS or all-lowercase
    let mut deduplicated: Vec<String> = Vec::new();

    for candidate in candidates {
        let norm_cand = normalize_artist_characters(&candidate);
        if norm_cand.is_empty() || is_ignorable_artist(&norm_cand) {
            continue;
        }
        let cand_key = canonical_artist_key(&norm_cand);
        if let Some(pos) = deduplicated.iter().position(|existing| canonical_artist_key(existing) == cand_key) {
            if should_prefer_casing(&norm_cand, &deduplicated[pos]) {
                deduplicated[pos] = norm_cand;
            }
        } else {
            deduplicated.push(norm_cand);
        }
    }

    deduplicated
}

/// Checks if a track features or is by a given artist name using character altmap and case insensitivity.
pub fn track_has_artist(track: &TrackEntry, target_artist: &str) -> bool {
    let target_key = canonical_artist_key(target_artist);
    if target_key.is_empty() {
        return false;
    }
    extract_all_artists_from_track(track)
        .iter()
        .any(|a| canonical_artist_key(a) == target_key)
}

/// Extracts all unique individual artists across all tracks in the library,
/// pinpointing artists using '&', 'feat.', 'ft.', commas, and title featured clauses.
pub fn extract_all_unique_artists(tracks: &[TrackEntry]) -> Vec<ArtistSummary> {
    let offload = crate::config::load_config().gpu_artist_index_offload;
    extract_all_unique_artists_with_offload(tracks, offload)
}

/// Extracts all unique individual artists across all tracks in the library,
/// optionally offloading batch parsing and JSON cache resolution in parallel threads.
pub fn extract_all_unique_artists_with_offload(tracks: &[TrackEntry], parallel_offload: bool) -> Vec<ArtistSummary> {
    use std::collections::HashSet;
    use rayon::prelude::*;

    let mut artist_map: HashMap<String, (Vec<TrackEntry>, HashSet<String>)> = HashMap::new();
    let mut canonical_names: HashMap<String, String> = HashMap::new();

    if parallel_offload && tracks.len() > 16 {
        let chunk_size = (tracks.len() / rayon::current_num_threads()).max(16);
        let partial_maps: Vec<HashMap<String, (String, Vec<TrackEntry>, HashSet<String>)>> = tracks
            .par_chunks(chunk_size)
            .map(|chunk| {
                let mut local_map: HashMap<String, (String, Vec<TrackEntry>, HashSet<String>)> = HashMap::new();
                for track in chunk {
                    let artists = extract_all_artists_from_track(track);
                    for artist in artists {
                        if is_ignorable_artist(&artist) {
                            continue;
                        }
                        let key = canonical_artist_key(&artist);
                        let entry = local_map.entry(key).or_insert_with(|| (artist.clone(), Vec::new(), HashSet::new()));
                        if should_prefer_casing(&artist, &entry.0) {
                            entry.0 = artist.clone();
                        }
                        if !entry.1.iter().any(|t| t.path == track.path) {
                            entry.1.push(track.clone());
                        }
                        entry.2.insert(track.album.clone());
                    }
                }
                local_map
            })
            .collect();

        for partial in partial_maps {
            for (key, (c_name, p_tracks, p_albums)) in partial {
                if let Some(existing) = canonical_names.get_mut(&key) {
                    if should_prefer_casing(&c_name, existing) {
                        *existing = c_name;
                    }
                } else {
                    canonical_names.insert(key.clone(), c_name);
                }

                let entry = artist_map.entry(key).or_insert_with(|| (Vec::new(), HashSet::new()));
                for t in p_tracks {
                    if !entry.0.iter().any(|exist| exist.path == t.path) {
                        entry.0.push(t);
                    }
                }
                entry.1.extend(p_albums);
            }
        }
    } else {
        for track in tracks {
            let artists = extract_all_artists_from_track(track);
            for artist in artists {
                if is_ignorable_artist(&artist) {
                    continue;
                }

                let key = canonical_artist_key(&artist);
                if let Some(existing) = canonical_names.get_mut(&key) {
                    if should_prefer_casing(&artist, existing) {
                        *existing = artist.clone();
                    }
                } else {
                    canonical_names.insert(key.clone(), artist.clone());
                }

                let entry = artist_map.entry(key).or_insert_with(|| (Vec::new(), HashSet::new()));
                if !entry.0.iter().any(|t| t.path == track.path) {
                    entry.0.push(track.clone());
                }
                entry.1.insert(track.album.clone());
            }
        }
    }

    let artist_entries: Vec<(String, (Vec<TrackEntry>, HashSet<String>))> = artist_map.into_iter().collect();

    let mut list: Vec<ArtistSummary> = if parallel_offload && artist_entries.len() > 4 {
        artist_entries
            .into_par_iter()
            .map(|(key, (artist_tracks, albums))| {
                let name = canonical_names.get(&key).cloned().unwrap_or(key);
                let fallback_artwork_url = artist_tracks.iter().find_map(|t| t.artwork_data_url.clone());
                let cached_profile = load_cached_artist_profile(&name);
                let is_verified = cached_profile.as_ref().map_or(false, |p| p.is_verified());
                let photo_url = cached_profile.and_then(|p| p.image_url);
                let sample_track = artist_tracks[0].clone();
                ArtistSummary {
                    name,
                    track_count: artist_tracks.len(),
                    album_count: albums.len(),
                    sample_track,
                    fallback_artwork_url,
                    photo_url,
                    is_verified,
                }
            })
            .collect()
    } else {
        artist_entries
            .into_iter()
            .map(|(key, (artist_tracks, albums))| {
                let name = canonical_names.get(&key).cloned().unwrap_or(key);
                let fallback_artwork_url = artist_tracks.iter().find_map(|t| t.artwork_data_url.clone());
                let cached_profile = load_cached_artist_profile(&name);
                let is_verified = cached_profile.as_ref().map_or(false, |p| p.is_verified());
                let photo_url = cached_profile.and_then(|p| p.image_url);
                let sample_track = artist_tracks[0].clone();
                ArtistSummary {
                    name,
                    track_count: artist_tracks.len(),
                    album_count: albums.len(),
                    sample_track,
                    fallback_artwork_url,
                    photo_url,
                    is_verified,
                }
            })
            .collect()
    };

    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    list
}

/// Computes the primary artist name, stripping collaborative '&', 'feat.', or 'ft.' clauses
/// while respecting multi-artist delimiters and character altmap normalization.
pub fn clean_primary_artist(raw_artist: &str) -> String {
    let normalized = normalize_artist_characters(raw_artist);
    let artists = split_artist_string(&normalized);
    if let Some(first) = artists.into_iter().next() {
        first
    } else {
        normalized.trim().to_string()
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
    let mut canonical_names: HashMap<String, String> = HashMap::new();

    // 1. Score from recent plays (high weight for listening affinity across all pinpointed artists)
    for (idx, track) in recent_tracks.iter().enumerate() {
        let artists = extract_all_artists_from_track(track);
        let recency_weight = (recent_tracks.len().saturating_sub(idx) as u32) * 5 + 10;
        for artist in artists {
            if is_ignorable_artist(&artist) {
                continue;
            }
            let key = canonical_artist_key(&artist);
            if let Some(existing) = canonical_names.get_mut(&key) {
                if should_prefer_casing(&artist, existing) {
                    *existing = artist.clone();
                }
            } else {
                canonical_names.insert(key.clone(), artist.clone());
            }
            *artist_scores.entry(key).or_insert(0) += recency_weight;
        }
    }

    // 2. Score from library presence (baseline affinity across all pinpointed artists)
    for track in library_tracks.iter() {
        let artists = extract_all_artists_from_track(track);
        for artist in artists {
            if is_ignorable_artist(&artist) {
                continue;
            }
            let key = canonical_artist_key(&artist);
            if let Some(existing) = canonical_names.get_mut(&key) {
                if should_prefer_casing(&artist, existing) {
                    *existing = artist.clone();
                }
            } else {
                canonical_names.insert(key.clone(), artist.clone());
            }
            *artist_scores.entry(key).or_insert(0) += 2;
        }
    }

    // Select artist with the highest composite score
    let (top_key, _) = artist_scores.into_iter().max_by_key(|(_, score)| *score)?;
    let top_artist = canonical_names.get(&top_key).cloned().unwrap_or(top_key);

    // Bundle all library tracks by this artist using pinpointed track_has_artist
    let mut bundled: Vec<TrackEntry> = library_tracks
        .iter()
        .filter(|t| track_has_artist(t, &top_artist))
        .cloned()
        .collect();

    // If library tracks was empty or didn't contain all, also check recents
    if bundled.is_empty() {
        for t in recent_tracks {
            if track_has_artist(t, &top_artist) && !bundled.iter().any(|b| b.path == t.path) {
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

/// Resizes Google CDN image URL to square dimensions with high quality (e.g. `=w512-h512-p-l90-rj`).
pub fn resize_google_avatar_url(url: &str, size: u32) -> String {
    let target_token = format!("=w{size}-h{size}-p-l90-rj");
    if let Some(idx) = url.find('=') {
        let base = &url[..idx];
        format!("{base}{target_token}")
    } else {
        format!("{url}{target_token}")
    }
}

/// Trims trailing Wikipedia / Creative Commons attribution links from YouTube Music's description shelf.
pub fn clean_innertube_bio(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    if let Some(pos) = text.find("\n\nFrom Wikipedia (") {
        text.truncate(pos);
    } else if let Some(pos) = text.find("\n\nFrom Wikipedia") {
        text.truncate(pos);
    } else if let Some(pos) = text.find("From Wikipedia (") {
        text.truncate(pos);
    }
    text.trim().to_string()
}

/// Queries YouTube Music Innertube API directly (Search + Browse) without OAuth or API keys.
pub fn query_innertube_artist_profile(artist: &str) -> OnlineArtistInfo {
    let clean = clean_primary_artist(artist);
    if clean.is_empty() {
        return OnlineArtistInfo::default();
    }

    // 1. Search endpoint
    let search_payload = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20240101.01.00",
                "hl": "en",
                "gl": "US"
            }
        },
        "query": clean
    });

    let search_res = ureq::post("https://music.youtube.com/youtubei/v1/search")
        .set("Content-Type", "application/json")
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .set("Origin", "https://music.youtube.com")
        .timeout(std::time::Duration::from_millis(3000))
        .send_json(search_payload);

    let mut browse_id: Option<String> = None;
    let mut search_avatar: Option<String> = None;
    let mut canonical_name: Option<String> = None;

    if let Ok(resp) = search_res {
        if let Ok(json) = resp.into_json::<serde_json::Value>() {
            let tabs = json
                .get("contents")
                .and_then(|c| c.get("tabbedSearchResultsRenderer"))
                .and_then(|t| t.get("tabs"))
                .and_then(|t| t.as_array());

            if let Some(first_tab) = tabs.and_then(|t| t.first()).and_then(|t| t.get("tabRenderer")).and_then(|t| t.get("content")).and_then(|c| c.get("sectionListRenderer")).and_then(|s| s.get("contents")).and_then(|c| c.as_array()) {
                // Check musicCardShelfRenderer
                for section in first_tab {
                    if let Some(card) = section.get("musicCardShelfRenderer") {
                        let name = card
                            .get("title")
                            .and_then(|t| t.get("runs"))
                            .and_then(|r| r.as_array())
                            .and_then(|a| a.first())
                            .and_then(|f| f.get("text"))
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string());

                        let id = card
                            .get("title")
                            .and_then(|t| t.get("runs"))
                            .and_then(|r| r.as_array())
                            .and_then(|a| a.first())
                            .and_then(|f| f.get("navigationEndpoint"))
                            .and_then(|n| n.get("browseEndpoint"))
                            .and_then(|b| b.get("browseId"))
                            .and_then(|b| b.as_str())
                            .or_else(|| {
                                card.get("onTap")
                                    .and_then(|o| o.get("browseEndpoint"))
                                    .and_then(|b| b.get("browseId"))
                                    .and_then(|b| b.as_str())
                            })
                            .map(|s| s.to_string());

                        let thumb = card
                            .get("thumbnail")
                            .and_then(|t| t.get("musicThumbnailRenderer"))
                            .and_then(|t| t.get("thumbnail"))
                            .and_then(|t| t.get("thumbnails"))
                            .and_then(|t| t.as_array())
                            .and_then(|arr| arr.last())
                            .and_then(|item| item.get("url"))
                            .and_then(|u| u.as_str())
                            .map(|u| resize_google_avatar_url(u, 512));

                        if id.is_some() {
                            browse_id = id;
                            canonical_name = name;
                            search_avatar = thumb;
                            break;
                        }
                    }
                }
            }
        }
    }

    let mut info = OnlineArtistInfo {
        name: canonical_name,
        image_url: search_avatar,
        browse_id: browse_id.clone(),
        ..Default::default()
    };

    // 2. Browse endpoint if browse_id is available
    if let Some(ref b_id) = browse_id {
        let browse_payload = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20240101.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "browseId": b_id
        });

        let browse_res = ureq::post("https://music.youtube.com/youtubei/v1/browse")
            .set("Content-Type", "application/json")
            .set(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .set("Origin", "https://music.youtube.com")
            .timeout(std::time::Duration::from_millis(3000))
            .send_json(browse_payload);

        if let Ok(resp) = browse_res {
            if let Ok(json) = resp.into_json::<serde_json::Value>() {
                if let Some(header) = json.get("header").and_then(|h| {
                    h.get("musicImmersiveHeaderRenderer")
                        .or_else(|| h.get("musicVisualHeaderRenderer"))
                }) {
                    if let Some(subs) = header
                        .get("subscriptionButton")
                        .and_then(|s| s.get("subscribeButtonRenderer"))
                        .and_then(|s| s.get("subscriberCountText"))
                        .and_then(|s| s.get("runs"))
                        .and_then(|r| r.as_array())
                        .and_then(|a| a.first())
                        .and_then(|f| f.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        info.subscribers = Some(subs.to_string());
                    }

                    if let Some(banner) = header
                        .get("thumbnail")
                        .and_then(|t| t.get("musicThumbnailRenderer"))
                        .and_then(|t| t.get("thumbnail"))
                        .and_then(|t| t.get("thumbnails"))
                        .and_then(|t| t.as_array())
                        .and_then(|arr| arr.last())
                        .and_then(|item| item.get("url"))
                        .and_then(|u| u.as_str())
                    {
                        info.banner_url = Some(banner.to_string());
                    }

                    if let Some(desc) = header
                        .get("description")
                        .and_then(|d| d.get("runs"))
                        .and_then(|r| r.as_array())
                        .and_then(|a| a.first())
                        .and_then(|f| f.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        info.bio = Some(clean_innertube_bio(desc));
                    }
                }

                // Check description shelf
                if let Some(contents) = json
                    .get("contents")
                    .and_then(|c| c.get("singleColumnBrowseResultsRenderer"))
                    .and_then(|s| s.get("tabs"))
                    .and_then(|t| t.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|t| t.get("tabRenderer"))
                    .and_then(|t| t.get("content"))
                    .and_then(|c| c.get("sectionListRenderer"))
                    .and_then(|s| s.get("contents"))
                    .and_then(|c| c.as_array())
                {
                    for section in contents {
                        if let Some(shelf) = section.get("musicDescriptionShelfRenderer") {
                            if let Some(subhead) = shelf
                                .get("subheader")
                                .and_then(|s| s.get("runs"))
                                .and_then(|r| r.as_array())
                                .and_then(|a| a.first())
                                .and_then(|f| f.get("text"))
                                .and_then(|t| t.as_str())
                            {
                                info.views = Some(subhead.to_string());
                            }

                            if let Some(runs) = shelf
                                .get("description")
                                .and_then(|d| d.get("runs"))
                                .and_then(|r| r.as_array())
                            {
                                let mut full_desc = String::new();
                                for r in runs {
                                    if let Some(txt) = r.get("text").and_then(|t| t.as_str()) {
                                        full_desc.push_str(txt);
                                    }
                                }
                                let cleaned = clean_innertube_bio(&full_desc);
                                if !cleaned.is_empty() {
                                    info.bio = Some(cleaned);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    info
}

fn query_wikipedia_summary(url: &str) -> OnlineArtistInfo {
    let res = ureq::get(url)
        .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
        .set("Accept", "application/json")
        .timeout(std::time::Duration::from_millis(2000))
        .call();

    match res {
        Ok(response) if response.status() == 200 => {
            if let Ok(val) = response.into_json::<serde_json::Value>() {
                let description = val.get("description")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string());
                let bio = val.get("extract")
                    .and_then(|e| e.as_str())
                    .map(|s| s.to_string());
                let image_url = val.get("thumbnail")
                    .and_then(|t| t.get("source"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        val.get("originalimage")
                            .and_then(|o| o.get("source"))
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string())
                    });

                OnlineArtistInfo {
                    description,
                    bio,
                    image_url,
                    ..Default::default()
                }
            } else {
                OnlineArtistInfo::default()
            }
        }
        _ => OnlineArtistInfo::default(),
    }
}

/// Asynchronously fetches comprehensive artist profile info from Wikipedia REST API.
/// Queries English Wikipedia first, and if unindexed, queries Japanese Wikipedia (`ja.wikipedia.org`).
/// If cached, returns immediately. If network fails or is unindexed, falls back gracefully.
pub fn fetch_wikipedia_artist_profile(artist: &str) -> OnlineArtistInfo {
    let clean = clean_primary_artist(artist);
    if clean.is_empty() || is_ignorable_artist(&clean) {
        return OnlineArtistInfo::default();
    }

    let encoded_name = urlencoding(&clean);

    // 1. Query English Wikipedia
    let en_url = format!(
        "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
        encoded_name
    );
    let mut info = query_wikipedia_summary(&en_url);

    // 2. If English Wikipedia yielded no bio or image, try Japanese Wikipedia
    if info.description.is_none() && info.bio.is_none() && info.image_url.is_none() {
        let ja_url = format!(
            "https://ja.wikipedia.org/api/rest_v1/page/summary/{}",
            encoded_name
        );
        let ja_info = query_wikipedia_summary(&ja_url);
        if ja_info.description.is_some() || ja_info.bio.is_some() || ja_info.image_url.is_some() {
            info = ja_info;
        }
    }

    info
}

/// Comprehensive online artist profile resolver.
///
/// Priority sequence:
/// 1. Memory / disk cache (unless `force_refresh` is true)
/// 2. Dynamic shared library C-ABI (`attune_metadata_fetch_artist_online`)
/// 3. Direct YouTube Music Innertube API (512x512 Google CDN portrait, 16:9 banner, subscriber count, views)
/// 4. Multi-language Wikipedia REST API fallback (`en.wikipedia.org` -> `ja.wikipedia.org`)
///
/// Results are persistently cached to disk in `~/.cache/attune/artists/{safe_name}.json`.
pub fn fetch_online_artist_profile(artist: &str, force_refresh: bool) -> OnlineArtistInfo {
    let clean = clean_primary_artist(artist);
    if clean.is_empty() || is_ignorable_artist(&clean) {
        return OnlineArtistInfo::default();
    }

    // 1. Check cache
    if !force_refresh {
        if let Some(cached) = load_cached_artist_profile(&clean) {
            if cached.is_verified() || cached.description.is_some() || cached.bio.is_some() || cached.image_url.is_some() {
                return cached;
            }
        }
    }

    // 2. Try dynamic C-ABI component if available
    let mut profile = if let Ok(json_str) = crate::dynamic_components::music_metadata_changer::fetch_artist_online(&clean) {
        if let Ok(c_abi_info) = serde_json::from_str::<OnlineArtistInfo>(&json_str) {
            c_abi_info
        } else {
            OnlineArtistInfo::default()
        }
    } else {
        OnlineArtistInfo::default()
    };

    // 3. Fallback to direct Innertube query if C-ABI was empty or missing avatar/bio
    if profile.image_url.is_none() || profile.bio.is_none() {
        let innertube_info = query_innertube_artist_profile(&clean);
        if profile.image_url.is_none() {
            profile.image_url = innertube_info.image_url;
        }
        if profile.bio.is_none() {
            profile.bio = innertube_info.bio;
        }
        if profile.banner_url.is_none() {
            profile.banner_url = innertube_info.banner_url;
        }
        if profile.subscribers.is_none() {
            profile.subscribers = innertube_info.subscribers;
        }
        if profile.views.is_none() {
            profile.views = innertube_info.views;
        }
        if profile.browse_id.is_none() {
            profile.browse_id = innertube_info.browse_id;
        }
    }

    // 4. Fallback to Wikipedia if avatar or bio still missing
    if profile.image_url.is_none() || profile.bio.is_none() {
        let wiki_info = fetch_wikipedia_artist_profile(&clean);
        if profile.image_url.is_none() {
            profile.image_url = wiki_info.image_url;
        }
        if profile.bio.is_none() {
            profile.bio = wiki_info.bio.or(wiki_info.description);
        }
    }

    // Save to disk & memory cache
    save_cached_artist_profile(&clean, &profile);

    profile
}

/// Asynchronously fetches the artist portrait image from Wikipedia REST API.
/// If cached, returns immediately. If network fails, falls back gracefully.
pub fn fetch_wikipedia_artist_image(artist: &str) -> Option<String> {
    fetch_wikipedia_artist_profile(artist).image_url
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
            ..Default::default()
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

    #[test]
    fn test_extract_all_unique_artists_splits_on_comma() {
        let tracks = vec![
            make_test_track("(Not) A Devil", "DECO*27, PinocchioP", "Singles", 180000),
            make_test_track("on the rocks", "OSTER project feat. KAITO, MEIKO", "mirai", 240000),
        ];

        let artists = extract_all_unique_artists(&tracks);
        let names: Vec<&str> = artists.iter().map(|a| a.name.as_str()).collect();

        // Must split comma-separated contributors into individual artists
        assert!(names.contains(&"DECO*27"));
        assert!(names.contains(&"PinocchioP"));
        assert!(names.contains(&"OSTER project"));
    }

    #[test]
    fn test_split_artist_string_with_ampersand_and_feat() {
        assert_eq!(
            split_artist_string("HALLO CEL & Kasane Teto"),
            vec!["HALLO CEL", "Kasane Teto"]
        );
        assert_eq!(
            split_artist_string("DECO*27 feat. Hatsune Miku"),
            vec!["DECO*27", "Hatsune Miku"]
        );
        assert_eq!(
            split_artist_string("OSTER project ft. KAITO, MEIKO"),
            vec!["OSTER project", "KAITO", "MEIKO"]
        );
        assert_eq!(
            split_artist_string("Artist A & Artist B & Artist C"),
            vec!["Artist A", "Artist B", "Artist C"]
        );
    }

    #[test]
    fn test_extract_featured_artists_from_title() {
        assert_eq!(
            extract_featured_artists_from_title("Teto the 31st (feat. KASANE TETO & Ui)"),
            vec!["KASANE TETO", "Ui"]
        );
        assert_eq!(
            extract_featured_artists_from_title("Song [ft. Artist B, Artist C]"),
            vec!["Artist B", "Artist C"]
        );
        assert_eq!(
            extract_featured_artists_from_title("Normal Song (Instrumental)"),
            Vec::<String>::new()
        );
        assert_eq!(
            extract_featured_artists_from_title("Remix Track (Original Mix)"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_extract_all_artists_from_track_pinpoints_collaborators() {
        let track = make_test_track(
            "Teto the 31st (feat. KASANE TETO & Ui)",
            "HALLO CEL & Kasane Teto",
            "Single",
            141000,
        );

        let artists = extract_all_artists_from_track(&track);
        assert_eq!(artists, vec!["HALLO CEL", "Kasane Teto", "Ui"]);

        assert!(track_has_artist(&track, "Kasane Teto"));
        assert!(track_has_artist(&track, "kasane teto"));
        assert!(track_has_artist(&track, "HALLO CEL"));
        assert!(track_has_artist(&track, "Ui"));
        assert!(!track_has_artist(&track, "Unknown Artist"));
    }

    #[test]
    fn test_extract_all_unique_artists_pinpoints_ampersand_and_featured() {
        let tracks = vec![
            make_test_track(
                "Teto the 31st (feat. KASANE TETO & Ui)",
                "HALLO CEL & Kasane Teto",
                "Single",
                141000,
            ),
        ];

        let artists = extract_all_unique_artists(&tracks);
        let names: Vec<&str> = artists.iter().map(|a| a.name.as_str()).collect();

        assert_eq!(names, vec!["HALLO CEL", "Kasane Teto", "Ui"]);
        assert!(!names.contains(&"HALLO CEL & Kasane Teto"));
    }

    #[test]
    fn test_split_artist_string_japanese_and_multilingual_delimiters() {
        assert_eq!(
            split_artist_string("DECO*27 × ピノキオピー"),
            vec!["DECO*27", "ピノキオピー"]
        );
        assert_eq!(
            split_artist_string("初音ミク・鏡音リン・鏡音レン"),
            vec!["初音ミク", "鏡音リン", "鏡音レン"]
        );
        assert_eq!(
            split_artist_string("Artist A ＆ Artist B"),
            vec!["Artist A", "Artist B"]
        );
        assert_eq!(
            split_artist_string("Artist A、Artist B；Artist C／Artist D"),
            vec!["Artist A", "Artist B", "Artist C", "Artist D"]
        );
    }

    #[test]
    fn test_split_artist_string_feat_without_trailing_space() {
        // Specifically testing "feat.<char>" without space, such as "daniwellP feat.初音ミク"
        assert_eq!(
            split_artist_string("daniwellP feat.初音ミク"),
            vec!["daniwellP", "初音ミク"]
        );
        assert_eq!(
            split_artist_string("ArtistA ft.ArtistB"),
            vec!["ArtistA", "ArtistB"]
        );
        assert_eq!(
            clean_primary_artist("daniwellP feat.初音ミク"),
            "daniwellP"
        );
        assert_eq!(
            extract_featured_artists_from_title("ねこみみスイッチ feat.初音ミク"),
            vec!["初音ミク"]
        );
        assert_eq!(
            extract_featured_artists_from_title("ねこみみスイッチ（feat.初音ミク）"),
            vec!["初音ミク"]
        );
    }

    #[test]
    fn test_extract_featured_artists_from_title_japanese_brackets() {
        assert_eq!(
            extract_featured_artists_from_title("メルト（feat. 初音ミク）"),
            vec!["初音ミク"]
        );
        assert_eq!(
            extract_featured_artists_from_title("Track Title【feat. KASANE TETO & Ui】"),
            vec!["KASANE TETO", "Ui"]
        );
    }

    #[test]
    fn test_find_favorite_artist_casing_consolidation() {
        let tracks = vec![
            make_test_track("Song 1", "Kasane Teto", "Album A", 120000),
            make_test_track("Song 2", "KASANE TETO", "Album B", 130000),
            make_test_track("Song 3", "Other Artist", "Album C", 140000),
        ];

        let fav = find_favorite_artist(&tracks, &tracks).expect("favorite artist found");
        assert_eq!(fav.artist, "Kasane Teto");
        assert_eq!(fav.tracks.len(), 2);
    }

    #[test]
    fn test_resize_google_avatar_url() {
        let url = "https://lh3.googleusercontent.com/abc=w120-h120-p-l90-rj";
        let resized = resize_google_avatar_url(url, 512);
        assert_eq!(resized, "https://lh3.googleusercontent.com/abc=w512-h512-p-l90-rj");

        let plain = "https://lh3.googleusercontent.com/xyz";
        let resized_plain = resize_google_avatar_url(plain, 512);
        assert_eq!(resized_plain, "https://lh3.googleusercontent.com/xyz=w512-h512-p-l90-rj");
    }

    #[test]
    fn test_online_artist_info_is_verified() {
        let empty = OnlineArtistInfo::default();
        assert!(!empty.is_verified());

        let local_only = OnlineArtistInfo {
            name: Some("Local Band".to_string()),
            ..Default::default()
        };
        assert!(!local_only.is_verified());

        let with_browse_id = OnlineArtistInfo {
            name: Some("DECO*27".to_string()),
            browse_id: Some("UC123".to_string()),
            ..Default::default()
        };
        assert!(with_browse_id.is_verified());

        let with_subs = OnlineArtistInfo {
            name: Some("Artist".to_string()),
            subscribers: Some("1M".to_string()),
            ..Default::default()
        };
        assert!(with_subs.is_verified());

        let with_bio = OnlineArtistInfo {
            name: Some("Artist".to_string()),
            bio: Some("Wikipedia bio".to_string()),
            ..Default::default()
        };
        assert!(with_bio.is_verified());
    }

    #[test]
    fn test_clean_innertube_bio() {
        let text = "Deco*27 is a Japanese musician.\n\nFrom Wikipedia (https://en.wikipedia.org/wiki/Deco*27)";
        assert_eq!(clean_innertube_bio(text), "Deco*27 is a Japanese musician.");

        let text2 = "No wiki link here";
        assert_eq!(clean_innertube_bio(text2), "No wiki link here");
    }

    #[test]
    fn test_normalize_artist_characters_altmap() {
        assert_eq!(normalize_artist_characters("DECO＊27"), "DECO*27");
        assert_eq!(normalize_artist_characters("DECO*27"), "DECO*27");
        assert_eq!(canonical_artist_key("DECO＊27"), "deco*27");
        assert_eq!(canonical_artist_key("DECO*27"), "deco*27");
        assert_eq!(normalize_artist_characters("CosMo＠暴走P"), "CosMo@暴走P");
        assert_eq!(normalize_artist_characters("ＡＢＢＡ"), "ABBA");
        assert_eq!(normalize_artist_characters("初音ミク　"), "初音ミク");
        assert_eq!(normalize_artist_characters("BPM １２０"), "BPM 120");
    }

    #[test]
    fn test_extract_all_unique_artists_deduplicates_fullwidth_asterisk() {
        let tracks = vec![
            make_test_track("Rabbit Hole", "DECO*27", "Album 1", 180000),
            make_test_track("Ghost Rule", "DECO*27", "Album 2", 200000),
            make_test_track("YUMEYUME", "DECO＊27", "Project Mirai", 220000),
            make_test_track("Other Song", "PinocchioP", "PINO", 150000),
        ];

        let summaries = extract_all_unique_artists(&tracks);
        assert_eq!(summaries.len(), 2);

        let deco = summaries.iter().find(|s| canonical_artist_key(&s.name) == "deco*27").expect("deco found");
        assert_eq!(deco.track_count, 3);
        assert_eq!(deco.album_count, 3);
    }

    #[test]
    fn test_artist_cache_memory_and_file_path() {
        let test_artist = "__attune_unit_test_mock_artist__";
        let path = artist_cache_file_path(test_artist).expect("path created");
        assert!(path.to_string_lossy().contains("__attune_unit_test_mock_artist__"));

        let info = OnlineArtistInfo {
            name: Some(test_artist.to_string()),
            image_url: Some("https://images.unsplash.com/mock-avatar.jpg".to_string()),
            bio: Some("Famous producer".to_string()),
            subscribers: Some("2.91M".to_string()),
            views: Some("1.8B views".to_string()),
            browse_id: Some("UC123".to_string()),
            ..Default::default()
        };
        save_cached_artist_profile(test_artist, &info);

        let loaded = load_cached_artist_profile(test_artist).expect("cached loaded");
        assert_eq!(loaded.name.as_deref(), Some(test_artist));
        assert_eq!(loaded.image_url.as_deref(), Some("https://images.unsplash.com/mock-avatar.jpg"));
        assert_eq!(loaded.subscribers.as_deref(), Some("2.91M"));

        // Clean up mock file so tests never pollute user cache
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_parse_musixmatch_artist_candidates() {
        let sample_json: serde_json::Value = serde_json::from_str(
            r#"{
                "message": {
                    "header": { "status_code": 200, "execute_time": 0.05 },
                    "body": {
                        "artist_list": [
                            {
                                "artist": {
                                    "artist_id": 118,
                                    "artist_name": "Attune Ensemble",
                                    "artist_comment": "Electronic DSP Group",
                                    "artist_country": "JP",
                                    "artist_rating": 91,
                                    "artist_alias_list": [
                                        { "artist_alias": "アチューン" }
                                    ],
                                    "primary_genres": {
                                        "music_genre_list": [
                                            {
                                                "music_genre": {
                                                    "music_genre_name": "Electronic",
                                                    "music_genre_name_extended": "Electronic / Synth"
                                                }
                                            }
                                        ]
                                    }
                                }
                            }
                        ]
                    }
                }
            }"#,
        )
        .unwrap();

        let cands = parse_musixmatch_artist_candidates(&sample_json);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].name, "Attune Ensemble");
        assert_eq!(cands[0].provider, "Musixmatch");
        assert_eq!(cands[0].subscribers.as_deref(), Some("Rating 91/100"));
        assert_eq!(cands[0].browse_id.as_deref(), Some("mxm-artist-118"));
        assert!(cands[0].description.as_deref().unwrap_or("").contains("Electronic DSP Group"));
        assert!(cands[0].description.as_deref().unwrap_or("").contains("Country: JP"));
        assert!(cands[0].bio.as_deref().unwrap_or("").contains("アチューン"));
    }

    #[test]
    fn test_extract_all_unique_artists_parallel_offload() {
        let mut tracks = Vec::new();
        for i in 0..50 {
            let artist = match i % 5 {
                0 => "Hatsune Miku",
                1 => "DECO*27 feat. Hatsune Miku",
                2 => "PinocchioP, DECO*27",
                3 => "KASANE TETO & Ui",
                _ => "Unknown Contributor",
            };
            tracks.push(make_test_track(&format!("Song {i}"), artist, &format!("Album {}", i % 3), 180000));
        }

        let seq_summaries = extract_all_unique_artists_with_offload(&tracks, false);
        let par_summaries = extract_all_unique_artists_with_offload(&tracks, true);

        assert_eq!(seq_summaries.len(), par_summaries.len());
        for (s, p) in seq_summaries.iter().zip(par_summaries.iter()) {
            assert_eq!(s.name, p.name);
            assert_eq!(s.track_count, p.track_count);
            assert_eq!(s.album_count, p.album_count);
        }
    }

    #[test]
    fn test_is_valid_profile_image_url_accepts_data_url_and_local() {
        assert!(is_valid_profile_image_url("https://images.google.com/avatar.jpg"));
        assert!(is_valid_profile_image_url("http://cdn.example.org/photo.png"));
        assert!(!is_valid_profile_image_url("http://example.com/bogus.jpg"));
        assert!(is_valid_profile_image_url("data:image/jpeg;base64,/9j/4AAQSkZJRg=="));
        assert!(is_valid_profile_image_url("file:///home/user/Pictures/artist.jpg"));
        assert!(!is_valid_profile_image_url(""));
        assert!(!is_valid_profile_image_url("   "));
    }

    #[test]
    fn test_save_and_load_manual_unverified_artist_profile() {
        let test_artist = "Manual Indie Producer";
        let info = OnlineArtistInfo {
            name: Some(test_artist.to_string()),
            description: Some("Independent Chiptune Artist".to_string()),
            bio: Some("Produces 8-bit synthetic tunes".to_string()),
            image_url: Some("data:image/jpeg;base64,/9j/4AAQSkZJRg==".to_string()),
            banner_url: None,
            subscribers: None,
            views: None,
            browse_id: Some("custom-manual-linked".to_string()),
        };

        save_cached_artist_profile(test_artist, &info);

        let loaded = load_cached_artist_profile(test_artist).expect("profile loaded");
        assert!(loaded.is_verified());
        assert_eq!(loaded.description.as_deref(), Some("Independent Chiptune Artist"));
        assert_eq!(loaded.bio.as_deref(), Some("Produces 8-bit synthetic tunes"));

        let online = fetch_online_artist_profile(test_artist, false);
        assert!(online.is_verified());
        assert_eq!(online.description.as_deref(), Some("Independent Chiptune Artist"));

        // Cleanup
        if let Some(path) = artist_cache_file_path(test_artist) {
            let _ = std::fs::remove_file(path);
        }
    }
}
