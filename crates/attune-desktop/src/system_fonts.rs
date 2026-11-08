//! System font discovery and CSS font-family resolution for Attune Desktop.
//!
//! Provides cross-platform font enumeration, deduplication, and exclusion of
//! Attune's default bundled fonts from the discovered system fonts list.

use std::collections::BTreeSet;
use std::sync::OnceLock;

static CACHED_SYSTEM_FONTS: OnceLock<Vec<String>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttuneFontOption {
    pub id: &'static str,
    pub label: &'static str,
}

pub const ATTUNE_DEFAULT_FONTS: &[AttuneFontOption] = &[
    AttuneFontOption {
        id: "system",
        label: "System Default (Adaptive)",
    },
    AttuneFontOption {
        id: "Plus Jakarta Sans",
        label: "Plus Jakarta Sans (Attune Body and Lyrics)",
    },
    AttuneFontOption {
        id: "Sora",
        label: "Sora (Attune Display)",
    },
    AttuneFontOption {
        id: "Inter",
        label: "Inter (Clean Geometric)",
    },
    AttuneFontOption {
        id: "Roboto",
        label: "Roboto (Modern Grotesque)",
    },
];

/// Checks if a font name corresponds to one of Attune's default or bundled fonts.
///
/// This prevents our fonts (e.g. Plus Jakarta Sans, Sora, Inter, Roboto) from
/// appearing duplicated in the "System Fonts" dropdown group when they are also
/// installed on the user's host OS.
pub fn is_our_font(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    if lower.is_empty()
        || lower == "system"
        || lower == "system default"
        || lower == "default"
    {
        return true;
    }

    // Plus Jakarta Sans family variants
    if lower == "plus jakarta sans" || lower.starts_with("plus jakarta sans") {
        return true;
    }

    // Sora family variants (carefully avoiding "Noto Sans Sora Sompeng" which is an Indian script)
    if lower == "sora" || (lower.starts_with("sora ") && !lower.contains("sompeng")) {
        return true;
    }

    // Inter family variants
    if lower == "inter"
        || lower == "inter display"
        || lower == "inter tight"
        || lower == "inter variable font"
        || lower.starts_with("inter ")
    {
        return true;
    }

    // Roboto family variants
    if lower == "roboto" || lower.starts_with("roboto ") {
        return true;
    }

    false
}

/// Resolves a font face identifier into a valid CSS `font-family` property string.
pub fn resolve_font_family_css(font_name: &str) -> String {
    let trimmed = font_name.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("system")
        || trimmed.eq_ignore_ascii_case("system default")
    {
        "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', 'Noto Sans', 'Noto Sans CJK JP', sans-serif".to_string()
    } else {
        let escaped = trimmed.replace('\'', "\\'");
        format!("'{}', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Noto Sans', sans-serif", escaped)
    }
}

/// Sanitizes text for GTK / WebKitGTK dropdown menu items and labels by escaping bare ampersands
/// as `&amp;` to prevent Pango markup parsing warnings (e.g. "Entity did not end with a semicolon").
pub fn pango_escape(text: &str) -> String {
    if text.contains('&') && !text.contains("&amp;") {
        text.replace('&', "&amp;")
    } else {
        text.to_string()
    }
}

/// Discovers and returns a cached list of system font family names, excluding Attune's default fonts.
pub fn get_available_system_fonts() -> &'static [String] {
    CACHED_SYSTEM_FONTS.get_or_init(discover_system_fonts)
}

fn discover_system_fonts() -> Vec<String> {
    let mut fonts = BTreeSet::new();

    #[cfg(unix)]
    {
        if let Ok(output) = std::process::Command::new("fc-list")
            .arg(":")
            .arg("family")
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    // fc-list often prints comma-separated aliases, e.g. "DejaVu Sans,DejaVu Sans Light"
                    // The first item is the primary family name
                    if let Some(family) = line.split(',').next() {
                        let trimmed = family.trim();
                        if !trimmed.is_empty()
                            && !trimmed.starts_with('@') // Vertical text layout font variants
                            && !trimmed.starts_with('.')
                            && !is_our_font(trimmed)
                        {
                            fonts.insert(trimmed.to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // Try Windows registry query for registered fonts
        if let Ok(output) = std::process::Command::new("reg")
            .args(["query", "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split("    ").filter(|s| !s.trim().is_empty()).collect();
                    if let Some(name) = parts.first() {
                        let clean_name = name.trim();
                        // Strip (TrueType), (OpenType), etc.
                        let base_name = if let Some(idx) = clean_name.find('(') {
                            clean_name[..idx].trim()
                        } else {
                            clean_name
                        };
                        // Strip " & ..." compound font declarations
                        let primary = if let Some(idx) = base_name.find('&') {
                            base_name[..idx].trim()
                        } else {
                            base_name
                        };
                        if !primary.is_empty() && !is_our_font(primary) {
                            fonts.insert(primary.to_string());
                        }
                    }
                }
            }
        }
    }

    // If font enumeration returned empty (e.g. headless container or stripped environment),
    // supply a rich fallback list of popular cross-platform fonts
    if fonts.is_empty() {
        for fallback in &[
            "Adwaita Sans",
            "Arial",
            "Calibri",
            "Cantarell",
            "Comic Sans MS",
            "Consolas",
            "Courier New",
            "DejaVu Sans",
            "DejaVu Serif",
            "Georgia",
            "Liberation Sans",
            "Liberation Serif",
            "Noto Sans",
            "Noto Sans CJK JP",
            "Noto Serif",
            "Segoe UI",
            "Times New Roman",
            "Trebuchet MS",
            "Ubuntu",
            "Verdana",
        ] {
            if !is_our_font(fallback) {
                fonts.insert(fallback.to_string());
            }
        }
    }

    fonts.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_our_font_exclusions() {
        assert!(is_our_font("system"));
        assert!(is_our_font("System Default"));
        assert!(is_our_font("Plus Jakarta Sans"));
        assert!(is_our_font("Plus Jakarta Sans Medium"));
        assert!(is_our_font("Sora"));
        assert!(is_our_font("Sora Bold"));
        assert!(is_our_font("Inter"));
        assert!(is_our_font("Inter Display"));
        assert!(is_our_font("Roboto"));
        assert!(is_our_font("Roboto Medium"));

        // Must NOT exclude non-Attune fonts containing "sora" or "sans"
        assert!(!is_our_font("Noto Sans Sora Sompeng"));
        assert!(!is_our_font("Adwaita Sans"));
        assert!(!is_our_font("DejaVu Sans"));
        assert!(!is_our_font("Noto Sans CJK JP"));
    }

    #[test]
    fn test_resolve_font_family_css() {
        let sys_css = resolve_font_family_css("system");
        assert!(sys_css.contains("system-ui"));

        let jakarta_css = resolve_font_family_css("Plus Jakarta Sans");
        assert!(jakarta_css.starts_with("'Plus Jakarta Sans'"));

        let dejavu_css = resolve_font_family_css("DejaVu Sans");
        assert!(dejavu_css.starts_with("'DejaVu Sans'"));
    }

    #[test]
    fn test_pango_escape_ampersands() {
        assert_eq!(pango_escape("Default and Attune Fonts"), "Default and Attune Fonts");
        assert_eq!(pango_escape("Default & Attune Fonts"), "Default &amp; Attune Fonts");
        assert_eq!(pango_escape("B&H Lucida"), "B&amp;H Lucida");
        assert_eq!(pango_escape("Already &amp; Escaped"), "Already &amp; Escaped");
    }

    #[test]
    fn test_system_fonts_do_not_contain_our_fonts() {
        let system_fonts = get_available_system_fonts();
        assert!(!system_fonts.is_empty());
        for f in system_fonts {
            assert!(!is_our_font(f), "System font list should not contain our font: {}", f);
        }
    }
}
