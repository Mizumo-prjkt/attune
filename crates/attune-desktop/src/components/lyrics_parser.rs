//! Synchronized LRC and plain-text lyrics parser for Attune.
//!
//! Parses standard timestamped LRC tags:
//!   `[mm:ss.xx] Lyric text`
//! and provides real-time active line lookup based on current playback milliseconds.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

static PENDING_LYRICS_OVERRIDE: Mutex<Option<(PathBuf, LyricsSheet)>> = Mutex::new(None);

/// Broadcasts an updated lyrics sheet for a track so the main player and UI update immediately.
pub fn notify_lyrics_updated(path: PathBuf, sheet: LyricsSheet) {
    let mut guard = PENDING_LYRICS_OVERRIDE.lock().unwrap();
    *guard = Some((path, sheet));
}

/// Consumes pending lyrics update for the given track path if one has been posted.
pub fn poll_lyrics_update(current_path: &Path) -> Option<LyricsSheet> {
    let mut guard = PENDING_LYRICS_OVERRIDE.lock().unwrap();
    if let Some((ref p, _)) = *guard {
        if p == current_path {
            return guard.take().map(|(_, s)| s);
        }
    }
    None
}

/// Saves lyrics content to disk next to the audio file or falls back gracefully
/// to the per-user attune lyrics cache directory.
pub fn save_lyrics_to_disk(audio_path: &Path, content: &str, preferred_ext: &str) -> Result<PathBuf, String> {
    let stem = audio_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "lyrics".to_string());
    let ext = if preferred_ext.starts_with('.') {
        &preferred_ext[1..]
    } else {
        preferred_ext
    };

    // 1. Try writing directly alongside audio file
    if let Some(parent) = audio_path.parent() {
        let target = parent.join(format!("{stem}.{ext}"));
        if std::fs::write(&target, content).is_ok() {
            return Ok(target);
        }
    }

    // 2. Fallback to per-user cache directory
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("attune-desktop")
        .join("lyrics");
    let _ = std::fs::create_dir_all(&cache_dir);
    let target = cache_dir.join(format!("{stem}.{ext}"));
    std::fs::write(&target, content).map_err(|e| format!("Failed to write lyrics: {e}"))?;
    Ok(target)
}

#[derive(Debug, Clone, PartialEq)]
pub struct LyricLine {
    pub timestamp_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LyricsSheet {
    pub is_timed: bool,
    pub lines: Vec<LyricLine>,
    pub source_label: String,
}

impl LyricsSheet {
    pub fn empty() -> Self {
        Self {
            is_timed: false,
            lines: Vec::new(),
            source_label: "No lyrics available".into(),
        }
    }

    /// Finds the index of the currently active lyric line.
    pub fn active_line_index(&self, current_pos_ms: u64) -> Option<usize> {
        if !self.is_timed || self.lines.is_empty() {
            return None;
        }

        let mut active = None;
        for (i, line) in self.lines.iter().enumerate() {
            if line.timestamp_ms <= current_pos_ms {
                active = Some(i);
            } else {
                break;
            }
        }
        active
    }

    /// Current single-line snippet for Normal Square Big skin.
    pub fn current_snippet(&self, current_pos_ms: u64) -> Option<&str> {
        self.active_line_index(current_pos_ms)
            .and_then(|idx| self.lines.get(idx))
            .map(|l| l.text.as_str())
            .filter(|t| !t.trim().is_empty())
    }
}

/// Parses an LRC formatted text string.
pub fn parse_lrc(content: &str, source_label: &str) -> LyricsSheet {
    crate::components::lyrics_validator::parse_verified_lyrics(content, source_label)
}

/// Resolves companion `.lrc`, `.elrc`, `.ttml`, `.yrc`, or `.txt` file next to an audio file on disk,
/// with fallback to the per-user lyrics cache directory.
pub fn resolve_companion_lyrics(audio_path: &Path) -> Option<LyricsSheet> {
    let parent = audio_path.parent()?;
    let stem = audio_path.file_stem()?.to_str()?;

    let mut candidates = vec![
        parent.join(format!("{stem}.elrc")),
        parent.join(format!("{stem}.ttml")),
        parent.join(format!("{stem}.yrc")),
        parent.join(format!("{stem}.lrc")),
        parent.join(format!("{stem}.txt")),
        parent.join("lyrics.lrc"),
    ];

    // Also check per-user lyrics cache
    if let Some(cache_dir) = dirs::cache_dir() {
        let app_cache = cache_dir.join("attune-desktop").join("lyrics");
        candidates.push(app_cache.join(format!("{stem}.elrc")));
        candidates.push(app_cache.join(format!("{stem}.ttml")));
        candidates.push(app_cache.join(format!("{stem}.lrc")));
        candidates.push(app_cache.join(format!("{stem}.txt")));
    }

    for path in &candidates {
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(path) {
                let validation = crate::components::lyrics_validator::validate_and_classify_lyrics(
                    &content,
                    path.file_name().and_then(|n| n.to_str()),
                );
                if validation.is_safe {
                    let label = format!("Local {}", path.extension().unwrap_or_default().to_string_lossy());
                    return Some(crate::components::lyrics_validator::parse_verified_lyrics(&content, &label));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lrc_standard() {
        let raw = "[00:12.50] Hello world\n[00:15.00] This is Attune lyrics";
        let sheet = parse_lrc(raw, "Test");
        assert!(sheet.is_timed);
        assert_eq!(sheet.lines.len(), 2);
        assert_eq!(sheet.lines[0].timestamp_ms, 12_500);
        assert_eq!(sheet.lines[0].text, "Hello world");
        assert_eq!(sheet.lines[1].timestamp_ms, 15_000);
    }

    #[test]
    fn test_active_line_lookup() {
        let raw = "[00:10.00] Line 1\n[00:20.00] Line 2\n[00:30.00] Line 3";
        let sheet = parse_lrc(raw, "Test");
        assert_eq!(sheet.active_line_index(5_000), None);
        assert_eq!(sheet.active_line_index(12_000), Some(0));
        assert_eq!(sheet.active_line_index(25_000), Some(1));
        assert_eq!(sheet.active_line_index(40_000), Some(2));
    }
}
