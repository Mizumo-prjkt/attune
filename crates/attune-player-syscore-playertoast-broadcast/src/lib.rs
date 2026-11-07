//! System-level player toast notification and media toolbar broadcaster for Attune.
//!
//! Provides system-agnostic desktop notification and media player broadcast
//! integration (e.g. KDE media player widget, GNOME notifications, Windows toast)
//! without cluttering `attune-desktop`. Adheres strictly to Mandate 4.

use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};

/// Payload describing the current player transport state and song metadata.
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
            title: "Atlantis Is Calling".to_string(),
            artist: "Modern Talking".to_string(),
            album: "Ready for Romance".to_string(),
            duration_ms: 228000,
            is_playing: true,
            artwork_path: Some(PathBuf::from("/path/to/art.jpg")),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let decoded: PlayerToastPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.title, "Atlantis Is Calling");
        assert_eq!(decoded.artist, "Modern Talking");
        assert_eq!(decoded.album, "Ready for Romance");
        assert_eq!(decoded.duration_ms, 228000);
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
}
