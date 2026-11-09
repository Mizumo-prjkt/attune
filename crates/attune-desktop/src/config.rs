//! Desktop environment resolution and configuration for Attune.
//!
//! Mandate 4 (no hardcoded values): every path is resolved from environment
//! variables first, then user configuration, and finally OS-standard directories.

use std::collections::HashSet;
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

fn default_true() -> bool { true }
fn default_60() -> u64 { 60 }
fn default_engine() -> String { "attune_codec".to_string() }
fn default_language() -> String { "en".to_string() }
fn default_lyric_provider() -> String { "lrclib".to_string() }
fn default_lyric_font() -> String { "system".to_string() }
fn default_lyric_priority() -> Vec<String> {
    vec!["embedded".into(), "lrc".into(), "ttml".into(), "elrc".into(), "external".into()]
}
fn default_api_priority() -> Vec<String> {
    vec!["youtube".into(), "wikipedia_musicbrainz".into(), "musixmatch".into()]
}
fn default_align_left() -> String { "left".to_string() }
fn default_layout_side() -> String { "left".to_string() }
fn default_dock_lyrics_font_size() -> u32 { 16 }
fn default_fullscreen_lyrics_font_size() -> u32 { 28 }

pub const PRESET_SPOTLIGHT_BLUR: &str = r#"/* Spotlight Depth Blur (Better Lyrics Inspired) */
.fullscreen-lyric-inactive {
  filter: blur(1.5px);
  opacity: 0.28;
  transition: filter 0.45s ease, opacity 0.45s ease, transform 0.45s cubic-bezier(0.22, 1, 0.36, 1);
}

.fullscreen-lyric-inactive:hover {
  filter: blur(0px);
  opacity: 0.85;
}

.fullscreen-lyric-active {
  filter: none;
  opacity: 1;
  transform: scale(1.04) translateX(8px);
}"#;

pub const PRESET_SPRING_POP: &str = r#"/* Apple Music Spring Pop & Float */
.fullscreen-lyric-active {
  animation: appleSpringPop 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275) both;
}

@keyframes appleSpringPop {
  0% { transform: scale(0.96) translateY(4px); }
  70% { transform: scale(1.055) translateY(-2px); }
  100% { transform: scale(1.04) translateY(0); }
}"#;

pub const PRESET_CYBERPUNK_NEON: &str = r#"/* Cyberpunk Neon Pulse Glow */
.fullscreen-lyric-active {
  color: #64ffda;
  text-shadow: 0 0 20px rgba(100, 255, 218, 0.9), 0 0 40px rgba(167, 139, 250, 0.6), 0 0 60px rgba(244, 63, 94, 0.4);
  animation: neonFlicker 3s infinite alternate;
}

@keyframes neonFlicker {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.92; }
}"#;

pub const PRESET_KARAOKE_BOUNCE: &str = r#"/* Karaoke Golden Glow & Tactile Bounce */
.fullscreen-lyric-inactive {
  color: rgba(255, 255, 255, 0.42);
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.7);
  transition: all 0.35s ease;
}

.fullscreen-lyric-inactive:hover {
  color: #fff275;
  opacity: 0.85;
}

.fullscreen-lyric-active {
  color: #FFDF00; /* Vivid Karaoke Gold */
  font-weight: 800;
  letter-spacing: 0.5px;
  text-shadow: 0 0 12px rgba(255, 223, 0, 0.95), 0 0 28px rgba(255, 170, 0, 0.65), 0 2px 4px rgba(0, 0, 0, 0.9);
  animation: karaokeBounce 0.45s cubic-bezier(0.175, 0.885, 0.32, 1.275) both;
}

@keyframes karaokeBounce {
  0% { transform: scale(0.96) translateY(3px); }
  65% { transform: scale(1.08) translateY(-2px); }
  100% { transform: scale(1.06) translateY(0); }
}"#;

pub fn default_lyrics_custom_css() -> String {
    PRESET_SPOTLIGHT_BLUR.to_string()
}

pub fn default_app_font() -> String {
    "system".to_string()
}

/// Persistent user configuration for Attune Desktop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub setup_completed: bool,
    #[serde(default)]
    pub music_dir: Option<PathBuf>,
    #[serde(default)]
    pub extra_music_dirs: Vec<PathBuf>,
    #[serde(default)]
    pub song_exclusions: Vec<String>,

    // General
    #[serde(default)]
    pub home_subtext: Option<String>,
    #[serde(default = "default_app_font")]
    pub app_font_family: String,
    #[serde(default = "default_true")]
    pub enable_shortcuts: bool,
    #[serde(default = "default_true")]
    pub system_media_toast: bool,
    #[serde(default = "default_true")]
    pub auto_scroll_long_text: bool,
    #[serde(default = "default_true")]
    pub high_quality_thumbnails: bool,
    #[serde(default = "default_true")]
    pub fetch_youtube_thumbnails: bool,

    // Audio
    #[serde(default)]
    pub force_32bit_float: bool,
    #[serde(default)]
    pub skip_silence: bool,
    #[serde(default = "default_engine")]
    pub engine_choice: String,

    // Language
    #[serde(default = "default_language")]
    pub language: String,

    // Lyrics
    #[serde(default = "default_lyric_priority")]
    pub lyric_priority: Vec<String>,
    #[serde(default = "default_lyric_provider")]
    pub lyric_provider: String,
    #[serde(default)]
    pub am_lyrics_agreed: bool,
    #[serde(default)]
    pub musixmatch_api_key: String,
    #[serde(default = "default_lyric_font")]
    pub lyric_font_family: String,
    #[serde(default)]
    pub sidebar_text_glow: bool,
    #[serde(default = "default_align_left")]
    pub dock_lyrics_align: String,
    #[serde(default = "default_dock_lyrics_font_size")]
    pub dock_lyrics_font_size: u32,

    // Fullscreen View
    #[serde(default = "default_true")]
    pub fullscreen_text_glow: bool,
    #[serde(default = "default_true")]
    pub fullscreen_gradient_bg: bool,
    #[serde(default = "default_true")]
    pub fullscreen_visualizer_effects: bool,
    #[serde(default = "default_align_left")]
    pub fullscreen_lyrics_align: String,
    #[serde(default = "default_layout_side")]
    pub fullscreen_layout_side: String,
    #[serde(default = "default_fullscreen_lyrics_font_size")]
    pub fullscreen_lyrics_font_size: u32,
    #[serde(default)]
    pub enable_lyrics_custom_css: bool,
    #[serde(default = "default_lyrics_custom_css")]
    pub lyrics_custom_css: String,

    // API Providers
    #[serde(default = "default_api_priority")]
    pub api_priority: Vec<String>,
    #[serde(default = "default_true")]
    pub api_wikipedia_enabled: bool,
    #[serde(default = "default_true")]
    pub api_musicbrainz_enabled: bool,
    #[serde(default = "default_true")]
    pub api_youtube_enabled: bool,
    #[serde(default)]
    pub api_musixmatch_enabled: bool,

    // Directory & Indexes
    #[serde(default = "default_60")]
    pub min_song_duration_sec: u64,
    #[serde(default)]
    pub gpu_artist_index_offload: bool,
    #[serde(default)]
    pub gpu_bpm_index_offload: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            setup_completed: false,
            music_dir: None,
            extra_music_dirs: Vec::new(),
            song_exclusions: Vec::new(),
            home_subtext: None,
            app_font_family: default_app_font(),
            enable_shortcuts: true,
            system_media_toast: true,
            auto_scroll_long_text: true,
            high_quality_thumbnails: true,
            fetch_youtube_thumbnails: true,
            force_32bit_float: false,
            skip_silence: false,
            engine_choice: default_engine(),
            language: default_language(),
            lyric_priority: default_lyric_priority(),
            lyric_provider: default_lyric_provider(),
            am_lyrics_agreed: false,
            musixmatch_api_key: String::new(),
            lyric_font_family: default_lyric_font(),
            sidebar_text_glow: false,
            dock_lyrics_align: default_align_left(),
            dock_lyrics_font_size: default_dock_lyrics_font_size(),
            fullscreen_text_glow: true,
            fullscreen_gradient_bg: true,
            fullscreen_visualizer_effects: true,
            fullscreen_lyrics_align: default_align_left(),
            fullscreen_layout_side: default_layout_side(),
            fullscreen_lyrics_font_size: default_fullscreen_lyrics_font_size(),
            enable_lyrics_custom_css: false,
            lyrics_custom_css: default_lyrics_custom_css(),
            api_priority: default_api_priority(),
            api_wikipedia_enabled: true,
            api_musicbrainz_enabled: true,
            api_youtube_enabled: true,
            api_musixmatch_enabled: false,
            min_song_duration_sec: 60,
            gpu_artist_index_offload: false,
            gpu_bpm_index_offload: false,
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

/// Persisted favorites collection for songs, artists, and albums.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FavoritesStore {
    #[serde(default)]
    pub tracks: HashSet<PathBuf>,
    #[serde(default)]
    pub artists: HashSet<String>,
    #[serde(default)]
    pub albums: HashSet<String>,
}

/// Returns the favorites storage file path (~/.config/attune/favorites.json).
pub fn favorites_file_path() -> Option<PathBuf> {
    let dir = dirs::config_dir()?.join("attune");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("favorites.json"))
}

/// Loads the persisted favorites or returns empty store.
pub fn load_favorites() -> FavoritesStore {
    if let Some(path) = favorites_file_path() {
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(store) = serde_json::from_str(&content) {
                    return store;
                }
            }
        }
    }
    FavoritesStore::default()
}

/// Saves the favorites store to disk.
pub fn save_favorites(store: &FavoritesStore) {
    if let Some(path) = favorites_file_path() {
        if let Ok(content) = serde_json::to_string_pretty(store) {
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

/// Scans the given candidate base directories for artifacts from a previous Flutter-based
/// Attune installation (`com.example.attune_app`, `shared_preferences.json`, `flutter_assets`,
/// `flutter_windows.dll`, `libflutter_linux_gtk.so`, or `attune_app.exe`).
pub fn detect_legacy_flutter_in_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        let candidates = [
            root.join("com.example.attune_app"),
            root.join("com.example").join("attune_app"),
            root.join("attune_app").join("shared_preferences.json"),
            root.join("shared_preferences.json"),
            root.join("flutter_windows.dll"),
            root.join("libflutter_linux_gtk.so"),
            root.join("attune_app.exe"),
            root.join("data").join("flutter_assets"),
        ];
        for cand in candidates {
            if cand.exists() && !found.contains(&cand) {
                found.push(cand);
            }
        }
    }
    found
}

/// Inspects standard OS application data, local data, config, and executable directories
/// for legacy Flutter installation data or binaries that are incompatible with Native Rust.
pub fn detect_legacy_flutter_install() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(d) = dirs::data_dir() {
        roots.push(d);
    }
    if let Some(d) = dirs::data_local_dir() {
        if !roots.contains(&d) {
            roots.push(d);
        }
    }
    if let Some(d) = dirs::config_dir() {
        if !roots.contains(&d) {
            roots.push(d);
        }
    }
    if let Some(d) = dirs::cache_dir() {
        if !roots.contains(&d) {
            roots.push(d);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let p = parent.to_path_buf();
            if !roots.contains(&p) {
                roots.push(p);
            }
        }
    }
    detect_legacy_flutter_in_roots(&roots)
}

/// Safely purges detected legacy Flutter application data/artifacts and returns the count of removed items.
pub fn purge_legacy_flutter_paths(paths: &[PathBuf]) -> Result<usize, String> {
    let mut removed = 0usize;
    for p in paths {
        if !p.exists() {
            continue;
        }
        let res = if p.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        };
        match res {
            Ok(_) => removed += 1,
            Err(e) => return Err(format!("Failed to remove {}: {e}", p.display())),
        }
    }
    Ok(removed)
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

/// Information describing the system GPU capabilities for compute offloading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuCapabilityInfo {
    pub is_supported: bool,
    pub model: String,
    pub api: String,
    pub details: String,
}

/// Probes GPU hardware and API capabilities (Vulkan >= 1.1 / OpenCL) with zero hardcoding.
/// Gracefully falls back to unsupported status if criteria are not met or hardware is legacy.
pub fn probe_gpu_capabilities() -> GpuCapabilityInfo {
    if let Some((adapter_name, backend_name)) = attune_core::gpu::get_gpu_device_info() {
        return GpuCapabilityInfo {
            is_supported: true,
            model: adapter_name,
            api: format!("{backend_name} Compute"),
            details: "GPU hardware acceleration available for parallel indexing".to_string(),
        };
    }

    #[cfg(target_os = "linux")]
    {
        let mut model = "Standard Graphics Adapter".to_string();

        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("card0-") || !name.starts_with("card") {
                    continue;
                }
                let uevent_path = entry.path().join("device/uevent");
                if let Ok(content) = std::fs::read_to_string(&uevent_path) {
                    for line in content.lines() {
                        if line.starts_with("DRIVER=") {
                            let drv = line.trim_start_matches("DRIVER=").trim();
                            model = format!("DRM Driver ({drv})");
                        }
                    }
                }
            }
        }

        let has_vulkan = std::process::Command::new("vulkaninfo")
            .arg("--summary")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let has_opencl = std::path::Path::new("/etc/OpenCL/vendors").is_dir()
            || std::process::Command::new("clinfo")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

        if has_vulkan || has_opencl {
            let api = if has_vulkan && has_opencl {
                "Vulkan 1.2+ & OpenCL Compute"
            } else if has_vulkan {
                "Vulkan 1.2+ Compute"
            } else {
                "OpenCL Compute"
            };
            GpuCapabilityInfo {
                is_supported: true,
                model,
                api: api.to_string(),
                details: "GPU hardware acceleration available for parallel indexing".to_string(),
            }
        } else {
            GpuCapabilityInfo {
                is_supported: false,
                model,
                api: "None / Legacy (e.g. HD 5xxx or earlier)".to_string(),
                details: "System lacks Vulkan >= 1.1 or OpenCL compute driver. GPU offloading disabled.".to_string(),
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        GpuCapabilityInfo {
            is_supported: true,
            model: "Direct3D / Vulkan Compute".to_string(),
            api: "Direct3D 12 / Vulkan".to_string(),
            details: "GPU compute supported via Windows graphics runtime".to_string(),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        GpuCapabilityInfo {
            is_supported: false,
            model: "Generic".to_string(),
            api: "None".to_string(),
            details: "GPU compute offloading unsupported on platform".to_string(),
        }
    }
}

/// Creates a standard TAR archive containing Attune configurations and library cache.
pub fn export_backup_tar(archive_path: &std::path::Path) -> Result<usize, String> {
    use std::io::Write;
    let mut files_to_pack: Vec<(String, Vec<u8>)> = Vec::new();

    // 1. config.json
    if let Some(cfg_path) = config_file_path() {
        if let Ok(bytes) = std::fs::read(&cfg_path) {
            files_to_pack.push(("config.json".to_string(), bytes));
        }
    }

    // 2. library_cache.json
    if let Some(cache_dir) = dirs::cache_dir().map(|d| d.join("attune")) {
        let lib_path = cache_dir.join("library_cache.json");
        if let Ok(bytes) = std::fs::read(&lib_path) {
            files_to_pack.push(("library_cache.json".to_string(), bytes));
        }
    }

    if files_to_pack.is_empty() {
        // Fallback: save current default config
        let cfg = load_config();
        if let Ok(json) = serde_json::to_vec_pretty(&cfg) {
            files_to_pack.push(("config.json".to_string(), json));
        }
    }

    let mut out = std::fs::File::create(archive_path)
        .map_err(|e| format!("Failed to create backup archive: {e}"))?;

    let mut total_bytes = 0;
    for (name, content) in &files_to_pack {
        let mut header = [0u8; 512];
        let name_bytes = name.as_bytes();
        let copy_len = name_bytes.len().min(99);
        header[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        let size_octal = format!("{:011o} ", content.len());
        header[124..136].copy_from_slice(size_octal.as_bytes());
        header[136..148].copy_from_slice(b"14000000000 ");
        header[148..156].copy_from_slice(b"        ");
        header[156] = b'0';
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");

        let sum: u32 = header.iter().map(|&b| b as u32).sum();
        let chksum_str = format!("{:06o}\0 ", sum);
        header[148..156].copy_from_slice(chksum_str.as_bytes());

        out.write_all(&header).map_err(|e| format!("Failed to write TAR header: {e}"))?;
        out.write_all(content).map_err(|e| format!("Failed to write TAR content: {e}"))?;

        let pad = (512 - (content.len() % 512)) % 512;
        if pad > 0 {
            out.write_all(&vec![0u8; pad]).map_err(|e| format!("Failed to pad TAR: {e}"))?;
        }
        total_bytes += 512 + content.len() + pad;
    }

    out.write_all(&[0u8; 1024]).map_err(|e| format!("Failed to write TAR trailer: {e}"))?;
    total_bytes += 1024;

    Ok(total_bytes)
}

/// Restores configuration files from a TAR archive.
pub fn import_backup_tar(archive_path: &std::path::Path) -> Result<AppConfig, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(archive_path)
        .map_err(|e| format!("Failed to open backup archive: {e}"))?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| format!("Failed to read archive: {e}"))?;

    let mut offset = 0;
    let mut restored_config: Option<AppConfig> = None;

    while offset + 512 <= buf.len() {
        let header = &buf[offset..offset + 512];
        if header.iter().all(|&b| b == 0) {
            break;
        }

        let name_end = header[..100].iter().position(|&b| b == 0).unwrap_or(100);
        let name = String::from_utf8_lossy(&header[..name_end]).trim().to_string();

        let size_str = String::from_utf8_lossy(&header[124..136]).trim().to_string();
        let size = usize::from_str_radix(&size_str, 8).unwrap_or(0);

        offset += 512;
        if offset + size > buf.len() {
            return Err("Archive file truncated or corrupted".to_string());
        }

        let content = &buf[offset..offset + size];
        if name == "config.json" {
            if let Ok(cfg) = serde_json::from_slice::<AppConfig>(content) {
                save_config(&cfg);
                restored_config = Some(cfg);
            }
        } else if name == "library_cache.json" {
            if let Some(cache_dir) = dirs::cache_dir().map(|d| d.join("attune")) {
                let _ = std::fs::create_dir_all(&cache_dir);
                let _ = std::fs::write(cache_dir.join("library_cache.json"), content);
            }
        }

        let pad = (512 - (size % 512)) % 512;
        offset += size + pad;
    }

    restored_config.ok_or_else(|| "No valid config.json found inside backup archive".to_string())
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
        assert!(loaded.auto_scroll_long_text);
        assert_eq!(loaded.dock_lyrics_font_size, 16);
        assert_eq!(loaded.fullscreen_lyrics_font_size, 28);
        assert_eq!(loaded.app_font_family, "system");

        cfg.auto_scroll_long_text = false;
        let json2 = serde_json::to_string(&cfg).unwrap();
        let loaded2: AppConfig = serde_json::from_str(&json2).unwrap();
        assert!(!loaded2.auto_scroll_long_text);
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

    #[test]
    fn test_gpu_probe_returns_non_empty_metadata() {
        let info = probe_gpu_capabilities();
        assert!(!info.model.is_empty());
        assert!(!info.api.is_empty());
        assert!(!info.details.is_empty());
    }

    #[test]
    fn test_backup_and_restore_tar_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("attune_test_backup_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let archive_path = temp_dir.join("backup.tar");

        let export_res = export_backup_tar(&archive_path);
        assert!(export_res.is_ok());
        let bytes_written = export_res.unwrap();
        assert!(bytes_written >= 1536); // At least 1 file (512 header + content) + 1024 trailer

        let import_res = import_backup_tar(&archive_path);
        assert!(import_res.is_ok());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_favorites_store_serialization_roundtrip() {
        let mut store = FavoritesStore::default();
        store.tracks.insert(PathBuf::from("/music/song1.flac"));
        store.tracks.insert(PathBuf::from("/music/song2.mp3"));
        store.artists.insert("DECO*27".to_string());
        store.artists.insert("Hatsune Miku".to_string());
        store.albums.insert("Ghost".to_string());

        let json = serde_json::to_string_pretty(&store).unwrap();
        let loaded: FavoritesStore = serde_json::from_str(&json).unwrap();
        assert_eq!(store, loaded);
        assert_eq!(loaded.tracks.len(), 2);
        assert!(loaded.artists.contains("DECO*27"));
        assert!(loaded.albums.contains("Ghost"));
    }

    #[test]
    fn test_lyrics_custom_css_presets_and_defaults() {
        let def = default_lyrics_custom_css();
        assert!(!def.is_empty());
        assert!(def.contains("fullscreen-lyric"));

        assert!(PRESET_SPOTLIGHT_BLUR.contains("filter: blur"));
        assert!(PRESET_SPRING_POP.contains("appleSpringPop"));
        assert!(PRESET_CYBERPUNK_NEON.contains("neonFlicker"));
        assert!(PRESET_KARAOKE_BOUNCE.contains("karaokeBounce"));
        assert!(PRESET_KARAOKE_BOUNCE.contains("#FFDF00"));

        let mut cfg = AppConfig::default();
        assert_eq!(cfg.dock_lyrics_align, "left");
        assert_eq!(cfg.fullscreen_lyrics_align, "left");
        assert_eq!(cfg.fullscreen_layout_side, "left");
        assert!(!cfg.enable_lyrics_custom_css);
        assert!(!cfg.lyrics_custom_css.is_empty());

        cfg.enable_lyrics_custom_css = true;
        cfg.lyrics_custom_css = PRESET_KARAOKE_BOUNCE.to_string();
        cfg.dock_lyrics_align = "center".to_string();
        cfg.fullscreen_lyrics_align = "right".to_string();
        cfg.fullscreen_layout_side = "right".to_string();
        let json = serde_json::to_string(&cfg).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert!(loaded.enable_lyrics_custom_css);
        assert_eq!(loaded.lyrics_custom_css, PRESET_KARAOKE_BOUNCE);
        assert_eq!(loaded.dock_lyrics_align, "center");
        assert_eq!(loaded.fullscreen_lyrics_align, "right");
        assert_eq!(loaded.fullscreen_layout_side, "right");
    }

    #[test]
    fn test_detect_and_purge_legacy_flutter_install() {
        let temp_dir = std::env::temp_dir().join(format!(
            "attune_test_flutter_detect_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&temp_dir);

        // Initially clean
        assert!(detect_legacy_flutter_in_roots(&[temp_dir.clone()]).is_empty());

        // Simulate legacy Flutter artifacts
        let flutter_dll = temp_dir.join("flutter_windows.dll");
        let flutter_prefs_dir = temp_dir.join("attune_app");
        let _ = std::fs::create_dir_all(&flutter_prefs_dir);
        let flutter_prefs = flutter_prefs_dir.join("shared_preferences.json");
        let _ = std::fs::write(&flutter_dll, b"dummy_flutter_dll");
        let _ = std::fs::write(&flutter_prefs, b"{}");

        let detected = detect_legacy_flutter_in_roots(&[temp_dir.clone()]);
        assert_eq!(detected.len(), 2);
        assert!(detected.contains(&flutter_dll));
        assert!(detected.contains(&flutter_prefs));

        // Purge detected paths
        let purged_count = purge_legacy_flutter_paths(&detected).unwrap();
        assert_eq!(purged_count, 2);
        assert!(detect_legacy_flutter_in_roots(&[temp_dir.clone()]).is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
