//! Runtime system font discovery for CJK / extended glyph coverage.
//!
//! egui's bundled fonts lack Japanese/Chinese/Korean glyphs, which renders many
//! track titles as tofu boxes. We do not bundle a multi-MB font; instead we ask
//! the OS for one at startup (Mandate 4: no hardcoded paths):
//!   1. `ATTUNE_UI_FONT` env var (explicit file path override)
//!   2. Linux/BSD: `fc-match` (fontconfig) for a Japanese-capable sans font
//!   3. Windows: CJK fonts under `%WINDIR%\Fonts`
//!
//! If nothing is found, the default fonts are kept and a warning is logged.

use std::path::PathBuf;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

/// Font families queried from fontconfig, in preference order.
#[cfg(not(windows))]
const FONTCONFIG_PATTERNS: &[&str] = &["sans-serif:lang=ja", "sans-serif:lang=zh-cn", "sans-serif:lang=ko"];

/// Font file names probed under `%WINDIR%\Fonts`, in preference order.
#[cfg(windows)]
const WINDOWS_CJK_FONTS: &[&str] = &["YuGothM.ttc", "meiryo.ttc", "msgothic.ttc", "msyh.ttc", "malgun.ttf"];

fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = std::env::var_os("ATTUNE_UI_FONT") {
        out.push(PathBuf::from(p));
    }

    #[cfg(not(windows))]
    for pattern in FONTCONFIG_PATTERNS {
        if let Ok(o) = std::process::Command::new("fc-match").args(["-f", "%{file}", pattern]).output() {
            if o.status.success() {
                let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !path.is_empty() {
                    out.push(PathBuf::from(path));
                }
            }
        }
    }

    #[cfg(windows)]
    if let Some(windir) = std::env::var_os("WINDIR") {
        let fonts = PathBuf::from(windir).join("Fonts");
        for f in WINDOWS_CJK_FONTS {
            out.push(fonts.join(f));
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert(p.clone()));
    out
}

/// Installs the first loadable system font as a fallback for both families.
pub fn install_fallback_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut installed = 0usize;

    for path in candidate_paths() {
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let name = format!("system-fallback-{installed}");
        // `.ttc` collections: index 0 is the regular face for all fonts listed above.
        fonts.font_data.insert(name.clone(), FontData::from_owned(bytes).into());
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push(name.clone());
        }
        log::info!("Loaded fallback UI font: {}", path.display());
        installed += 1;
        // One CJK-capable font covers the common case; a second adds resilience.
        if installed >= 2 {
            break;
        }
    }

    if installed == 0 {
        log::warn!("No system CJK font found; set ATTUNE_UI_FONT=/path/to/font.ttf to fix missing glyphs");
        return;
    }
    ctx.set_fonts(fonts);
}
