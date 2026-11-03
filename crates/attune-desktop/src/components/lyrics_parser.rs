//! Synchronized LRC and plain-text lyrics parser for Attune egui.
//!
//! Parses standard timestamped LRC tags:
//!   `[mm:ss.xx] Lyric text`
//! and provides real-time active line lookup based on current playback milliseconds.

use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct LyricLine {
    pub timestamp_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
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

        // Binary search or reverse iteration for active line
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
    let mut lines = Vec::new();
    let mut is_timed = false;

    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Look for timestamp tags: [mm:ss.xx] or [mm:ss:xx]
        let mut text_start = 0;
        let mut timestamps = Vec::new();

        let bytes = trimmed.as_bytes();
        let mut idx = 0;
        while idx < bytes.len() {
            if bytes[idx] == b'[' {
                if let Some(close) = trimmed[idx..].find(']') {
                    let tag = &trimmed[idx + 1..idx + close];
                    if let Some(ms) = parse_timestamp(tag) {
                        timestamps.push(ms);
                        idx += close + 1;
                        text_start = idx;
                        continue;
                    }
                }
            }
            break;
        }

        let lyric_text = trimmed[text_start..].trim().to_string();
        if !timestamps.is_empty() {
            is_timed = true;
            for ts in timestamps {
                lines.push(LyricLine {
                    timestamp_ms: ts,
                    text: lyric_text.clone(),
                });
            }
        } else if !lyric_text.starts_with('[') {
            // Plain text line
            lines.push(LyricLine {
                timestamp_ms: 0,
                text: lyric_text,
            });
        }
    }

    lines.sort_by_key(|l| l.timestamp_ms);

    LyricsSheet {
        is_timed,
        lines,
        source_label: source_label.to_string(),
    }
}

/// Parses a timestamp tag like `01:23.45` or `01:23:450` into milliseconds.
fn parse_timestamp(tag: &str) -> Option<u64> {
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() < 2 {
        return None;
    }

    let mins: u64 = parts[0].trim().parse().ok()?;
    let sec_parts: Vec<&str> = parts[1].split('.').collect();
    let secs: u64 = sec_parts[0].trim().parse().ok()?;
    let millis: u64 = if sec_parts.len() > 1 {
        let frac = sec_parts[1].trim();
        if frac.len() == 2 {
            frac.parse::<u64>().ok()? * 10
        } else if frac.len() == 3 {
            frac.parse::<u64>().ok()?
        } else {
            0
        }
    } else {
        0
    };

    Some(mins * 60_000 + secs * 1_000 + millis)
}

/// Resolves companion `.lrc` or `.txt` file next to an audio file on disk.
pub fn resolve_companion_lyrics(audio_path: &Path) -> Option<LyricsSheet> {
    let parent = audio_path.parent()?;
    let stem = audio_path.file_stem()?.to_str()?;

    let candidates = [
        parent.join(format!("{stem}.lrc")),
        parent.join(format!("{stem}.txt")),
        parent.join("lyrics.lrc"),
    ];

    for path in &candidates {
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(path) {
                let label = format!("Local {}", path.extension().unwrap_or_default().to_string_lossy());
                return Some(parse_lrc(&content, &label));
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
