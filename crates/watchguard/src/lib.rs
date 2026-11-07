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

fn detect_logical_cpu_cores() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

fn detect_system_total_memory_bytes() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return kb.saturating_mul(1024);
                        }
                    }
                }
            }
        }
        let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) };
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };
        if pages > 0 && page_size > 0 {
            return (pages as u64).saturating_mul(page_size as u64);
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::mem::size_of;
        #[repr(C)]
        struct MEMORYSTATUSEX {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }
        extern "system" {
            fn GlobalMemoryStatusEx(stat: *mut MEMORYSTATUSEX) -> i32;
        }
        let mut status = MEMORYSTATUSEX {
            dw_length: size_of::<MEMORYSTATUSEX>() as u32,
            dw_memory_load: 0,
            ull_total_phys: 0,
            ull_avail_phys: 0,
            ull_total_page_file: 0,
            ull_avail_page_file: 0,
            ull_total_virtual: 0,
            ull_avail_virtual: 0,
            ull_avail_extended_virtual: 0,
        };
        if unsafe { GlobalMemoryStatusEx(&mut status) } != 0 {
            return status.ull_total_phys;
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mut mib = [libc::CTL_HW, libc::HW_MEMSIZE];
        let mut memsize: u64 = 0;
        let mut len = std::mem::size_of::<u64>();
        let ret = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                2,
                &mut memsize as *mut _ as *mut libc::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if ret == 0 {
            return memsize;
        }
    }

    0
}

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

    // Desktop hardware resource telemetry
    #[serde(default)]
    pub cpu_usage_pct: f32,
    #[serde(default)]
    pub memory_rss_bytes: u64,
    #[serde(default)]
    pub gpu_usage_pct: f32,
    #[serde(default)]
    pub thread_count: u32,
    #[serde(default)]
    pub is_desktop_mode: bool,
    #[serde(default)]
    pub system_memory_total_bytes: u64,
    #[serde(default)]
    pub logical_cpu_cores: u32,
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
            cpu_usage_pct: 0.0,
            memory_rss_bytes: 0,
            gpu_usage_pct: 0.0,
            thread_count: 0,
            is_desktop_mode: false,
            system_memory_total_bytes: 0,
            logical_cpu_cores: 1,
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

    // Desktop hardware telemetry
    is_desktop_mode: bool,
    last_cpu_pct: f32,
    last_memory_rss_bytes: u64,
    last_gpu_usage_pct: f32,
    last_thread_count: u32,
    system_memory_total_bytes: u64,
    logical_cpu_cores: u32,
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
            is_desktop_mode: false,
            last_cpu_pct: 0.0,
            last_memory_rss_bytes: 0,
            last_gpu_usage_pct: 0.0,
            last_thread_count: 0,
            system_memory_total_bytes: detect_system_total_memory_bytes(),
            logical_cpu_cores: detect_logical_cpu_cores(),
        }
    }

    fn set_desktop_mode(&mut self, is_desktop: bool) {
        self.is_desktop_mode = is_desktop;
    }

    fn record_hardware_resources(
        &mut self,
        cpu_usage_pct: f32,
        memory_rss_bytes: u64,
        gpu_usage_pct: f32,
        thread_count: u32,
    ) {
        self.is_desktop_mode = true;
        self.last_cpu_pct = cpu_usage_pct;
        self.last_memory_rss_bytes = memory_rss_bytes;
        self.last_gpu_usage_pct = gpu_usage_pct;
        self.last_thread_count = thread_count;
        if self.logical_cpu_cores == 0 {
            self.logical_cpu_cores = detect_logical_cpu_cores();
        }
        if self.system_memory_total_bytes == 0 {
            self.system_memory_total_bytes = detect_system_total_memory_bytes();
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

        if self.is_desktop_mode {
            // =============================================================
            // Desktop Stability Evaluation (Kernel Telemetry & Hardware Resource Model)
            // =============================================================
            // On desktop, UI FPS pacing restrictions are bypassed because desktop compositors
            // and high refresh rates legitimately vary without affecting DSP audio playback.
            // Instability is governed strictly by real hardware resources:
            // 1. Audio ring buffer underruns (DAC deadline starvation): up to 0.50 penalty
            // 2. Audio signal clipping: up to 0.25 penalty
            // 3. CPU hardware load / single-core thread saturation: up to 0.35 penalty
            // 4. Memory footprint / system RAM consumption ratio: up to 0.30 penalty
            // 5. GPU utilization & OS thread pressure: up to 0.15 penalty

            let underrun_penalty = (self.recent_underruns as f32 * 0.30).min(0.50);
            let clip_penalty = (self.recent_clipping as f32 * 0.15).min(0.25);

            // CPU load penalty:
            // Normalized whole-system load: self.last_cpu_pct / (cores * 100.0)
            let cores = self.logical_cpu_cores.max(1) as f32;
            let whole_system_cpu_ratio = (self.last_cpu_pct / (cores * 100.0)).clamp(0.0, 1.0);
            // Single-core saturation (e.g. audio thread bottleneck):
            let single_core_saturation = (self.last_cpu_pct / 100.0).clamp(0.0, 1.0);

            let cpu_penalty = if whole_system_cpu_ratio > 0.80 {
                ((whole_system_cpu_ratio - 0.80) / 0.20 * 0.35).min(0.35)
            } else if single_core_saturation > 0.90 && self.recent_underruns > 0 {
                ((single_core_saturation - 0.90) / 0.10 * 0.25).min(0.25)
            } else {
                0.0
            };

            // Memory footprint penalty:
            // Dynamic fraction of physical system RAM (or > 1.5 GB if system memory query unavailable)
            let mem_penalty = if self.system_memory_total_bytes > 0 {
                let mem_ratio = self.last_memory_rss_bytes as f64 / self.system_memory_total_bytes as f64;
                if mem_ratio > 0.35 {
                    (((mem_ratio - 0.35) / 0.40) as f32).clamp(0.0, 0.30)
                } else {
                    0.0
                }
            } else if self.last_memory_rss_bytes > 1_500_000_000 {
                (((self.last_memory_rss_bytes - 1_500_000_000) as f32) / 1_500_000_000.0).clamp(0.0, 0.30)
            } else {
                0.0
            };

            // Thread contention penalty (excessive threads relative to CPU cores)
            let thread_threshold = self.logical_cpu_cores.max(1) * 16;
            let thread_penalty = if self.last_thread_count > thread_threshold {
                (((self.last_thread_count - thread_threshold) as f32) / 64.0).clamp(0.0, 0.15)
            } else {
                0.0
            };

            let instability_score = (underrun_penalty + clip_penalty + cpu_penalty + mem_penalty + thread_penalty).clamp(0.0, 1.0);

            let has_hardware_buffer_starvation = self.recent_underruns >= 2;
            let has_severe_cpu_contention = (whole_system_cpu_ratio > 0.90 || self.last_cpu_pct > 95.0) && self.recent_underruns > 0;
            let has_memory_exhaustion = mem_penalty > 0.20 && self.recent_underruns > 0;

            let is_overloaded = has_hardware_buffer_starvation
                || has_severe_cpu_contention
                || has_memory_exhaustion
                || (instability_score >= OVERLOAD_INSTABILITY_THRESHOLD && self.recent_underruns > 0);

            let recommend_system_codec = is_overloaded;

            let warning_message = if has_hardware_buffer_starvation && has_severe_cpu_contention {
                Some("System Overload: Buffer underruns and CPU starvation detected on audio DAC.".to_string())
            } else if has_hardware_buffer_starvation {
                Some("Audio Buffer Pressure: Buffer underruns detected on audio hardware DAC.".to_string())
            } else if has_severe_cpu_contention {
                Some("CPU Hardware Contention: High process CPU load is impacting audio DSP deadlines.".to_string())
            } else if mem_penalty > 0.20 {
                Some("Memory Bloat Detected: Process memory is consuming an unusually high fraction of system RAM.".to_string())
            } else if self.recent_clipping >= 5 {
                Some("Audio Signal Clipping: Pre-DAC signal level exceeded 0 dBFS ceiling multiple times.".to_string())
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
                cpu_usage_pct: self.last_cpu_pct,
                memory_rss_bytes: self.last_memory_rss_bytes,
                gpu_usage_pct: self.last_gpu_usage_pct,
                thread_count: self.last_thread_count,
                is_desktop_mode: true,
                system_memory_total_bytes: self.system_memory_total_bytes,
                logical_cpu_cores: self.logical_cpu_cores,
            }
        } else {
            // =============================================================
            // Mobile (Android / Flutter) Evaluation
            // =============================================================
            // Preserves existing frame pacing and thermal throttling analysis
            let clip_penalty = (self.recent_clipping as f32 * 0.20).min(0.40);
            let underrun_penalty = (self.recent_underruns as f32 * 0.25).min(0.45);
            let fps_penalty = (dropped_frame_ratio * 0.50).min(0.35);
            let spike_penalty = if peak_frame_ms > 60.0 { 0.20 } else { 0.0 };

            let instability_score = (clip_penalty + underrun_penalty + fps_penalty + spike_penalty).clamp(0.0, 1.0);

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
                cpu_usage_pct: self.last_cpu_pct,
                memory_rss_bytes: self.last_memory_rss_bytes,
                gpu_usage_pct: self.last_gpu_usage_pct,
                thread_count: self.last_thread_count,
                is_desktop_mode: false,
                system_memory_total_bytes: self.system_memory_total_bytes,
                logical_cpu_cores: self.logical_cpu_cores,
            }
        }
    }

    fn reset(&mut self) {
        self.clipping_total = 0;
        self.underrun_total = 0;
        self.recent_clipping = 0;
        self.recent_underruns = 0;
        self.frame_durations.clear();
        self.last_eval_time = std::time::Instant::now();
        self.last_cpu_pct = 0.0;
        self.last_memory_rss_bytes = 0;
        self.last_gpu_usage_pct = 0.0;
        self.last_thread_count = 0;
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

/// Records a UI frame duration in milliseconds (used primarily on mobile for frame pacing).
#[no_mangle]
pub extern "C" fn watchguard_record_ui_frame(frame_time_ms: f32) {
    let _ = std::panic::catch_unwind(|| {
        if let Ok(mut guard) = WATCHGUARD.lock() {
            guard.record_ui_frame(frame_time_ms);
        }
    });
}

/// Records live hardware and process resource telemetry for desktop stability evaluation.
/// Automatically activates desktop stability evaluation mode.
#[no_mangle]
pub extern "C" fn watchguard_record_hardware_resources(
    cpu_usage_pct: f32,
    memory_rss_bytes: u64,
    gpu_usage_pct: f32,
    thread_count: u32,
) {
    let _ = std::panic::catch_unwind(|| {
        if let Ok(mut guard) = WATCHGUARD.lock() {
            guard.record_hardware_resources(cpu_usage_pct, memory_rss_bytes, gpu_usage_pct, thread_count);
        }
    });
}

/// Explicitly configures desktop vs mobile stability evaluation mode.
/// When desktop mode is active (1), UI FPS pacing restrictions are bypassed
/// in favor of real hardware resource analysis (CPU, memory, GPU, DAC buffer underruns).
#[no_mangle]
pub extern "C" fn watchguard_set_desktop_mode(is_desktop: i32) {
    let _ = std::panic::catch_unwind(|| {
        if let Ok(mut guard) = WATCHGUARD.lock() {
            guard.set_desktop_mode(is_desktop != 0);
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

    #[test]
    fn test_watchguard_desktop_mode_bypasses_fps_restrictions() {
        let mut state = WatchguardState::new();
        state.set_desktop_mode(true);

        // Inject 30 severely slow frames (> 100ms, < 10 FPS)
        for _ in 0..30 {
            state.record_ui_frame(120.0);
        }

        // On desktop, slow UI frame pacing alone MUST NEVER trigger is_overloaded or recommend_system_codec!
        let status = state.evaluate_status();
        assert!(!status.is_overloaded, "Desktop mode must not flag overload due to UI frame pacing");
        assert!(!status.recommend_system_codec);
        assert_eq!(status.instability_score, 0.0, "Instability score should be 0.0 when hardware underruns and clipping are 0");

        // Now record real hardware resources in desktop mode
        state.record_hardware_resources(12.5, 150_000_000, 25.0, 16);
        let hw_status = state.evaluate_status();
        assert!(hw_status.is_desktop_mode);
        assert_eq!(hw_status.cpu_usage_pct, 12.5);
        assert_eq!(hw_status.memory_rss_bytes, 150_000_000);
        assert!(!hw_status.is_overloaded);
    }

    #[test]
    fn test_watchguard_desktop_hardware_overload_detection() {
        let mut state = WatchguardState::new();
        state.set_desktop_mode(true);

        // Buffer starvation (>= 2 underruns on physical DAC)
        state.record_audio_event(0.5, true, false);
        state.record_audio_event(0.5, true, false);
        let status = state.evaluate_status();
        assert!(status.is_overloaded);
        assert!(status.recommend_system_codec);
        assert!(status.warning_message.as_ref().unwrap().contains("Buffer underruns"));
    }

    #[test]
    fn test_watchguard_desktop_c_abi() {
        assert_eq!(watchguard_init(), 0);
        watchguard_set_desktop_mode(1);
        watchguard_record_hardware_resources(8.5, 120_000_000, 15.0, 12);

        let json_ptr = watchguard_get_status_json();
        assert!(!json_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(json_ptr).to_str().unwrap() };
        assert!(json_str.contains("\"is_desktop_mode\":true"));
        assert!(json_str.contains("\"cpu_usage_pct\":8.5"));
        assert!(json_str.contains("\"memory_rss_bytes\":120000000"));
        assert!(json_str.contains("\"gpu_usage_pct\":15.0"));
        assert!(json_str.contains("\"thread_count\":12"));
        watchguard_free_string(json_ptr as *mut c_char);
    }
}
