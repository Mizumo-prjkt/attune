//! Background library scanning and deep audio profiling: discovers audio files,
//! extracts genuine CQT Camelot musical keys & nominal BPMs via AudioProfiler,
//! and caches the indexed library to disk for instantaneous startup.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
use std::thread;

use attune_core::profiler::AudioProfiler;
use attune_core::schema::AnalysisTier;
use attune_core::{AudioDecoder, LibraryScanner};
use serde::{Deserialize, Serialize};

static THUMBNAIL_CACHE: Mutex<Option<HashMap<PathBuf, String>>> = Mutex::new(None);

/// Clears the in-memory thumbnail string cache to free heap memory.
pub fn clear_thumbnail_cache() {
    let mut guard = THUMBNAIL_CACHE.lock().unwrap();
    if let Some(ref mut cache) = *guard {
        cache.clear();
    }
}

/// Resolves an album cover to a Base64 Data URL, supporting both high-quality mode (up to 1024x1024 HD)
/// and lightweight mode (~160x160 JPEG).
/// Deduplicates across tracks in the same album to eliminate memory bloat on library scan.
pub fn resolve_thumbnail_data_url_with_quality(art_p: &Path, high_quality: bool) -> Option<String> {
    if !art_p.is_file() {
        return None;
    }

    // 1. Fast path: check in-memory deduplication cache
    let mem_cache_key = format!("{}:{}", art_p.to_string_lossy(), if high_quality { "hq" } else { "lq" });
    {
        let mut guard = THUMBNAIL_CACHE.lock().unwrap();
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(existing) = cache.get(Path::new(&mem_cache_key)) {
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
    let mod_time = art_meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let key = format!("{}-{}-{}-{}", art_p.to_string_lossy(), art_meta.len(), mod_time, if high_quality { "hq" } else { "lq" });

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    let hash_hex = format!("{:016x}", hasher.finish());
    let cached_file = cache_dir.join(format!("{hash_hex}_{}.jpg", if high_quality { "hq" } else { "lq" }));

    let (image_bytes, mime_type) = if cached_file.is_file() {
        (std::fs::read(&cached_file).ok()?, "image/jpeg")
    } else {
        let raw_bytes = std::fs::read(art_p).ok()?;
        let is_png = raw_bytes.starts_with(b"\x89PNG\r\n\x1a\n");
        let is_webp = raw_bytes.len() >= 12 && &raw_bytes[0..4] == b"RIFF" && &raw_bytes[8..12] == b"WEBP";

        if high_quality {
            // High quality mode: If original image is <= 512 KB, preserve original bytes directly
            if raw_bytes.len() <= 512 * 1024 {
                let mime = if is_png { "image/png" } else if is_webp { "image/webp" } else { "image/jpeg" };
                (raw_bytes, mime)
            } else if let Ok(img) = image::load_from_memory(&raw_bytes) {
                // Downscale gracefully to max 1024x1024 HD
                let thumb = img.thumbnail(1024, 1024);
                let mut out = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut out);
                if thumb.write_to(&mut cursor, image::ImageFormat::Jpeg).is_ok() {
                    let _ = std::fs::write(&cached_file, &out);
                    (out, "image/jpeg")
                } else {
                    (raw_bytes, "image/jpeg")
                }
            } else {
                (raw_bytes, "image/jpeg")
            }
        } else {
            // Standard mode: lightweight 160x160 thumbnail
            if raw_bytes.len() <= 16 * 1024 {
                let mime = if is_png { "image/png" } else if is_webp { "image/webp" } else { "image/jpeg" };
                (raw_bytes, mime)
            } else if let Ok(img) = image::load_from_memory(&raw_bytes) {
                let thumb = img.thumbnail(160, 160);
                let mut out = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut out);
                if thumb.write_to(&mut cursor, image::ImageFormat::Jpeg).is_ok() {
                    let _ = std::fs::write(&cached_file, &out);
                    (out, "image/jpeg")
                } else {
                    (raw_bytes, "image/jpeg")
                }
            } else {
                (raw_bytes, "image/jpeg")
            }
        }
    };

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
    let data_url = format!("data:{mime_type};base64,{b64}");

    // Store in in-memory cache
    {
        let mut guard = THUMBNAIL_CACHE.lock().unwrap();
        if let Some(ref mut cache) = *guard {
            cache.insert(PathBuf::from(mem_cache_key), data_url.clone());
            cache.insert(art_p.to_path_buf(), data_url.clone());
        }
    }

    Some(data_url)
}

/// Resolves an album cover to a Base64 Data URL using the active configuration's thumbnail quality setting.
pub fn resolve_thumbnail_data_url(art_p: &Path) -> Option<String> {
    let hq = crate::config::load_config().high_quality_thumbnails;
    resolve_thumbnail_data_url_with_quality(art_p, hq)
}

/// A single playable track in the desktop library.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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
    #[serde(default)]
    pub aggression: Option<f32>,
    #[serde(default)]
    pub vibe_family: Option<String>,
    #[serde(default)]
    pub composer: Option<String>,
    #[serde(default)]
    pub writer: Option<String>,
    #[serde(default)]
    pub vocalist: Option<String>,
    #[serde(default)]
    pub producer: Option<String>,
}

impl TrackEntry {
    /// Builds an entry from a probed file metadata; falls back to filename/"Unknown"
    /// when tags are missing.
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
                    aggression: None,
                    vibe_family: None,
                    composer: p.composer,
                    writer: p.writer,
                    vocalist: p.vocalist,
                    producer: p.producer,
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
                    aggression: None,
                    vibe_family: None,
                    composer: None,
                    writer: None,
                    vocalist: None,
                    producer: None,
                }
            }
        }
    }

    /// Performs deep 5-layer audio profiling using `AudioProfiler` if BPM or Key
    /// are not already provided in ID3 tags.
    pub fn profile_and_index(path: &Path) -> Self {
        let mut entry = Self::from_probe(path);

        let need_bpm = entry.bpm.map(|b| b < 10.0).unwrap_or(true);
        let need_key = entry
            .key
            .as_deref()
            .map(|k| k.is_empty() || k == "—" || k == "Unknown")
            .unwrap_or(true);

        if need_bpm || need_key {
            // Load Head/Tail cue zones (first 45s and last 45s) for fast JIT profiling
            if let Ok((head, _tail, dur_ms)) = AudioDecoder::load_head_tail(path, 45.0, 45.0) {
                if entry.duration_ms == 0 {
                    entry.duration_ms = dur_ms;
                }
                let profiler = AudioProfiler::new();
                let tap = profiler.profile_track(&head, &path.to_string_lossy(), AnalysisTier::FastHeadTail);

                if need_bpm && tap.global_summary.nominal_bpm > 10.0 {
                    entry.bpm = Some(tap.global_summary.nominal_bpm);
                }
                if need_key && !tap.global_summary.primary_key.is_empty() && tap.global_summary.primary_key != "Unknown" {
                    entry.key = Some(tap.global_summary.primary_key);
                }
                entry.aggression = Some(tap.global_summary.mean_aggression_index);
                entry.vibe_family = Some(tap.global_summary.macro_vibe_family);
            }
        }

        entry
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

// ---------------------------------------------------------------------------
// Persistent Library Cache Schema & Disk I/O
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryCache {
    pub version: u32,
    pub root_path: PathBuf,
    pub total_tracks: usize,
    pub last_updated_secs: u64,
    pub tracks: Vec<TrackEntry>,
}

/// Computes cache path for a given music directory.
pub fn library_cache_path(root: &Path) -> PathBuf {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("attune");
    let _ = std::fs::create_dir_all(&cache_dir);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    root.to_string_lossy().hash(&mut hasher);
    let root_hash = format!("{:012x}", hasher.finish());

    cache_dir.join(format!("library_cache_{root_hash}.json"))
}

/// Loads cached library from disk if available and valid.
pub fn load_library_cache(root: &Path) -> Option<Vec<TrackEntry>> {
    let p = library_cache_path(root);
    if !p.is_file() {
        return None;
    }

    let bytes = std::fs::read(&p).ok()?;
    let cache: LibraryCache = serde_json::from_slice(&bytes).ok()?;

    if cache.version == 1 && cache.root_path == root && !cache.tracks.is_empty() {
        log::info!(
            "Loaded {} indexed tracks from disk cache in {}",
            cache.tracks.len(),
            p.display()
        );
        Some(cache.tracks)
    } else {
        None
    }
}

/// Persists indexed library tracks to disk cache.
pub fn save_library_cache(root: &Path, tracks: &[TrackEntry]) -> Result<(), std::io::Error> {
    let p = library_cache_path(root);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let cache = LibraryCache {
        version: 1,
        root_path: root.to_path_buf(),
        total_tracks: tracks.len(),
        last_updated_secs: now,
        tracks: tracks.to_vec(),
    };

    let data = serde_json::to_vec_pretty(&cache)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Atomic write via temp file
    let tmp_path = p.with_extension("tmp");
    std::fs::write(&tmp_path, data)?;
    std::fs::rename(tmp_path, &p)?;

    log::info!(
        "Persisted {} indexed tracks with audio profiles to {}",
        tracks.len(),
        p.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Library Scanning & Deep Profiling Events
// ---------------------------------------------------------------------------

/// Events emitted during deep library indexing and profiling.
#[derive(Debug, Clone)]
pub enum IndexingProgressEvent {
    Discovered { total_files: usize },
    Progress {
        current: usize,
        total: usize,
        track_title: String,
        track_artist: String,
        bpm: Option<f32>,
        key: Option<String>,
        format: String,
    },
    Finished {
        tracks: Vec<TrackEntry>,
        lossless_count: usize,
    },
    Error(String),
}

/// Spawns a background worker that performs full 5-layer audio indexing & profiling.
/// Reuses existing cached profiles for unmodified tracks unless `reindex_all` is true.
pub fn spawn_deep_indexing(root: PathBuf, reindex_all: bool) -> Receiver<IndexingProgressEvent> {
    let (tx, rx) = mpsc::channel();

    thread::Builder::new()
        .name("attune-deep-indexer".into())
        .spawn(move || {
            if !root.is_dir() {
                let _ = tx.send(IndexingProgressEvent::Error(format!(
                    "Music directory does not exist or is not accessible: {}",
                    root.display()
                )));
                return;
            }
            let files = LibraryScanner::discover_audio_files(&root);
            let total = files.len();
            log::info!(
                "Deep Indexer: Discovered {} audio files under {}",
                total,
                root.display()
            );

            if tx.send(IndexingProgressEvent::Discovered { total_files: total }).is_err() {
                return;
            }

            if total == 0 {
                let _ = tx.send(IndexingProgressEvent::Finished {
                    tracks: Vec::new(),
                    lossless_count: 0,
                });
                return;
            }

            // Existing cache lookup map for fast incremental indexing
            let existing_map: HashMap<PathBuf, TrackEntry> = if !reindex_all {
                load_library_cache(&root)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| (t.path.clone(), t))
                    .collect()
            } else {
                HashMap::new()
            };

            let mut indexed_tracks = Vec::with_capacity(total);
            let mut lossless_count = 0usize;

            for (idx, file) in files.iter().enumerate() {
                let current_num = idx + 1;

                // Check if already completely profiled in cache
                let entry = if let Some(existing) = existing_map.get(file) {
                    let has_bpm = existing.bpm.map(|b| b > 10.0).unwrap_or(false);
                    let has_key = existing
                        .key
                        .as_deref()
                        .map(|k| !k.is_empty() && k != "—" && k != "Unknown")
                        .unwrap_or(false);

                    if has_bpm && has_key {
                        existing.clone()
                    } else {
                        TrackEntry::profile_and_index(file)
                    }
                } else {
                    TrackEntry::profile_and_index(file)
                };

                if entry.format == "FLAC" || entry.format == "WAV" || entry.format == "ALAC" {
                    lossless_count += 1;
                }

                let ev = IndexingProgressEvent::Progress {
                    current: current_num,
                    total,
                    track_title: entry.title.clone(),
                    track_artist: entry.artist.clone(),
                    bpm: entry.bpm,
                    key: entry.key.clone(),
                    format: entry.format.clone(),
                };

                if tx.send(ev).is_err() {
                    log::warn!("Deep Indexer: Receiver dropped early, saving partial cache");
                    let _ = save_library_cache(&root, &indexed_tracks);
                    return;
                }

                indexed_tracks.push(entry);
            }

            // Save completed cache to disk
            let _ = save_library_cache(&root, &indexed_tracks);

            let _ = tx.send(IndexingProgressEvent::Finished {
                tracks: indexed_tracks,
                lossless_count,
            });
        })
        .expect("failed to spawn deep indexing thread");

    rx
}

/// Progress events emitted by the fast scan worker.
pub enum ScanEvent {
    Discovered(usize),
    Track(Box<TrackEntry>),
    Finished,
}

/// Spawns a fast worker across multiple root directories with song exclusion filtering,
/// yielding existing cached tracks immediately and indexing missing files without blocking.
pub fn spawn_scan_multi(roots: Vec<PathBuf>, exclusions: Vec<String>) -> Receiver<ScanEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("attune-library-scan".into())
        .spawn(move || {
            let valid_roots: Vec<PathBuf> = roots.into_iter().filter(|r| r.is_dir()).collect();
            if valid_roots.is_empty() {
                let _ = tx.send(ScanEvent::Discovered(0));
                let _ = tx.send(ScanEvent::Finished);
                return;
            }

            let mut all_scanned_tracks: Vec<TrackEntry> = Vec::new();

            for root in &valid_roots {
                // 1. Check if cache exists for this root
                if let Some(cached_tracks) = load_library_cache(root) {
                    for t in cached_tracks {
                        if !crate::config::AppConfig::is_path_or_title_excluded(&exclusions, &t.path, Some(&t.title)) {
                            all_scanned_tracks.push(t);
                        }
                    }
                    continue;
                }

                // 2. Discover and probe files for uncached root
                let files = LibraryScanner::discover_audio_files(root);
                log::info!("Discovered {} audio files under {}", files.len(), root.display());
                let mut root_tracks = Vec::with_capacity(files.len());
                for file in files.iter() {
                    if crate::config::AppConfig::is_path_or_title_excluded(&exclusions, file, None) {
                        continue;
                    }
                    let entry = TrackEntry::profile_and_index(file);
                    if !crate::config::AppConfig::is_path_or_title_excluded(&exclusions, &entry.path, Some(&entry.title)) {
                        root_tracks.push(entry.clone());
                        all_scanned_tracks.push(entry);
                    }
                }
                let _ = save_library_cache(root, &root_tracks);
            }

            if tx.send(ScanEvent::Discovered(all_scanned_tracks.len())).is_err() {
                return;
            }

            for t in all_scanned_tracks {
                if tx.send(ScanEvent::Track(Box::new(t))).is_err() {
                    return;
                }
            }

            let _ = tx.send(ScanEvent::Finished);
        })
        .expect("failed to spawn library scan thread");
    rx
}

/// Spawns a fast worker that yields existing cached tracks immediately,
/// then indexes any missing files without blocking.
pub fn spawn_scan(root: PathBuf) -> Receiver<ScanEvent> {
    spawn_scan_multi(vec![root], Vec::new())
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
            aggression: Some(0.75),
            vibe_family: Some("HighEnergyRock".into()),
            ..Default::default()
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

    #[test]
    fn cache_serialization_roundtrip() {
        let root = PathBuf::from("/test/music");
        let tracks = vec![
            entry("Song 1", "Artist 1", "Album 1"),
            entry("Song 2", "Artist 2", "Album 2"),
        ];
        let cache = LibraryCache {
            version: 1,
            root_path: root.clone(),
            total_tracks: 2,
            last_updated_secs: 12345678,
            tracks: tracks.clone(),
        };
        let json = serde_json::to_string(&cache).unwrap();
        let loaded: LibraryCache = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.total_tracks, 2);
        assert_eq!(loaded.tracks[0].title, "Song 1");
        assert_eq!(loaded.tracks[0].bpm, Some(171.4));
        assert_eq!(loaded.tracks[0].aggression, Some(0.75));
    }
}
