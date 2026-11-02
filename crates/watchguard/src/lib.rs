//! Attune Watchguard — Autonomous Device Stability, Audio Clipping & Overhead Monitor
//!
//! Exposes a high-performance C-ABI shared library (`libattune_watchguard.so`) that
//! tracks real-time DSP audio buffer deadlines, clipping events, and UI frame pacing.
//! Detects device overload and recommends falling back to the system default codec
//! when high DSP overhead impacts hardware playback.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::{LazyLock, Mutex};

const MAX_FRAME_HISTORY: usize = 120;
const CLIPPING_AMPLITUDE_THRESHOLD: f32 = 1.02;
const OVERLOAD_INSTABILITY_THRESHOLD: f32 = 0.70;

/// Real-time health and stability status snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityStatus {
    pub is_overloaded: bool,
    pub instability_score: f32,
    pub clipping_count: u32,
    pub underrun_count: u32,
    pub recent_clipping_events: u32,
    pub recent_underrun_events: u32,
    pub average_fps: f32,
    pub dropped_frame_ratio: f32,
    pub peak_frame_duration_ms: f32,
    pub warning_message: Option<String>,
    pub recommend_system_codec: bool,
    pub timestamp_ms: u64,
}

impl Default for StabilityStatus {
    fn default() -> Self {
        Self {
            is_overloaded: false,
            instability_score: 0.0,
            clipping_count: 0,
            underrun_count: 0,
            recent_clipping_events: 0,
            recent_underrun_events: 0,
            average_fps: 60.0,
            dropped_frame_ratio: 0.0,
            peak_frame_duration_ms: 16.6,
            warning_message: None,
            recommend_system_codec: false,
            timestamp_ms: 0,
        }
    }
}

/// Internal state for the watchguard monitor
struct WatchguardState {
    clipping_total: u32,
    underrun_total: u32,
    recent_clipping: u32,
    recent_underruns: u32,
    frame_durations: VecDeque<f32>,
    last_eval_time: std::time::Instant,
}

impl WatchguardState {
    fn new() -> Self {
        Self {
            clipping_total: 0,
            underrun_total: 0,
            recent_clipping: 0,
            recent_underruns: 0,
            frame_durations: VecDeque::with_capacity(MAX_FRAME_HISTORY),
            last_eval_time: std::time::Instant::now(),
        }
    }

    fn record_audio_event(&mut self, peak_amplitude: f32, is_underrun: bool, is_clipping: bool) {
        if is_clipping || peak_amplitude >= CLIPPING_AMPLITUDE_THRESHOLD {
            self.clipping_total = self.clipping_total.saturating_add(1);
            self.recent_clipping = self.recent_clipping.saturating_add(1);
        }
        if is_underrun {
            self.underrun_total = self.underrun_total.saturating_add(1);
            self.recent_underruns = self.recent_underruns.saturating_add(1);
        }
    }

    fn record_ui_frame(&mut self, frame_time_ms: f32) {
        if frame_time_ms <= 0.0 || frame_time_ms > 5000.0 {
            return;
        }
        if self.frame_durations.len() >= MAX_FRAME_HISTORY {
            self.frame_durations.pop_front();
        }
        self.frame_durations.push_back(frame_time_ms);
    }

    fn evaluate_status(&mut self) -> StabilityStatus {
        let now = std::time::Instant::now();
        // Decay recent events if more than 3 seconds elapsed
        if now.duration_since(self.last_eval_time).as_secs() >= 3 {
            self.recent_clipping = (self.recent_clipping as f32 * 0.70) as u32;
            self.recent_underruns = (self.recent_underruns as f32 * 0.70) as u32;
            self.last_eval_time = now;
        }

        let mut peak_frame_ms = 16.6f32;
        let mut total_duration_ms = 0.0f32;
        let mut dropped_frames = 0usize;
        let count = self.frame_durations.len();

        if count > 0 {
            for &dur in &self.frame_durations {
                total_duration_ms += dur;
                if dur > peak_frame_ms {
                    peak_frame_ms = dur;
                }
                // Anything slower than 30 FPS (> 33.3ms) or significant jank (> 22.0ms)
                if dur > 22.0 {
                    dropped_frames += 1;
                }
            }
        }

        let avg_frame_ms = if count > 0 && total_duration_ms > 0.0 {
            total_duration_ms / count as f32
        } else {
            16.6
        };

        let average_fps = if avg_frame_ms > 0.0 {
            (1000.0 / avg_frame_ms).clamp(1.0, 240.0)
        } else {
            60.0
        };

        let dropped_frame_ratio = if count > 0 {
            dropped_frames as f32 / count as f32
        } else {
            0.0
        };

        // Instability score calculation:
        // - Audio clipping penalty: up to 0.40
        // - Audio underrun penalty: up to 0.45
        // - Dropped UI frame ratio: up to 0.35
        // - Severe lag spike penalty (> 60ms): up to 0.20
        let clip_penalty = (self.recent_clipping as f32 * 0.20).min(0.40);
        let underrun_penalty = (self.recent_underruns as f32 * 0.25).min(0.45);
        let fps_penalty = (dropped_frame_ratio * 0.50).min(0.35);
        let spike_penalty = if peak_frame_ms > 60.0 { 0.20 } else { 0.0 };

        let instability_score = (clip_penalty + underrun_penalty + fps_penalty + spike_penalty).clamp(0.0, 1.0);

        // When UI is fluid (>= 50 FPS) and underrun count is 0, device hardware is demonstrably NOT overloaded!
        let is_heavy_throttling = (average_fps < 25.0 && count >= 10) || (dropped_frame_ratio > 0.40 && peak_frame_ms > 70.0);
        let has_hardware_buffer_starvation = self.recent_underruns >= 2;

        let is_overloaded = has_hardware_buffer_starvation
            || is_heavy_throttling
            || (instability_score >= OVERLOAD_INSTABILITY_THRESHOLD && self.recent_underruns > 0);

        let recommend_system_codec = is_overloaded;

        let warning_message = if has_hardware_buffer_starvation && is_heavy_throttling {
            Some("System Overloaded: Buffer underruns and heavy throttling detected. Suggest switching to System Codec.".to_string())
        } else if has_hardware_buffer_starvation {
            Some("Audio Buffer Pressure: Buffer underruns detected on device hardware.".to_string())
        } else if is_heavy_throttling {
            Some("Device Strain Detected: Heavy system throttling detected (low FPS / frame pacing drops). Suggest switching to System Codec.".to_string())
        } else {
            None
        };

        StabilityStatus {
            is_overloaded,
            instability_score,
            clipping_count: self.clipping_total,
            underrun_count: self.underrun_total,
            recent_clipping_events: self.recent_clipping,
            recent_underrun_events: self.recent_underruns,
            average_fps,
            dropped_frame_ratio,
            peak_frame_duration_ms: peak_frame_ms,
            warning_message,
            recommend_system_codec,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    fn reset(&mut self) {
        self.clipping_total = 0;
        self.underrun_total = 0;
        self.recent_clipping = 0;
        self.recent_underruns = 0;
        self.frame_durations.clear();
        self.last_eval_time = std::time::Instant::now();
    }
}

static WATCHGUARD: LazyLock<Mutex<WatchguardState>> = LazyLock::new(|| Mutex::new(WatchguardState::new()));

// =========================================================================
// C-ABI Exports for FFI
// =========================================================================

/// Initializes the watchguard subsystem. Returns 0 on success.
#[no_mangle]
pub extern "C" fn watchguard_init() -> i32 {
    let res = std::panic::catch_unwind(|| {
        if let Ok(mut guard) = WATCHGUARD.lock() {
            guard.reset();
            log::info!("Watchguard: Initialized real-time device stability monitor");
            0
        } else {
            -1
        }
    });
    res.unwrap_or(-1)
}

/// Returns the current watchguard version string.
#[no_mangle]
pub extern "C" fn watchguard_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

/// Records an audio event (peak amplitude, underrun flag, clipping flag).
#[no_mangle]
pub extern "C" fn watchguard_record_audio_event(peak_amplitude: f32, is_underrun: i32, is_clipping: i32) {
    let _ = std::panic::catch_unwind(|| {
        if let Ok(mut guard) = WATCHGUARD.lock() {
            guard.record_audio_event(peak_amplitude, is_underrun != 0, is_clipping != 0);
        }
    });
}

/// Records a UI frame duration in milliseconds.
#[no_mangle]
pub extern "C" fn watchguard_record_ui_frame(frame_time_ms: f32) {
    let _ = std::panic::catch_unwind(|| {
        if let Ok(mut guard) = WATCHGUARD.lock() {
            guard.record_ui_frame(frame_time_ms);
        }
    });
}

/// Queries current device stability metrics as a JSON string.
/// The caller must free the returned string via `watchguard_free_string`.
#[no_mangle]
pub extern "C" fn watchguard_get_status_json() -> *const c_char {
    let res = std::panic::catch_unwind(|| {
        let status = if let Ok(mut guard) = WATCHGUARD.lock() {
            guard.evaluate_status()
        } else {
            StabilityStatus::default()
        };

        match serde_json::to_string(&status) {
            Ok(json_str) => match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null(),
            },
            Err(_) => std::ptr::null(),
        }
    });
    res.unwrap_or(std::ptr::null())
}

/// Resets all watchguard metric counters. Returns 0 on success.
#[no_mangle]
pub extern "C" fn watchguard_reset_metrics() -> i32 {
    let res = std::panic::catch_unwind(|| {
        if let Ok(mut guard) = WATCHGUARD.lock() {
            guard.reset();
            0
        } else {
            -1
        }
    });
    res.unwrap_or(-1)
}

/// Frees a CString pointer returned by `watchguard_get_status_json`.
#[no_mangle]
pub extern "C" fn watchguard_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = std::panic::catch_unwind(|| {
            unsafe {
                let _ = CString::from_raw(ptr);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_watchguard_default_state() {
        let mut state = WatchguardState::new();
        let status = state.evaluate_status();
        assert!(!status.is_overloaded);
        assert_eq!(status.clipping_count, 0);
        assert_eq!(status.underrun_count, 0);
        assert!(!status.recommend_system_codec);
        assert!(status.warning_message.is_none());
    }

    #[test]
    fn test_watchguard_clipping_detection() {
        let mut state = WatchguardState::new();
        state.record_audio_event(1.05, false, true);
        state.record_audio_event(1.04, false, true);

        let status = state.evaluate_status();
        assert_eq!(status.clipping_count, 2);
        // Audio clipping alone without hardware buffer underruns or severe UI drops
        // does not falsely flag hardware system overload.
        assert!(!status.is_overloaded);

        // However, if hardware buffer underruns occur alongside clipping, overload is flagged
        state.record_audio_event(0.8, true, false);
        state.record_audio_event(0.8, true, false);
        let overloaded_status = state.evaluate_status();
        assert!(overloaded_status.is_overloaded);
        assert!(overloaded_status.recommend_system_codec);
    }

    #[test]
    fn test_watchguard_underrun_detection() {
        let mut state = WatchguardState::new();
        state.record_audio_event(0.5, true, false);
        state.record_audio_event(0.5, true, false);

        let status = state.evaluate_status();
        assert_eq!(status.underrun_count, 2);
        assert!(status.is_overloaded);
        assert!(status.recommend_system_codec);
    }

    #[test]
    fn test_watchguard_ui_frame_pacing() {
        let mut state = WatchguardState::new();
        // Add 60 smooth frames
        for _ in 0..60 {
            state.record_ui_frame(16.6);
        }
        let smooth = state.evaluate_status();
        assert!((smooth.average_fps - 60.0).abs() < 1.0);
        assert!(!smooth.is_overloaded);

        // Inject severe frame spikes (lag spikes)
        for _ in 0..30 {
            state.record_ui_frame(85.0);
        }
        let laggy = state.evaluate_status();
        assert!(laggy.average_fps < 40.0);
        assert!(laggy.peak_frame_duration_ms >= 85.0);
        assert!(laggy.dropped_frame_ratio > 0.30);
    }

    #[test]
    fn test_watchguard_c_abi_lifecycle() {
        assert_eq!(watchguard_init(), 0);
        let ver_ptr = watchguard_version();
        assert!(!ver_ptr.is_null());
        let ver = unsafe { CStr::from_ptr(ver_ptr).to_str().unwrap() };
        assert_eq!(ver, "0.1.0");

        watchguard_record_audio_event(1.05, 1, 1);
        watchguard_record_audio_event(1.05, 1, 1);
        let json_ptr = watchguard_get_status_json();
        assert!(!json_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(json_ptr).to_str().unwrap() };
        assert!(json_str.contains("\"is_overloaded\":true"));
        assert!(json_str.contains("\"clipping_count\":2"));
        watchguard_free_string(json_ptr as *mut c_char);

        assert_eq!(watchguard_reset_metrics(), 0);
        let reset_json_ptr = watchguard_get_status_json();
        let reset_str = unsafe { CStr::from_ptr(reset_json_ptr).to_str().unwrap() };
        assert!(reset_str.contains("\"clipping_count\":0"));
        watchguard_free_string(reset_json_ptr as *mut c_char);
    }
}
