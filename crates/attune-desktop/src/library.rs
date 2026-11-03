//! Background library scanning: discovers audio files and probes their tags
//! without ever blocking the egui render thread.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use attune_core::{AudioDecoder, LibraryScanner};

/// A single playable track in the desktop library.
#[derive(Debug, Clone)]
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
    pub bit_depth: u32,
    pub artwork_path: Option<PathBuf>,
}

impl TrackEntry {
    /// Builds an entry from a probed file; falls back to filename/"Unknown" only
    /// when tags are genuinely missing (never fabricated BPM/duration).
    fn from_probe(path: &Path) -> Self {
        let file_stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        match AudioDecoder::probe_metadata(path) {
            Ok(p) => TrackEntry {
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
                artwork_path: p.artwork_path.map(PathBuf::from),
            },
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
pub fn spawn_scan(root: PathBuf, ctx: eframe::egui::Context) -> Receiver<ScanEvent> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("attune-library-scan".into())
        .spawn(move || {
            let files = LibraryScanner::discover_audio_files(&root);
            log::info!("Discovered {} audio files under {}", files.len(), root.display());
            if tx.send(ScanEvent::Discovered(files.len())).is_err() {
                return;
            }
            ctx.request_repaint();
            for (i, file) in files.iter().enumerate() {
                if tx.send(ScanEvent::Track(Box::new(TrackEntry::from_probe(file)))).is_err() {
                    return; // UI dropped the receiver (rescan started) — stop quietly.
                }
                if i % 16 == 0 {
                    ctx.request_repaint();
                }
            }
            let _ = tx.send(ScanEvent::Finished);
            ctx.request_repaint();
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
