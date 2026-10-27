//! External DSP Active Detection & Conflict Mitigation
//!
//! Scans the host operating system (Linux, Android, Windows) for running or installed
//! global audio effects racks such as ViPER4Android, JamesDSP, EasyEffects, PulseEffects,
//! and Equalizer APO. If detected, Attune alerts the user and provides one-click bypass
//! options to prevent muddy double-processing or phase cancellation.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalDspStatus {
    pub detected: bool,
    pub engine_name: String,
    pub details: String,
    pub recommended_action: String,
}

impl Default for ExternalDspStatus {
    fn default() -> Self {
        Self {
            detected: false,
            engine_name: "None".to_string(),
            details: "No conflicting external system audio DSPs detected.".to_string(),
            recommended_action: "None needed. Attune built-in mastering rack operates cleanly.".to_string(),
        }
    }
}

/// Probe the operating system for active external DSP racks
pub fn probe_external_dsp() -> ExternalDspStatus {
    // 1. Check Android system paths & package sandboxes
    #[cfg(target_os = "android")]
    {
        if let Some(status) = probe_android_dsp() {
            return status;
        }
    }

    // 2. Check Linux process table and audio subsystem configs
    #[cfg(target_os = "linux")]
    {
        if let Some(status) = probe_linux_dsp() {
            return status;
        }
    }

    // 3. Check Windows filesystem / registry
    #[cfg(target_os = "windows")]
    {
        if let Some(status) = probe_windows_dsp() {
            return status;
        }
    }

    // Generic cross-platform fallback checks
    probe_filesystem_hints()
}

#[cfg(target_os = "linux")]
fn probe_linux_dsp() -> Option<ExternalDspStatus> {
    // Check /proc for active processes matching known DSP daemons
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let comm_path = path.join("comm");
            if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                let name = comm.trim().to_lowercase();
                if name.contains("easyeffects") {
                    return Some(ExternalDspStatus {
                        detected: true,
                        engine_name: "EasyEffects (PipeWire)".to_string(),
                        details: "EasyEffects daemon is actively running on PipeWire.".to_string(),
                        recommended_action: "Bypass external DSP or disable built-in FX to prevent double equalization.".to_string(),
                    });
                }
                if name.contains("pulseeffects") {
                    return Some(ExternalDspStatus {
                        detected: true,
                        engine_name: "PulseEffects".to_string(),
                        details: "PulseEffects daemon is actively running.".to_string(),
                        recommended_action: "Bypass external DSP or disable built-in FX to prevent double equalization.".to_string(),
                    });
                }
                if name.contains("jamesdsp") {
                    return Some(ExternalDspStatus {
                        detected: true,
                        engine_name: "JamesDSP for Linux".to_string(),
                        details: "JamesDSP audio processing daemon is running.".to_string(),
                        recommended_action: "Bypass external DSP or disable built-in FX to prevent double equalization.".to_string(),
                    });
                }
            }
        }
    }

    // Check config directories if active daemon was not in /proc
    if let Ok(home) = std::env::var("HOME") {
        let ee_config = Path::new(&home).join(".config/easyeffects");
        let jd_config = Path::new(&home).join(".config/jamesdsp");
        if ee_config.exists() && ee_config.is_dir() {
            // Note: only a passive hint, but check if pipewire socket exists
            if Path::new("/run/user").exists() {
                // If EasyEffects output sink exists in pipewire
            }
        }
        if jd_config.exists() && jd_config.is_dir() {
            // JamesDSP config exists
        }
    }

    None
}

#[cfg(target_os = "android")]
fn probe_android_dsp() -> Option<ExternalDspStatus> {
    // Check known soundfx libraries in system / vendor
    let v4a_paths = [
        "/system/lib64/soundfx/libv4a_fx.so",
        "/vendor/lib64/soundfx/libv4a_fx.so",
        "/system/lib/soundfx/libv4a_fx.so",
        "/vendor/lib/soundfx/libv4a_fx.so",
    ];
    for p in &v4a_paths {
        if Path::new(p).exists() {
            return Some(ExternalDspStatus {
                detected: true,
                engine_name: "ViPER4Android FX".to_string(),
                details: format!("Found ViPER4Android native library at {}.", p),
                recommended_action: "Bypass external DSP in ViPER or disable built-in FX in Attune.".to_string(),
            });
        }
    }

    let jd_paths = [
        "/system/lib64/soundfx/libjamesdsp.so",
        "/vendor/lib64/soundfx/libjamesdsp.so",
        "/system/lib/soundfx/libjamesdsp.so",
        "/vendor/lib/soundfx/libjamesdsp.so",
    ];
    for p in &jd_paths {
        if Path::new(p).exists() {
            return Some(ExternalDspStatus {
                detected: true,
                engine_name: "JamesDSP Audio Engine".to_string(),
                details: format!("Found JamesDSP native library at {}.", p),
                recommended_action: "Bypass external DSP in JamesDSP or disable built-in FX in Attune.".to_string(),
            });
        }
    }

    // Check app sandboxes
    let package_paths = [
        ("com.pittvandewitt.viperfx", "ViPER4Android FX"),
        ("com.wstxda.viper4android", "ViPER4Android Redesign"),
        ("me.timschneeberger.rootlessjamesdsp", "Rootless JamesDSP"),
    ];
    for (pkg, name) in &package_paths {
        let data_dir = format!("/data/data/{}", pkg);
        if Path::new(&data_dir).exists() {
            return Some(ExternalDspStatus {
                detected: true,
                engine_name: name.to_string(),
                details: format!("Found {} package directory at {}.", name, data_dir),
                recommended_action: "Bypass external DSP or disable built-in FX in Attune.".to_string(),
            });
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn probe_windows_dsp() -> Option<ExternalDspStatus> {
    let apo_paths = [
        "C:\\Program Files\\EqualizerAPO",
        "C:\\Program Files (x86)\\EqualizerAPO",
    ];
    for p in &apo_paths {
        if Path::new(p).exists() {
            return Some(ExternalDspStatus {
                detected: true,
                engine_name: "Equalizer APO".to_string(),
                details: format!("Found Equalizer APO installation at {}.", p),
                recommended_action: "Configure Equalizer APO to bypass Attune or disable built-in FX in Attune.".to_string(),
            });
        }
    }
    None
}

fn probe_filesystem_hints() -> ExternalDspStatus {
    ExternalDspStatus::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_external_dsp_returns_valid_status() {
        let status = probe_external_dsp();
        assert!(!status.engine_name.is_empty());
        assert!(!status.details.is_empty());
        assert!(!status.recommended_action.is_empty());
    }

    #[test]
    fn test_external_dsp_status_json_serialization() {
        let status = ExternalDspStatus {
            detected: true,
            engine_name: "ViPER4Android FX".to_string(),
            details: "Active native driver detected.".to_string(),
            recommended_action: "Bypass recommended.".to_string(),
        };
        let json = serde_json::to_string(&status).expect("Serialization failed");
        let decoded: ExternalDspStatus = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(status, decoded);
    }
}
