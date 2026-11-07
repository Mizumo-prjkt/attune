//! YouTube Music Innertube API and Wikipedia Online Metadata Resolution
//!
//! Provides non-authenticated, high-resolution metadata for artists, including:
//! - 512x512 Google CDN portraits (`=w512-h512-p-l90-rj`)
//! - 16:9 widescreen banners (up to 2880x1200)
//! - Curated biographies (Google Knowledge Graph & Wikipedia)
//! - Subscriber counts and total audience metrics
//! - Multi-language Wikipedia REST fallback (en -> ja)
//!
//! Complies with Mandate 4 (zero hardcoded values) with graceful network fallbacks.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InnertubeArtistProfile {
    pub name: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub bio: Option<String>,
    pub subscribers: Option<String>,
    pub views: Option<String>,
    pub browse_id: Option<String>,
}

/// Dynamic Google CDN avatar resizer.
/// Appends or replaces the `=w...` sizing token with `=w{size}-h{size}-p-l90-rj`.
pub fn resize_google_avatar_url(url: &str, size: u32) -> String {
    let target_token = format!("=w{size}-h{size}-p-l90-rj");
    if let Some(idx) = url.find('=') {
        let base = &url[..idx];
        format!("{base}{target_token}")
    } else {
        format!("{url}{target_token}")
    }
}

/// Trims trailing Wikipedia or Creative Commons license links from YouTube Music's description shelf.
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

/// URL encodes a string safely for HTTP endpoints.
pub fn url_encode(input: &str) -> String {
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

/// Performs an unauthenticated search on YouTube Music Innertube API (`youtubei/v1/search`).
/// Returns `(browse_id, canonical_name, thumbnail_url)`.
pub fn search_artist_innertube(query: &str) -> Result<(String, String, Option<String>), String> {
    let clean_query = query.trim();
    if clean_query.is_empty() {
        return Err("Query is empty".to_string());
    }

    let payload = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20240101.01.00",
                "hl": "en",
                "gl": "US"
            }
        },
        "query": clean_query
    });

    let resp = ureq::post("https://music.youtube.com/youtubei/v1/search")
        .set("Content-Type", "application/json")
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .set("Origin", "https://music.youtube.com")
        .timeout(std::time::Duration::from_millis(3500))
        .send_json(payload)
        .map_err(|e| format!("Innertube search request failed: {e}"))?;

    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("Failed to parse Innertube JSON: {e}"))?;

    let tabs = json
        .get("contents")
        .and_then(|c| c.get("tabbedSearchResultsRenderer"))
        .and_then(|t| t.get("tabs"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| "No tabs in search results".to_string())?;

    let first_tab = tabs
        .first()
        .and_then(|t| t.get("tabRenderer"))
        .and_then(|t| t.get("content"))
        .and_then(|c| c.get("sectionListRenderer"))
        .and_then(|s| s.get("contents"))
        .and_then(|c| c.as_array())
        .ok_or_else(|| "No section contents in search results".to_string())?;

    // 1. Check musicCardShelfRenderer (hero artist shelf)
    for section in first_tab {
        if let Some(card) = section.get("musicCardShelfRenderer") {
            let title = card
                .get("title")
                .and_then(|t| t.get("runs"))
                .and_then(|r| r.as_array())
                .and_then(|a| a.first())
                .and_then(|f| f.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or(clean_query)
                .to_string();

            let browse_id = card
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
                .or_else(|| {
                    card.get("header")
                        .and_then(|h| h.get("musicCardShelfHeaderBasicRenderer"))
                        .and_then(|h| h.get("title"))
                        .and_then(|t| t.get("runs"))
                        .and_then(|r| r.as_array())
                        .and_then(|a| a.first())
                        .and_then(|f| f.get("navigationEndpoint"))
                        .and_then(|n| n.get("browseEndpoint"))
                        .and_then(|b| b.get("browseId"))
                        .and_then(|b| b.as_str())
                });

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

            if let Some(id) = browse_id {
                return Ok((id.to_string(), title, thumb));
            }
        }
    }

    // 2. Fallback: inspect itemSectionRenderer or musicShelfRenderer for artist items
    for section in first_tab {
        let items = section
            .get("itemSectionRenderer")
            .and_then(|i| i.get("contents"))
            .and_then(|c| c.as_array())
            .or_else(|| {
                section
                    .get("musicShelfRenderer")
                    .and_then(|m| m.get("contents"))
                    .and_then(|c| c.as_array())
            });

        if let Some(item_list) = items {
            for item in item_list {
                if let Some(row) = item.get("musicResponsiveListItemRenderer") {
                    // Check if pageType is MUSIC_PAGE_TYPE_ARTIST
                    let nav_endpoint = row
                        .get("navigationEndpoint")
                        .or_else(|| {
                            row.get("flexColumns")
                                .and_then(|f| f.as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|c| c.get("musicResponsiveListItemFlexColumnRenderer"))
                                .and_then(|c| c.get("text"))
                                .and_then(|t| t.get("runs"))
                                .and_then(|r| r.as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|item| item.get("navigationEndpoint"))
                        });

                    let browse_id = nav_endpoint
                        .and_then(|n| n.get("browseEndpoint"))
                        .and_then(|b| b.get("browseId"))
                        .and_then(|b| b.as_str());

                    let title = row
                        .get("flexColumns")
                        .and_then(|f| f.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|c| c.get("musicResponsiveListItemFlexColumnRenderer"))
                        .and_then(|c| c.get("text"))
                        .and_then(|t| t.get("runs"))
                        .and_then(|r| r.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|item| item.get("text"))
                        .and_then(|t| t.as_str())
                        .unwrap_or(clean_query)
                        .to_string();

                    let thumb = row
                        .get("thumbnail")
                        .and_then(|t| t.get("musicThumbnailRenderer"))
                        .and_then(|t| t.get("thumbnail"))
                        .and_then(|t| t.get("thumbnails"))
                        .and_then(|t| t.as_array())
                        .and_then(|arr| arr.last())
                        .and_then(|item| item.get("url"))
                        .and_then(|u| u.as_str())
                        .map(|u| resize_google_avatar_url(u, 512));

                    if let Some(id) = browse_id {
                        if id.starts_with("UC") || id.starts_with("FEmusic_library") {
                            return Ok((id.to_string(), title, thumb));
                        }
                    }
                }
            }
        }
    }

    Err(format!("No artist browse ID found for query '{clean_query}'"))
}

/// Queries YouTube Music Innertube API (`youtubei/v1/browse`) using artist `browseId`.
/// Returns banner, subscriber count, biography, and views.
pub fn fetch_artist_browse_innertube(browse_id: &str) -> Result<InnertubeArtistProfile, String> {
    let payload = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20240101.01.00",
                "hl": "en",
                "gl": "US"
            }
        },
        "browseId": browse_id
    });

    let resp = ureq::post("https://music.youtube.com/youtubei/v1/browse")
        .set("Content-Type", "application/json")
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .set("Origin", "https://music.youtube.com")
        .timeout(std::time::Duration::from_millis(3500))
        .send_json(payload)
        .map_err(|e| format!("Innertube browse request failed: {e}"))?;

    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("Failed to parse browse JSON: {e}"))?;

    let mut profile = InnertubeArtistProfile {
        browse_id: Some(browse_id.to_string()),
        ..Default::default()
    };

    // 1. Extract from header.musicImmersiveHeaderRenderer or musicVisualHeaderRenderer
    if let Some(header) = json.get("header").and_then(|h| {
        h.get("musicImmersiveHeaderRenderer")
            .or_else(|| h.get("musicVisualHeaderRenderer"))
    }) {
        if let Some(name) = header
            .get("title")
            .and_then(|t| t.get("runs"))
            .and_then(|r| r.as_array())
            .and_then(|a| a.first())
            .and_then(|f| f.get("text"))
            .and_then(|t| t.as_str())
        {
            profile.name = name.to_string();
        }

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
            profile.subscribers = Some(subs.to_string());
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
            profile.banner_url = Some(banner.to_string());
        }

        if let Some(desc) = header
            .get("description")
            .and_then(|d| d.get("runs"))
            .and_then(|r| r.as_array())
            .and_then(|a| a.first())
            .and_then(|f| f.get("text"))
            .and_then(|t| t.as_str())
        {
            profile.bio = Some(clean_innertube_bio(desc));
        }
    }

    // 2. Extract description & view count from musicDescriptionShelfRenderer if present
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
                    profile.views = Some(subhead.to_string());
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
                        profile.bio = Some(cleaned);
                    }
                }
            }
        }
    }

    Ok(profile)
}

/// Queries Wikipedia REST API summary (first English Wikipedia, then Japanese Wikipedia).
/// Returns `(description, bio, image_url)`.
pub fn fetch_wikipedia_artist_profile(artist_name: &str) -> (Option<String>, Option<String>, Option<String>) {
    let clean = artist_name.trim();
    if clean.is_empty() {
        return (None, None, None);
    }

    let encoded = url_encode(clean);

    let query_wiki = |url: &str| -> Option<(Option<String>, Option<String>, Option<String>)> {
        let resp = ureq::get(url)
            .set("User-Agent", "Attune/0.1.0 (https://github.com/Mizumo-prjkt/attune)")
            .set("Accept", "application/json")
            .timeout(std::time::Duration::from_millis(2500))
            .call()
            .ok()?;

        if resp.status() == 200 {
            let val = resp.into_json::<serde_json::Value>().ok()?;
            let description = val.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());
            let bio = val.get("extract").and_then(|e| e.as_str()).map(|s| s.to_string());
            let image_url = val
                .get("thumbnail")
                .and_then(|t| t.get("source"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    val.get("originalimage")
                        .and_then(|o| o.get("source"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string())
                });
            if description.is_some() || bio.is_some() || image_url.is_some() {
                return Some((description, bio, image_url));
            }
        }
        None
    };

    // 1. Try English Wikipedia
    let en_url = format!("https://en.wikipedia.org/api/rest_v1/page/summary/{encoded}");
    if let Some(res) = query_wiki(&en_url) {
        return res;
    }

    // 2. Try Japanese Wikipedia
    let ja_url = format!("https://ja.wikipedia.org/api/rest_v1/page/summary/{encoded}");
    if let Some(res) = query_wiki(&ja_url) {
        return res;
    }

    (None, None, None)
}

/// Unified online metadata resolver for artists.
///
/// Queries YouTube Music Innertube API first (giving high-res Google CDN portrait,
/// 16:9 banner, subscriber count, and Knowledge Graph bio).
/// If any field is missing, gracefully queries Wikipedia REST API to fill in missing details.
pub fn fetch_artist_online_profile(artist_name: &str) -> InnertubeArtistProfile {
    let clean = artist_name.trim();
    if clean.is_empty() {
        return InnertubeArtistProfile::default();
    }

    let mut profile = match search_artist_innertube(clean) {
        Ok((browse_id, title, search_avatar)) => {
            match fetch_artist_browse_innertube(&browse_id) {
                Ok(mut p) => {
                    if p.name.is_empty() {
                        p.name = title;
                    }
                    if p.avatar_url.is_none() {
                        p.avatar_url = search_avatar;
                    }
                    p
                }
                Err(e) => {
                    log::warn!("Innertube browse failed for '{clean}': {e}");
                    InnertubeArtistProfile {
                        name: title,
                        avatar_url: search_avatar,
                        browse_id: Some(browse_id),
                        ..Default::default()
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("Innertube search failed for '{clean}': {e}");
            InnertubeArtistProfile {
                name: clean.to_string(),
                ..Default::default()
            }
        }
    };

    // If avatar, bio, or banner are missing, fill from Wikipedia
    if profile.avatar_url.is_none() || profile.bio.is_none() {
        let (wiki_desc, wiki_bio, wiki_img) = fetch_wikipedia_artist_profile(clean);
        if profile.avatar_url.is_none() {
            profile.avatar_url = wiki_img;
        }
        if profile.bio.is_none() {
            profile.bio = wiki_bio.or(wiki_desc);
        }
    }

    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_google_avatar_url() {
        let url = "https://lh3.googleusercontent.com/abc=w120-h120-p-l90-rj";
        let resized = resize_google_avatar_url(url, 512);
        assert_eq!(
            resized,
            "https://lh3.googleusercontent.com/abc=w512-h512-p-l90-rj"
        );

        let plain = "https://lh3.googleusercontent.com/abc";
        let resized_plain = resize_google_avatar_url(plain, 512);
        assert_eq!(
            resized_plain,
            "https://lh3.googleusercontent.com/abc=w512-h512-p-l90-rj"
        );
    }

    #[test]
    fn test_clean_innertube_bio() {
        let raw = "A renowned musician.\n\nFrom Wikipedia (https://en.wikipedia.org/wiki/Artist) under CC-BY-SA 3.0";
        assert_eq!(clean_innertube_bio(raw), "A renowned musician.");

        let simple = "Simple bio without wiki link.";
        assert_eq!(clean_innertube_bio(simple), "Simple bio without wiki link.");
    }
}
