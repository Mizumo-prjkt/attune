//! Desktop environment resolution and configuration for Attune.
//!
//! Mandate 4 (no hardcoded values): every path is resolved from environment
//! variables first, then user configuration, and finally OS-standard directories.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Resolution source for the music library directory (shown in the UI for transparency).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MusicDirSource {
    /// `ATTUNE_MUSIC_DIR` environment variable.
    Environment,
    /// Directory previously chosen by the user and persisted in config.
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

/// Persistent user configuration for Attune Desktop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub setup_completed: bool,
    #[serde(default)]
    pub music_dir: Option<PathBuf>,
    #[serde(default)]
    pub extra_music_dirs: Vec<PathBuf>,
    #[serde(default)]
    pub song_exclusions: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            setup_completed: false,
            music_dir: None,
            extra_music_dirs: Vec::new(),
            song_exclusions: Vec::new(),
        }
    }
}

impl AppConfig {
    /// Returns all configured music directories: primary `music_dir` followed by any `extra_music_dirs`.
    pub fn all_music_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(ref d) = self.music_dir {
            if d.is_dir() && !dirs.contains(d) {
                dirs.push(d.clone());
            }
        }
        for d in &self.extra_music_dirs {
            if d.is_dir() && !dirs.contains(d) {
                dirs.push(d.clone());
            }
        }
        dirs
    }

    /// Checks whether a track path or title is excluded under the exclusion index.
    pub fn is_path_or_title_excluded(exclusions: &[String], path: &std::path::Path, title: Option<&str>) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        let file_name = path.file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let title_lower = title.map(|t| t.to_lowercase());

        for exc in exclusions {
            let exc_clean = exc.trim().to_lowercase();
            if exc_clean.is_empty() {
                continue;
            }
            if path_str.contains(&exc_clean)
                || file_name == exc_clean
                || title_lower.as_deref() == Some(&exc_clean)
            {
                return true;
            }
        }
        false
    }

    pub fn is_excluded(&self, path: &std::path::Path, title: Option<&str>) -> bool {
        Self::is_path_or_title_excluded(&self.song_exclusions, path, title)
    }
}

/// Adds an exclusion to the song exclusion index and persists to config.
pub fn add_song_exclusion(pattern: String) {
    let mut cfg = load_config();
    let trimmed = pattern.trim().to_string();
    if !trimmed.is_empty() && !cfg.song_exclusions.iter().any(|x| x.eq_ignore_ascii_case(&trimmed)) {
        cfg.song_exclusions.push(trimmed);
        save_config(&cfg);
    }
}

/// Removes an exclusion from the song exclusion index and persists to config.
pub fn remove_song_exclusion(pattern: &str) {
    let mut cfg = load_config();
    cfg.song_exclusions.retain(|x| !x.eq_ignore_ascii_case(pattern));
    save_config(&cfg);
}

/// Adds a music directory and persists to config.
pub fn add_music_dir(dir: PathBuf) {
    if !dir.is_dir() { return; }
    let mut cfg = load_config();
    if cfg.music_dir.is_none() {
        cfg.music_dir = Some(dir);
    } else if cfg.music_dir.as_ref() != Some(&dir) && !cfg.extra_music_dirs.contains(&dir) {
        cfg.extra_music_dirs.push(dir);
    }
    save_config(&cfg);
}

/// Removes a music directory and persists to config.
pub fn remove_music_dir(dir: &std::path::Path) {
    let mut cfg = load_config();
    if cfg.music_dir.as_deref() == Some(dir) {
        cfg.music_dir = if !cfg.extra_music_dirs.is_empty() {
            Some(cfg.extra_music_dirs.remove(0))
        } else {
            None
        };
    } else {
        cfg.extra_music_dirs.retain(|d| d != dir);
    }
    save_config(&cfg);
}

/// Returns the configuration file path (~/.config/attune/config.json).
pub fn config_file_path() -> Option<PathBuf> {
    let dir = dirs::config_dir()?.join("attune");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("config.json"))
}

/// Loads the persisted configuration or returns defaults.
pub fn load_config() -> AppConfig {
    if let Some(path) = config_file_path() {
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str(&content) {
                    return cfg;
                }
            }
        }
    }
    AppConfig::default()
}

/// Saves the configuration to disk.
pub fn save_config(cfg: &AppConfig) {
    if let Some(path) = config_file_path() {
        if let Ok(content) = serde_json::to_string_pretty(cfg) {
            let _ = std::fs::write(path, content);
        }
    }
}

/// Checks if initial setup has been completed.
pub fn is_setup_completed() -> bool {
    load_config().setup_completed
}

/// Marks setup as completed and persists chosen music directory.
pub fn set_setup_completed(completed: bool, music_dir: Option<PathBuf>) {
    let mut cfg = load_config();
    cfg.setup_completed = completed;
    if let Some(dir) = music_dir {
        cfg.music_dir = Some(dir);
    }
    save_config(&cfg);
}

/// Resolve the music directory in priority order: env → persisted arg → saved config → OS default.
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
    let saved = load_config();
    if let Some(dir) = saved.music_dir.filter(|d| d.is_dir()) {
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
        let nonexistent = PathBuf::from("/nonexistent/attune/music");
        let (dir, _src) = resolve_music_dir(Some(nonexistent.clone()));
        assert_ne!(dir, Some(nonexistent));
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

    #[test]
    fn config_roundtrip_test() {
        let mut cfg = AppConfig::default();
        cfg.setup_completed = true;
        cfg.music_dir = Some(PathBuf::from("/tmp"));
        cfg.extra_music_dirs = vec![PathBuf::from("/tmp/extra")];
        cfg.song_exclusions = vec!["excluded_song.flac".to_string(), "sample".to_string()];
        let json = serde_json::to_string(&cfg).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert!(loaded.setup_completed);
        assert_eq!(loaded.music_dir, Some(PathBuf::from("/tmp")));
        assert_eq!(loaded.extra_music_dirs, vec![PathBuf::from("/tmp/extra")]);
        assert_eq!(loaded.song_exclusions, vec!["excluded_song.flac", "sample"]);
    }

    #[test]
    fn test_song_exclusion_matching() {
        let exclusions = vec!["sample.wav".to_string(), "/skits/".to_string(), "Interlude".to_string()];
        let path1 = PathBuf::from("/music/skits/track1.mp3");
        let path2 = PathBuf::from("/music/sample.wav");
        let path3 = PathBuf::from("/music/song.flac");

        assert!(AppConfig::is_path_or_title_excluded(&exclusions, &path1, None));
        assert!(AppConfig::is_path_or_title_excluded(&exclusions, &path2, None));
        assert!(AppConfig::is_path_or_title_excluded(&exclusions, &path3, Some("Interlude")));
        assert!(!AppConfig::is_path_or_title_excluded(&exclusions, &path3, Some("Good Song")));
    }
}
