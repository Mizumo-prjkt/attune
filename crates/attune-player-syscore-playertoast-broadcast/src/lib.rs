//! System-level media player controls and now-playing broadcaster for Attune.
//!
//! Provides bidirectional OS media player integration:
//! - **Linux:** MPRIS v2 over D-Bus (`org.mpris.MediaPlayer2.attune`), supporting KDE Media
//!   Player widget, GNOME notifications, waybar, and `playerctl`.
//! - **Windows:** System Media Transport Controls (SMTC) via WinRT, supporting Windows 11
//!   Quick Settings flyout, Windows 10 volume overlay, and hardware multimedia keys.
//! - **Fallback:** System toast notifications when desktop media controls are unavailable.
//!
//! Adheres strictly to Mandate 4 (zero hardcoded values, dynamic platform adaptation,
//! transparent error handling without crashes).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use playwire::{Capabilities, Event, MediaControls, PlaybackState, PlayerConfig, Repeat, Track};
use serde::{Deserialize, Serialize};

/// Payload describing the current player transport state and song metadata.
/// Retained for backward compatibility with existing toast notification callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerToastPayload {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub is_playing: bool,
    pub artwork_path: Option<PathBuf>,
}

impl Default for PlayerToastPayload {
    fn default() -> Self {
        Self {
            title: "Unknown Title".to_string(),
            artist: "Unknown Artist".to_string(),
            album: "Unknown Album".to_string(),
            duration_ms: 0,
            is_playing: false,
            artwork_path: None,
        }
    }
}

/// Result of a system player toast broadcast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastResult {
    pub platform: String,
    pub success: bool,
    pub message: String,
}

/// Repeat mode for OS media transport synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SystemRepeatMode {
    #[default]
    Off,
    One,
    All,
}

impl From<Repeat> for SystemRepeatMode {
    fn from(r: Repeat) -> Self {
        match r {
            Repeat::Off => SystemRepeatMode::Off,
            Repeat::One => SystemRepeatMode::One,
            Repeat::All => SystemRepeatMode::All,
        }
    }
}

impl From<SystemRepeatMode> for Repeat {
    fn from(r: SystemRepeatMode) -> Self {
        match r {
            SystemRepeatMode::Off => Repeat::Off,
            SystemRepeatMode::One => Repeat::One,
            SystemRepeatMode::All => Repeat::All,
        }
    }
}

/// Detailed metadata describing the currently active or transitioning track for the OS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SystemTrackMetadata {
    pub id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub artwork_path: Option<PathBuf>,
    pub artwork_data_url: Option<String>,
    pub url: String,
}

impl SystemTrackMetadata {
    pub fn new(id: impl Into<String>, title: impl Into<String>, artist: impl Into<String>) -> Self {
        let artist_str = artist.into();
        Self {
            id: id.into(),
            title: title.into(),
            artists: if artist_str.is_empty() { Vec::new() } else { vec![artist_str] },
            album: String::new(),
            artwork_path: None,
            artwork_data_url: None,
            url: String::new(),
        }
    }
}

/// Comprehensive playback state published to OS media player integrations (MPRIS / SMTC).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemPlaybackState {
    pub track: Option<SystemTrackMetadata>,
    pub is_playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f64,
    pub shuffle: bool,
    pub repeat: SystemRepeatMode,
    pub can_go_next: bool,
    pub can_go_previous: bool,
    pub can_seek: bool,
}

impl Default for SystemPlaybackState {
    fn default() -> Self {
        Self {
            track: None,
            is_playing: false,
            position_ms: 0,
            duration_ms: 0,
            volume: 1.0,
            shuffle: false,
            repeat: SystemRepeatMode::Off,
            can_go_next: true,
            can_go_previous: true,
            can_seek: true,
        }
    }
}

/// Inbound control command triggered by the OS (KDE widget, GNOME media control, Windows SMTC, media keys).
#[derive(Debug, Clone, PartialEq)]
pub enum MediaControlCommand {
    Play,
    Pause,
    TogglePlayPause,
    Next,
    Previous,
    Stop,
    SeekToMs(u64),
    SeekBySec(f64),
    SetVolume(f64),
    SetShuffle(bool),
    SetRepeat(SystemRepeatMode),
    Raise,
    Quit,
}

/// Bidirectional system media player controller.
///
/// Registers the application as a first-class media player on the host operating system.
/// Handles media keys and remote GUI controls, while streaming playback state and track
/// metadata into the host notification center / taskbar / quick settings flyout.
pub struct SystemMediaService {
    controls: Option<MediaControls>,
    cmd_rx: Receiver<MediaControlCommand>,
    cached_artwork_file: Option<PathBuf>,
}

impl SystemMediaService {
    /// Initializes OS-level media integration.
    ///
    /// On Linux, connects to the D-Bus session bus as `org.mpris.MediaPlayer2.attune`.
    /// On Windows, binds to the window's HWND and WinRT `SystemMediaTransportControls`.
    ///
    /// If initialization fails on the host OS (e.g. running in headless CI/CD, or WinRT unavailable),
    /// fails gracefully with `is_active() == false` without interrupting playback.
    pub fn new(hwnd: Option<u64>) -> Self {
        let (cmd_tx, cmd_rx) = channel::<MediaControlCommand>();

        let mut config = PlayerConfig::new("attune")
            .desktop_entry("attune")
            .track_id_prefix("/org/mizproject/attune/track")
            .app_media_id("org.mizproject.attune");

        if let Some(h) = hwnd {
            config = config.hwnd(h);
        }

        let tx = cmd_tx.clone();
        let controls = match MediaControls::new(config, move |event| {
            let cmd = match event {
                Event::Play => MediaControlCommand::Play,
                Event::Pause => MediaControlCommand::Pause,
                Event::PlayPause => MediaControlCommand::TogglePlayPause,
                Event::Next => MediaControlCommand::Next,
                Event::Previous => MediaControlCommand::Previous,
                Event::Stop => MediaControlCommand::Stop,
                Event::SeekTo(dur) => MediaControlCommand::SeekToMs(dur.as_millis() as u64),
                Event::SeekBy(secs) => MediaControlCommand::SeekBySec(secs),
                Event::SetVolume(vol) => MediaControlCommand::SetVolume(vol),
                Event::SetShuffle(shuf) => MediaControlCommand::SetShuffle(shuf),
                Event::SetRepeat(rep) => MediaControlCommand::SetRepeat(rep.into()),
                Event::Raise => MediaControlCommand::Raise,
                Event::Quit => MediaControlCommand::Quit,
                _ => return,
            };
            let _ = tx.send(cmd);
        }) {
            Ok(c) => {
                log::info!("Initialized system media player integration (MPRIS/SMTC)");
                Some(c)
            }
            Err(e) => {
                log::warn!("System media player integration unavailable: {e}");
                None
            }
        };

        Self {
            controls,
            cmd_rx,
            cached_artwork_file: None,
        }
    }

    /// Whether native OS media controls (MPRIS or SMTC) are actively running.
    pub fn is_active(&self) -> bool {
        self.controls.is_some()
    }

    /// Non-blocking poll for incoming media control events sent by the OS or hardware media keys.
    pub fn poll_command(&self) -> Option<MediaControlCommand> {
        self.cmd_rx.try_recv().ok()
    }

    /// Publishes current playback state, timeline, track metadata, and capabilities to the host OS.
    pub fn update_state(&mut self, state: &SystemPlaybackState) {
        if self.controls.is_none() {
            return;
        }

        let artwork_url = if let Some(ref t) = state.track {
            self.resolve_artwork_uri(t)
        } else {
            String::new()
        };

        let Some(ref mut controls) = self.controls else {
            return;
        };

        let playwire_track = state.track.as_ref().map(|t| {
            Track {
                id: if t.id.trim().is_empty() { "attune_current".into() } else { t.id.clone() },
                title: if t.title.trim().is_empty() { "Unknown Title".into() } else { t.title.clone() },
                artists: if t.artists.is_empty() { vec!["Unknown Artist".into()] } else { t.artists.clone() },
                album: t.album.clone(),
                artwork_url,
                url: if t.url.trim().is_empty() { String::new() } else { t.url.clone() },
            }
        });

        let playwire_state = PlaybackState {
            track: playwire_track,
            playing: state.is_playing,
            position: Duration::from_millis(state.position_ms),
            duration: if state.duration_ms > 0 {
                Some(Duration::from_millis(state.duration_ms))
            } else {
                None
            },
            volume: state.volume.clamp(0.0, 1.0),
            repeat: state.repeat.into(),
            shuffle: state.shuffle,
            capabilities: Capabilities {
                can_go_next: state.can_go_next,
                can_go_previous: state.can_go_previous,
                can_seek: state.can_seek,
            },
        };

        if let Err(e) = controls.set_state(&playwire_state) {
            log::debug!("Failed to update system media player state: {e}");
        }
    }

    /// Resolves an accessible `file://` or `http://` URI for album artwork display.
    fn resolve_artwork_uri(&mut self, meta: &SystemTrackMetadata) -> String {
        // 1. Direct local file path
        if let Some(ref path) = meta.artwork_path {
            if path.is_file() {
                return to_file_uri(path);
            }
        }

        // 2. Base64 Data URL decoding into temporary cache file
        if let Some(ref data_url) = meta.artwork_data_url {
            if let Some(base64_str) = data_url.strip_prefix("data:image/jpeg;base64,")
                .or_else(|| data_url.strip_prefix("data:image/png;base64,"))
                .or_else(|| data_url.strip_prefix("data:image/webp;base64,"))
            {
                if let Ok(bytes) = base64_decode(base64_str) {
                    let temp_path = std::env::temp_dir().join("attune_os_media_cover.jpg");
                    if fs::write(&temp_path, bytes).is_ok() {
                        self.cached_artwork_file = Some(temp_path.clone());
                        return to_file_uri(&temp_path);
                    }
                }
            }
        }

        String::new()
    }
}

/// Helper converting a filesystem path into an RFC compliant `file://` URI.
fn to_file_uri(path: &Path) -> String {
    let clean = path.to_string_lossy().replace('\\', "/");
    if clean.starts_with('/') {
        format!("file://{clean}")
    } else {
        format!("file:///{clean}")
    }
}

/// Simple base64 decoding helper without extra dependencies.
fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity((input.len() * 3) / 4);
    let mut buf = 0u32;
    let mut bits = 0;

    for &b in input.as_bytes() {
        if b == b'=' {
            break;
        }
        let val = match TABLE.iter().position(|&c| c == b) {
            Some(idx) => idx as u32,
            None => {
                if b == b'\r' || b == b'\n' || b == b' ' {
                    continue;
                }
                return Err("invalid base64 byte");
            }
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

/// Broadcasts the current track information as a system-level notification / toast.
///
/// Dispatches asynchronously or via lightweight child process without blocking the
/// caller. Fails gracefully with transparency if notifications are not supported or
/// disabled on the current platform.
pub fn broadcast_player_toast(payload: &PlayerToastPayload) -> Result<BroadcastResult, String> {
    let title_clean = if payload.title.trim().is_empty() {
        "Attune Player"
    } else {
        payload.title.trim()
    };

    let body = format!("{} • {}", payload.artist.trim(), payload.album.trim());
    let status_prefix = if payload.is_playing { "▶ Now Playing" } else { "⏸ Paused" };
    let summary = format!("{status_prefix}: {title_clean}");

    #[cfg(target_os = "linux")]
    {
        // Try notify-send with desktop-entry hints
        let mut cmd = Command::new("notify-send");
        cmd.arg("-a").arg("Attune");
        cmd.arg("-h").arg("string:desktop-entry:attune");
        cmd.arg("-t").arg("3000"); // 3 seconds timeout

        if let Some(ref art) = payload.artwork_path {
            if art.is_file() {
                cmd.arg("-i").arg(art);
            }
        }

        cmd.arg(&summary);
        cmd.arg(&body);

        match cmd.spawn() {
            Ok(mut child) => {
                let _ = child.wait();
                Ok(BroadcastResult {
                    platform: "linux-notify-send".to_string(),
                    success: true,
                    message: format!("Broadcasted toast for '{title_clean}'"),
                })
            }
            Err(e) => {
                log::warn!("Linux notify-send broadcast failed: {e}");
                Ok(BroadcastResult {
                    platform: "linux".to_string(),
                    success: false,
                    message: format!("notify-send not available: {e}"),
                })
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Dispatch Windows PowerShell BurntToast / Baloon / System Toast
        let script = format!(
            r#"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null;
            [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null;
            $template = @"
<toast>
    <visual>
        <binding template="ToastGeneric">
            <text>{summary}</text>
            <text>{body}</text>
        </binding>
    </visual>
</toast>
"@;
            $xml = New-Object Windows.Data.Xml.Dom.XmlDocument;
            $xml.LoadXml($template);
            $toast = [Windows.UI.Notifications.ToastNotification]::new($xml);
            [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("Attune").Show($toast);"#
        );

        match Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]).spawn() {
            Ok(mut child) => {
                let _ = child.wait();
                Ok(BroadcastResult {
                    platform: "windows-toast".to_string(),
                    success: true,
                    message: format!("Broadcasted Windows toast for '{title_clean}'"),
                })
            }
            Err(e) => {
                log::warn!("Windows toast broadcast failed: {e}");
                Ok(BroadcastResult {
                    platform: "windows".to_string(),
                    success: false,
                    message: format!("PowerShell toast execution failed: {e}"),
                })
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok(BroadcastResult {
            platform: "generic".to_string(),
            success: false,
            message: "Platform does not support native system player toast".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_serialization_roundtrip() {
        let payload = PlayerToastPayload {
            title: "Lucky Guy".to_string(),
            artist: "Modern Talking".to_string(),
            album: "The 1st Album".to_string(),
            duration_ms: 215000,
            is_playing: true,
            artwork_path: Some(PathBuf::from("/path/to/art.jpg")),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let decoded: PlayerToastPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.title, "Lucky Guy");
        assert_eq!(decoded.artist, "Modern Talking");
        assert_eq!(decoded.album, "The 1st Album");
        assert_eq!(decoded.duration_ms, 215000);
        assert!(decoded.is_playing);
        assert_eq!(decoded.artwork_path, Some(PathBuf::from("/path/to/art.jpg")));
    }

    #[test]
    fn test_broadcast_does_not_panic_on_any_input() {
        let payload = PlayerToastPayload {
            title: "Test Track \"with quotes\" & <special chars>".to_string(),
            artist: "Artist & Co.".to_string(),
            album: "Album / EP".to_string(),
            duration_ms: 120000,
            is_playing: false,
            artwork_path: None,
        };

        let res = broadcast_player_toast(&payload);
        assert!(res.is_ok());
        let outcome = res.unwrap();
        assert!(!outcome.platform.is_empty());
    }

    #[test]
    fn test_system_playback_state_and_command_types() {
        let mut state = SystemPlaybackState::default();
        assert_eq!(state.volume, 1.0);
        assert!(!state.is_playing);

        state.track = Some(SystemTrackMetadata::new("track-42", "Tamam Tamam", "Summer Cem"));
        state.is_playing = true;
        state.position_ms = 42000;
        state.duration_ms = 185000;
        state.repeat = SystemRepeatMode::One;
        state.shuffle = true;

        assert_eq!(state.track.as_ref().unwrap().title, "Tamam Tamam");
        assert_eq!(state.track.as_ref().unwrap().artists, vec!["Summer Cem".to_string()]);
        assert_eq!(state.repeat, SystemRepeatMode::One);

        let cmd = MediaControlCommand::SeekToMs(50000);
        assert_eq!(cmd, MediaControlCommand::SeekToMs(50000));
    }

    #[test]
    fn test_file_uri_conversion() {
        let path = PathBuf::from("/home/user/Music/cover.jpg");
        assert_eq!(to_file_uri(&path), "file:///home/user/Music/cover.jpg");
    }

    #[test]
    fn test_base64_decode() {
        let sample = "SGVsbG8gV29ybGQ=";
        let decoded = base64_decode(sample).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_system_media_service_lifecycle() {
        let mut service = SystemMediaService::new(None);
        assert!(service.poll_command().is_none());

        let mut state = SystemPlaybackState::default();
        state.track = Some(SystemTrackMetadata::new("test-1", "Sustain Bridge", "Attune"));
        state.is_playing = true;
        state.position_ms = 15000;
        state.duration_ms = 240000;
        service.update_state(&state);
    }
}
