//! Background library scanning: discovers audio files and probes their tags
//! without ever blocking the egui render thread.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
use std::thread;

use attune_core::{AudioDecoder, LibraryScanner};

static THUMBNAIL_CACHE: Mutex<Option<HashMap<PathBuf, String>>> = Mutex::new(None);

/// Clears the in-memory thumbnail string cache to free heap memory.
pub fn clear_thumbnail_cache() {
    let mut guard = THUMBNAIL_CACHE.lock().unwrap();
    if let Some(ref mut cache) = *guard {
        cache.clear();
    }
}

/// Resolves or downsamples an album cover to a lightweight (~160x160 JPEG, ~4-8 KB) Base64 Data URL.
/// Deduplicates across tracks in the same album to eliminate memory bloat on library scan.
pub fn resolve_thumbnail_data_url(art_p: &Path) -> Option<String> {
    if !art_p.is_file() {
        return None;
    }

    // 1. Fast path: check in-memory deduplication cache
    {
        let mut guard = THUMBNAIL_CACHE.lock().unwrap();
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(existing) = cache.get(art_p) {
            return Some(existing.clone());
        }
    }

    // 2. Check disk thumbnail cache in ~/.cache/attune-desktop/thumbnails/
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("attune-desktop")
        .join("thumbnails");
    let _ = std::fs::create_dir_all(&cache_dir);

    let art_meta = art_p.metadata().ok()?;
    let mod_time = art_meta.modified().ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let key = format!("{}-{}-{}", art_p.to_string_lossy(), art_meta.len(), mod_time);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    let hash_hex = format!("{:016x}", hasher.finish());
    let cached_file = cache_dir.join(format!("{hash_hex}.jpg"));

    let jpeg_bytes = if cached_file.is_file() {
        std::fs::read(&cached_file).ok()?
    } else {
        let raw_bytes = std::fs::read(art_p).ok()?;
        // If image is already tiny (<= 16 KB), keep directly
        if raw_bytes.len() <= 16 * 1024 {
            raw_bytes
        } else if let Ok(img) = image::load_from_memory(&raw_bytes) {
            let thumb = img.thumbnail(160, 160);
            let mut out = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut out);
            if thumb.write_to(&mut cursor, image::ImageFormat::Jpeg).is_ok() {
                let _ = std::fs::write(&cached_file, &out);
                out
            } else {
                raw_bytes
            }
        } else {
            raw_bytes
        }
    };

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);
    let data_url = format!("data:image/jpeg;base64,{b64}");

    // Store in in-memory cache
    {
        let mut guard = THUMBNAIL_CACHE.lock().unwrap();
        if let Some(ref mut cache) = *guard {
            cache.insert(art_p.to_path_buf(), data_url.clone());
        }
    }

    Some(data_url)
}

/// A single playable track in the desktop library.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackEntry {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: Option<String>,
    pub duration_ms: u64,
    pub bpm: Option<f32>,
    pub key: Option<String>,
    pub format: String,
    pub sample_rate: u32,
    #[allow(dead_code)]
    pub bit_depth: u32,
    pub artwork_path: Option<PathBuf>,
    pub artwork_data_url: Option<String>,
}

impl TrackEntry {
    /// Builds an entry from a probed file; falls back to filename/"Unknown" only
    /// when tags are genuinely missing (never fabricated BPM/duration).
    pub fn from_probe(path: &Path) -> Self {
        let file_stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        match AudioDecoder::probe_metadata(path) {
            Ok(p) => {
                let mut artwork_path = p.artwork_path.map(PathBuf::from);
                if artwork_path.as_ref().map(|p| !p.is_file()).unwrap_or(true) {
                    if let Some(parent) = path.parent() {
                        for candidate_name in &[
                            "cover.jpg", "cover.png", "folder.jpg", "folder.png",
                            "front.jpg", "front.png", "Cover.jpg", "Cover.png",
                        ] {
                            let cand = parent.join(candidate_name);
                            if cand.is_file() {
                                artwork_path = Some(cand);
                                break;
                            }
                        }
                    }
                }

                let artwork_data_url = artwork_path.as_deref().and_then(resolve_thumbnail_data_url);

                TrackEntry {
                    path: path.to_path_buf(),
                    title: p.title.filter(|t| !t.trim().is_empty()).unwrap_or(file_stem),
                    artist: p.artist.filter(|a| !a.trim().is_empty()).unwrap_or_else(|| "Unknown Artist".into()),
                    album: p.album.filter(|a| !a.trim().is_empty()).unwrap_or_else(|| "Unknown Album".into()),
                    genre: p.genre,
                    duration_ms: p.duration_ms,
                    bpm: p.bpm,
                    key: p.initial_key,
                    format: p.format,
                    sample_rate: p.sample_rate,
                    bit_depth: p.bit_depth,
                    artwork_path,
                    artwork_data_url,
                }
            }
            Err(e) => {
                log::warn!("Metadata probe failed for {}: {}", path.display(), e);
                TrackEntry {
                    path: path.to_path_buf(),
                    title: file_stem,
                    artist: "Unknown Artist".into(),
                    album: "Unknown Album".into(),
                    genre: None,
                    duration_ms: 0,
                    bpm: None,
                    key: None,
                    format: path
                        .extension()
                        .map(|e| e.to_string_lossy().to_uppercase())
                        .unwrap_or_default(),
                    sample_rate: 0,
                    bit_depth: 0,
                    artwork_path: None,
                    artwork_data_url: None,
                }
            }
        }
    }

    /// Multi-line hover summary of the track's container + tag profile.
    pub fn detail_line(&self) -> String {
        let mut parts = vec![format!("{} — {}", self.artist, self.album)];
        if let Some(g) = self.genre.as_deref().filter(|g| !g.is_empty()) {
            parts.push(format!("Genre: {g}"));
        }
        if let Some(k) = self.key.as_deref().filter(|k| !k.is_empty()) {
            parts.push(format!("Key: {k}"));
        }
        if let Some(b) = self.bpm {
            parts.push(format!("BPM: {b:.1}"));
        }
        parts.push(self.path.display().to_string());
        parts.join("\n")
    }

    /// True if the given lowercase query matches title, artist or album.
    pub fn matches(&self, query_lower: &str) -> bool {
        query_lower.is_empty()
            || self.title.to_lowercase().contains(query_lower)
            || self.artist.to_lowercase().contains(query_lower)
            || self.album.to_lowercase().contains(query_lower)
    }
}

/// Progress events emitted by the scan worker.
pub enum ScanEvent {
    Discovered(usize),
    Track(Box<TrackEntry>),
    Finished,
}

/// Spawns a worker that discovers and probes every audio file under `root`.
pub fn spawn_scan(root: PathBuf) -> Receiver<ScanEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("attune-library-scan".into())
        .spawn(move || {
            let files = LibraryScanner::discover_audio_files(&root);
            log::info!("Discovered {} audio files under {}", files.len(), root.display());
            if tx.send(ScanEvent::Discovered(files.len())).is_err() {
                return;
            }
            for file in files.iter() {
                if tx.send(ScanEvent::Track(Box::new(TrackEntry::from_probe(file)))).is_err() {
                    return; // Receiver dropped — stop quietly.
                }
            }
            let _ = tx.send(ScanEvent::Finished);
        })
        .expect("failed to spawn library scan thread");
    rx
}

/// Formats milliseconds as `m:ss`.
pub fn fmt_ms(ms: u64) -> String {
    let total = ms / 1000;
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, artist: &str, album: &str) -> TrackEntry {
        TrackEntry {
            path: PathBuf::from(format!("{title}.flac")),
            title: title.into(),
            artist: artist.into(),
            album: album.into(),
            genre: Some("Vocaloid".into()),
            duration_ms: 159_000,
            bpm: Some(171.4),
            key: Some("F#m".into()),
            format: "FLAC".into(),
            sample_rate: 44_100,
            bit_depth: 16,
            artwork_path: None,
            artwork_data_url: None,
        }
    }

    #[test]
    fn search_matches_title_artist_album_case_insensitively() {
        let t = entry("Rabbit Hole", "DECO*27", "Rabbit Hole - Single");
        assert!(t.matches(""));
        assert!(t.matches("rabbit"));
        assert!(t.matches("deco"));
        assert!(t.matches("single"));
        assert!(!t.matches("mariah"));
    }

    #[test]
    fn fmt_ms_formats_minutes_and_seconds() {
        assert_eq!(fmt_ms(0), "0:00");
        assert_eq!(fmt_ms(159_000), "2:39");
        assert_eq!(fmt_ms(3_723_000), "62:03");
    }

    #[test]
    fn detail_line_includes_profile_fields() {
        let d = entry("Rabbit Hole", "DECO*27", "Single").detail_line();
        assert!(d.contains("Genre: Vocaloid"));
        assert!(d.contains("Key: F#m"));
        assert!(d.contains("BPM: 171.4"));
    }

    #[test]
    fn probe_of_missing_file_falls_back_without_fabricating_values() {
        let t = TrackEntry::from_probe(Path::new("/nonexistent/attune/Some Song.mp3"));
        assert_eq!(t.title, "Some Song");
        assert_eq!(t.duration_ms, 0);
        assert!(t.bpm.is_none());
        assert_eq!(t.format, "MP3");
    }
}
