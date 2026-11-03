//! Desktop environment resolution for Attune egui.
//!
//! Mandate 4 (no hardcoded values): every path is resolved from environment
//! variables first, then OS-standard directories, and finally left unset so the
//! UI can ask the user explicitly instead of silently guessing.

use std::path::PathBuf;

/// Resolution source for the music library directory (shown in the UI for transparency).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MusicDirSource {
    /// `ATTUNE_MUSIC_DIR` environment variable.
    Environment,
    /// Directory previously chosen by the user and persisted by eframe.
    Persisted,
    /// OS audio directory (XDG_MUSIC_DIR on Linux, `%USERPROFILE%\Music` on Windows).
    OsDefault,
    /// Nothing could be resolved; the user must pick a folder.
    Unresolved,
}

impl MusicDirSource {
    pub fn label(&self) -> &'static str {
        match self {
            MusicDirSource::Environment => "ATTUNE_MUSIC_DIR",
            MusicDirSource::Persisted => "saved preference",
            MusicDirSource::OsDefault => "OS music folder",
            MusicDirSource::Unresolved => "not set",
        }
    }
}

/// Resolve the music directory in priority order: env → persisted → OS default.
pub fn resolve_music_dir(persisted: Option<PathBuf>) -> (Option<PathBuf>, MusicDirSource) {
    if let Some(dir) = std::env::var_os("ATTUNE_MUSIC_DIR").map(PathBuf::from) {
        if dir.is_dir() {
            return (Some(dir), MusicDirSource::Environment);
        }
        log::warn!("ATTUNE_MUSIC_DIR is set but is not a directory: {}", dir.display());
    }
    if let Some(dir) = persisted.filter(|d| d.is_dir()) {
        return (Some(dir), MusicDirSource::Persisted);
    }
    if let Some(dir) = dirs::audio_dir().filter(|d| d.is_dir()) {
        return (Some(dir), MusicDirSource::OsDefault);
    }
    (None, MusicDirSource::Unresolved)
}

/// Ensure the core engine writes extracted artwork into a per-user cache directory
/// rather than the shared system temp dir. Respects an existing `ATTUNE_CACHE_DIR`.
///
/// Must be called before any worker threads are spawned.
pub fn ensure_cache_dir_env() -> Option<PathBuf> {
    if let Some(existing) = std::env::var_os("ATTUNE_CACHE_DIR") {
        return Some(PathBuf::from(existing));
    }
    let dir = dirs::cache_dir()?.join("attune");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("Could not create cache directory {}: {}", dir.display(), e);
        return None;
    }
    // SAFETY (edition 2021): called once at startup on the main thread, before
    // any other threads exist, so no concurrent environment access can race.
    std::env::set_var("ATTUNE_CACHE_DIR", &dir);
    Some(dir)
}

/// Application version: injected at build time via `ATTUNE_VERSION`
/// (set by `tools/builder.py` / CI), falling back to the crate version.
pub fn app_version() -> &'static str {
    option_env!("ATTUNE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_dir_is_ignored_when_missing_on_disk() {
        // A persisted path that no longer exists must never be returned.
        let (dir, src) = resolve_music_dir(Some(PathBuf::from("/nonexistent/attune/music")));
        if std::env::var_os("ATTUNE_MUSIC_DIR").is_none() {
            assert_ne!(src, MusicDirSource::Persisted);
        }
        if let Some(d) = dir {
            assert!(d.is_dir());
        }
    }

    #[test]
    fn persisted_dir_is_used_when_valid() {
        if std::env::var_os("ATTUNE_MUSIC_DIR").is_some() {
            return; // env override legitimately wins
        }
        let tmp = std::env::temp_dir();
        let (dir, src) = resolve_music_dir(Some(tmp.clone()));
        assert_eq!(src, MusicDirSource::Persisted);
        assert_eq!(dir, Some(tmp));
    }

    #[test]
    fn app_version_is_non_empty() {
        assert!(!app_version().is_empty());
    }
}
