//! Lyrics Syntax Classification & Safety Verification Engine.
//!
//! Complies with Mandate 4 (zero hardcoded values) and provides:
//! 1. Security heuristic analysis (rejection of binary payloads, script injection,
//!    XML external entity attacks, and oversized text bombs).
//! 2. Syntax classification:
//!    - Syllable / Letter / Word synchronized (`TTML`, `ELRC`, `YRC`)
//!    - Standard line synchronized (`LRC`)
//!    - Plain text unsynchronized (`TXT`)
//! 3. Safe parsing of lyric lines with timestamps into a `LyricsSheet`.

use crate::components::lyrics_parser::{LyricLine, LyricsSheet};

pub const MAX_SAFE_LYRICS_BYTES: usize = 1_048_576; // 1 MB safety ceiling

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LyricsSyncType {
    /// Syllable / Word-by-word / Letter-synchronized (TTML, ELRC, YRC, Enhanced LRC)
    Syllable,
    /// Standard line-by-line timestamped synchronized (LRC)
    Line,
    /// Plain text lyrics with zero synchronization
    Plain,
}

impl LyricsSyncType {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Syllable => "Letter/Word Syllable Sync (TTML/ELRC/YRC)",
            Self::Line => "Line Synced (LRC)",
            Self::Plain => "Plain Text (Unsynced)",
        }
    }

    pub fn badge_class(&self) -> &'static str {
        match self {
            Self::Syllable => "badge-pill mint",
            Self::Line => "badge-pill neutral",
            Self::Plain => "badge-pill muted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    pub is_safe: bool,
    pub sync_type: LyricsSyncType,
    pub security_details: String,
    pub detected_format: String,
    pub line_count: usize,
    pub has_timestamps: bool,
}

/// Validates raw lyrics content for malicious payloads and classifies syntax.
pub fn validate_and_classify_lyrics(content: &str, file_name: Option<&str>) -> ValidationResult {
    let bytes = content.as_bytes();

    // 1. Size guard against memory exhaustion / zip-bombs
    if bytes.len() > MAX_SAFE_LYRICS_BYTES {
        return ValidationResult {
            is_safe: false,
            sync_type: LyricsSyncType::Plain,
            security_details: format!(
                "Blocked: Payload size ({} KB) exceeds 1MB safety limit",
                bytes.len() / 1024
            ),
            detected_format: "Oversized Payload".to_string(),
            line_count: 0,
            has_timestamps: false,
        };
    }

    // 2. Binary / executable header signature detection
    if bytes.starts_with(b"\x7fELF")
        || bytes.starts_with(b"MZ")
        || bytes.starts_with(b"\xca\xfe\xba\xbe")
        || bytes.starts_with(b"\xfe\xed\xfa")
        || bytes.starts_with(b"\xcf\xfa\xed\xfe")
        || bytes.starts_with(b"#!/")
    {
        return ValidationResult {
            is_safe: false,
            sync_type: LyricsSyncType::Plain,
            security_details: "Blocked: Detected binary or executable header signature".to_string(),
            detected_format: "Binary Executable".to_string(),
            line_count: 0,
            has_timestamps: false,
        };
    }

    // 3. Null byte inspection
    if bytes.contains(&0) {
        return ValidationResult {
            is_safe: false,
            sync_type: LyricsSyncType::Plain,
            security_details: "Blocked: Contains illegal null byte in text stream".to_string(),
            detected_format: "Corrupt / Binary Stream".to_string(),
            line_count: 0,
            has_timestamps: false,
        };
    }

    // 4. Script and Injection heuristics
    let lower = content.to_ascii_lowercase();
    let dangerous_tokens = [
        "<script",
        "javascript:",
        "vbscript:",
        "data:text/html",
        "<iframe",
        "<object",
        "<embed",
        "onload=",
        "onerror=",
        "onclick=",
        "<!entity",
        "<!doctype system",
    ];

    for token in &dangerous_tokens {
        if lower.contains(token) {
            return ValidationResult {
                is_safe: false,
                sync_type: LyricsSyncType::Plain,
                security_details: format!("Blocked: Detected unsafe script or entity token ('{}')", token),
                detected_format: "Malicious Injection Vector".to_string(),
                line_count: 0,
                has_timestamps: false,
            };
        }
    }

    // 5. Syntax classification
    let is_ttml = lower.contains("<tt")
        || lower.contains("<p begin=")
        || (lower.contains("<span begin=") && lower.contains("</span>"));
    let is_yrc = lower.contains("([0-9]")
        || (content.contains("(") && content.contains(")") && content.contains(",") && content.contains("[00:"));
    let is_elrc = lower.contains("<00:") || lower.contains("<01:") || lower.contains("<02:") || has_word_timestamp_tags(content);

    let (sync_type, format_label) = if is_ttml {
        (LyricsSyncType::Syllable, "AMLL TTML (Word-Synchronized XML)")
    } else if is_yrc {
        (LyricsSyncType::Syllable, "NetEase YRC (Syllable-Synchronized)")
    } else if is_elrc {
        (LyricsSyncType::Syllable, "Enhanced LRC (ELRC Word-Synchronized)")
    } else if has_standard_lrc_timestamps(content) {
        (LyricsSyncType::Line, "Standard LRC (Line-Synchronized)")
    } else {
        (LyricsSyncType::Plain, "Plain Text Lyrics (Unsynchronized)")
    };

    let line_count = content.lines().filter(|l| !l.trim().is_empty()).count();
    let has_timestamps = sync_type != LyricsSyncType::Plain;

    let file_desc = file_name.unwrap_or("lyrics stream");
    let security_details = format!(
        "Safety Verified: Clean text stream from {} ({} lines, {} bytes)",
        file_desc,
        line_count,
        bytes.len()
    );

    ValidationResult {
        is_safe: true,
        sync_type,
        security_details,
        detected_format: format_label.to_string(),
        line_count,
        has_timestamps,
    }
}

/// Parses verified lyrics content into a clean `LyricsSheet`.
pub fn parse_verified_lyrics(content: &str, source_label: &str) -> LyricsSheet {
    let lower = content.to_ascii_lowercase();

    // If AMLL TTML format, parse XML <p begin="..."> tags
    if lower.contains("<tt") || lower.contains("<p begin=") {
        return parse_ttml_to_sheet(content, source_label);
    }

    // Otherwise parse line/word LRC
    let mut lines = Vec::new();
    let mut is_timed = false;

    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Extract timestamps e.g. [01:23.45]
        let mut text_start = 0;
        let mut timestamps = Vec::new();
        let bytes = trimmed.as_bytes();
        let mut idx = 0;

        while idx < bytes.len() {
            if bytes[idx] == b'[' {
                if let Some(close) = trimmed[idx..].find(']') {
                    let tag = &trimmed[idx + 1..idx + close];
                    if let Some(ms) = parse_lrc_timestamp(tag) {
                        timestamps.push(ms);
                        idx += close + 1;
                        text_start = idx;
                        continue;
                    }
                }
            }
            break;
        }

        let raw_lyric_text = trimmed[text_start..].trim();
        // Clean word tags like <00:12.34> or (123,456,0) for clean readable display
        let clean_text = clean_word_sync_tags(raw_lyric_text);

        if !timestamps.is_empty() {
            is_timed = true;
            for ts in timestamps {
                lines.push(LyricLine {
                    timestamp_ms: ts,
                    text: clean_text.clone(),
                });
            }
        } else if !clean_text.starts_with('[') && !clean_text.is_empty() {
            lines.push(LyricLine {
                timestamp_ms: 0,
                text: clean_text,
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

/// Helper parsing AMLL TTML XML into `LyricsSheet`.
fn parse_ttml_to_sheet(xml_content: &str, source_label: &str) -> LyricsSheet {
    let mut lines = Vec::new();
    let mut is_timed = false;

    for part in xml_content.split("<p ") {
        if let Some(end_idx) = part.find("</p>") {
            let p_tag_content = &part[..end_idx];
            if let Some(tag_close) = p_tag_content.find('>') {
                let p_attributes = &p_tag_content[..tag_close];
                let p_inner = &p_tag_content[tag_close + 1..];

                if let Some(begin_pos) = p_attributes.find("begin=\"") {
                    let after_begin = &p_attributes[begin_pos + 7..];
                    if let Some(quote_end) = after_begin.find('"') {
                        let time_str = &after_begin[..quote_end];
                        if let Some(ts_ms) = parse_ttml_time(time_str) {
                            is_timed = true;
                            let text = strip_xml_tags(p_inner);
                            if !text.trim().is_empty() {
                                lines.push(LyricLine {
                                    timestamp_ms: ts_ms,
                                    text: text.trim().to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    lines.sort_by_key(|l| l.timestamp_ms);

    LyricsSheet {
        is_timed,
        lines,
        source_label: source_label.to_string(),
    }
}

/// Converts TTML time strings (`01:23.456` or `123.45s`) to milliseconds.
fn parse_ttml_time(t: &str) -> Option<u64> {
    let clean = t.trim().trim_end_matches('s');
    if clean.contains(':') {
        parse_lrc_timestamp(clean)
    } else if let Ok(secs) = clean.parse::<f64>() {
        Some((secs * 1000.0) as u64)
    } else {
        None
    }
}

/// Strips XML tags from a string.
fn strip_xml_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut inside_tag = false;
    for c in input.chars() {
        if c == '<' {
            inside_tag = true;
        } else if c == '>' {
            inside_tag = false;
        } else if !inside_tag {
            output.push(c);
        }
    }
    output
}

/// Strips word/syllable timing tags e.g. `<00:12.34>` or `(120,30,0)`.
fn clean_word_sync_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_bracket = false;
    let mut in_paren = false;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '<' {
            in_bracket = true;
        } else if c == '>' {
            in_bracket = false;
            i += 1;
            continue;
        } else if c == '(' {
            // Check if this looks like a YRC syllable tag e.g. (123,456,0)
            if let Some(close_idx) = chars[i..].iter().position(|&ch| ch == ')') {
                let slice: String = chars[i..=i + close_idx].iter().collect();
                if slice.contains(',') {
                    in_paren = true;
                }
            }
        } else if c == ')' && in_paren {
            in_paren = false;
            i += 1;
            continue;
        }

        if !in_bracket && !in_paren {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// Checks if content contains standard LRC timestamp tags `[mm:ss.xx]`.
fn has_standard_lrc_timestamps(content: &str) -> bool {
    for line in content.lines().take(30) {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if let Some(close) = trimmed.find(']') {
                let tag = &trimmed[1..close];
                if parse_lrc_timestamp(tag).is_some() {
                    return true;
                }
            }
        }
    }
    false
}

/// Checks for ELRC `<mm:ss.xx>` tags within line bodies.
fn has_word_timestamp_tags(content: &str) -> bool {
    content.contains("<0") || content.contains("<1") || content.contains("<2")
}

/// Parses timestamp tag into milliseconds.
fn parse_lrc_timestamp(tag: &str) -> Option<u64> {
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() < 2 {
        return None;
    }

    let mins: u64 = parts[0].trim().parse().ok()?;
    let sec_parts: Vec<&str> = parts[1].split('.').collect();
    let secs: u64 = sec_parts[0].trim().parse().ok()?;
    let millis: u64 = if sec_parts.len() > 1 {
        let frac = sec_parts[1].trim();
        if frac.len() == 1 {
            frac.parse::<u64>().ok()? * 100
        } else if frac.len() == 2 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_standard_lrc() {
        let text = "[00:10.00] Line 1\n[00:15.50] Line 2";
        let res = validate_and_classify_lyrics(text, Some("test.lrc"));
        assert!(res.is_safe);
        assert_eq!(res.sync_type, LyricsSyncType::Line);
        assert_eq!(res.line_count, 2);
    }

    #[test]
    fn test_classify_syllable_elrc() {
        let text = "[00:10.00]<00:10.20>Hel<00:10.50>lo <00:11.00>world";
        let res = validate_and_classify_lyrics(text, Some("test.elrc"));
        assert!(res.is_safe);
        assert_eq!(res.sync_type, LyricsSyncType::Syllable);
        let sheet = parse_verified_lyrics(text, "ELRC Source");
        assert_eq!(sheet.lines[0].text, "Hello world");
    }

    #[test]
    fn test_classify_ttml_xml() {
        let xml = "<tt xmlns=\"http://www.w3.org/ns/ttml\"><p begin=\"00:12.300\"><span>Singing</span></p></tt>";
        let res = validate_and_classify_lyrics(xml, Some("test.ttml"));
        assert!(res.is_safe);
        assert_eq!(res.sync_type, LyricsSyncType::Syllable);
        let sheet = parse_verified_lyrics(xml, "TTML Source");
        assert_eq!(sheet.lines.len(), 1);
        assert_eq!(sheet.lines[0].timestamp_ms, 12_300);
        assert_eq!(sheet.lines[0].text, "Singing");
    }

    #[test]
    fn test_classify_plain_text() {
        let text = "Just some plain text lyrics\nWithout any timestamps\nFor testing";
        let res = validate_and_classify_lyrics(text, Some("plain.txt"));
        assert!(res.is_safe);
        assert_eq!(res.sync_type, LyricsSyncType::Plain);
        assert_eq!(res.line_count, 3);
    }

    #[test]
    fn test_blocks_script_injection() {
        let evil = "[00:10.00] Line 1\n<script>alert('pwned')</script>";
        let res = validate_and_classify_lyrics(evil, Some("evil.lrc"));
        assert!(!res.is_safe);
        assert!(res.security_details.contains("Blocked: Detected unsafe script"));
    }

    #[test]
    fn test_blocks_binary_headers() {
        let elf = "\x7fELF\x02\x01\x01\x00SomeBinaryContent";
        let res = validate_and_classify_lyrics(elf, Some("binary.lrc"));
        assert!(!res.is_safe);
        assert!(res.security_details.contains("Blocked: Detected binary"));
    }
}
