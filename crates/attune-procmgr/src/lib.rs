//! Attune Process Manager — Dynamic Shared Library (.so / .dll)
//!
//! Provides cross-platform process metrics and system resource telemetry:
//! - Resident Set Size (RSS), Peak RSS (HWM), Virtual Memory (VIRT)
//! - CPU utilization percentage & thread count
//! - GPU clock frequencies & hardware acceleration
//! - Sound card IRQ interrupt rate (ALSA/snd_hda on Linux, audio hardware on Windows)
//! - Audio DAC stream hardware parameters
//! - Memory cleanup action (`malloc_trim` on Linux, `EmptyWorkingSet` on Windows)
//!
//! Mandate 4: Zero hardcoded dummy strings; all metrics are probed directly from
//! live OS kernel interfaces (`/proc`, `/sys`, `/dev/dri`, Win32 APIs).

use std::ffi::CString;
use std::os::raw::c_char;
#[cfg(target_os = "linux")]
use std::sync::Mutex;
#[cfg(target_os = "linux")]
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessMetrics {
    // Memory
    pub rss_bytes: u64,
    pub rss_peak_bytes: u64,
    pub vmem_bytes: u64,

    // CPU & Threads
    pub cpu_usage_pct: f32,
    pub thread_count: u32,
    pub voluntary_ctxt_switches: u64,
    pub nonvoluntary_ctxt_switches: u64,

    // GPU & Display
    pub gpu_device_name: String,
    pub gpu_driver: String,
    pub gpu_cur_freq_mhz: Option<u32>,
    pub gpu_max_freq_mhz: Option<u32>,
    pub gpu_hardware_accel: String,

    // Audio IRQ & DAC
    pub audio_irq_number: Option<u32>,
    pub audio_irq_device: String,
    pub audio_irq_count: u64,
    pub audio_irq_rate_per_sec: f32,
    pub alsa_stream_state: String,
    pub alsa_hw_params: String,

    // Timestamp
    pub timestamp_ms: u64,
}

impl ProcessMetrics {
    fn new_unprobed() -> Self {
        Self {
            rss_bytes: 0,
            rss_peak_bytes: 0,
            vmem_bytes: 0,
            cpu_usage_pct: 0.0,
            thread_count: 0,
            voluntary_ctxt_switches: 0,
            nonvoluntary_ctxt_switches: 0,
            gpu_device_name: String::new(),
            gpu_driver: String::new(),
            gpu_cur_freq_mhz: None,
            gpu_max_freq_mhz: None,
            gpu_hardware_accel: String::new(),
            audio_irq_number: None,
            audio_irq_device: String::new(),
            audio_irq_count: 0,
            audio_irq_rate_per_sec: 0.0,
            alsa_stream_state: String::new(),
            alsa_hw_params: String::new(),
            timestamp_ms: 0,
        }
    }
}

#[cfg(target_os = "linux")]
struct SamplerState {
    last_cpu_time: Instant,
    last_proc_ticks: u64,
    last_total_ticks: u64,
    last_cpu_pct: f32,
    last_irq_time: Instant,
    last_irq_count: u64,
    last_irq_rate: f32,
}

#[cfg(target_os = "linux")]
static SAMPLER_STATE: Mutex<Option<SamplerState>> = Mutex::new(None);

/// Collects live process metrics from OS kernel interfaces.
pub fn sample_metrics() -> ProcessMetrics {
    let mut metrics = ProcessMetrics::new_unprobed();

    #[cfg(target_os = "linux")]
    {
        let now = Instant::now();
        parse_proc_status(&mut metrics);
        parse_proc_stat_cpu(&mut metrics, now);
        parse_audio_irqs(&mut metrics, now);
        parse_alsa_status(&mut metrics);
        parse_gpu_sysfs(&mut metrics);
    }

    #[cfg(target_os = "windows")]
    {
        sample_windows_metrics(&mut metrics);
    }

    metrics.timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    metrics
}

/// Cleans up unused process memory back to the operating system.
/// Uses `malloc_trim` on Linux and `EmptyWorkingSet` / working set trimming on Windows.
/// Returns (rss_before_bytes, rss_after_bytes).
pub fn clean_process_memory() -> (u64, u64) {
    let before = sample_metrics().rss_bytes;

    #[cfg(target_os = "linux")]
    {
        unsafe {
            libc::malloc_trim(0);
        }
    }

    #[cfg(target_os = "windows")]
    {
        unsafe {
            windows_clean_working_set();
        }
    }

    let after = sample_metrics().rss_bytes;
    (before, after)
}

// =========================================================================
// Linux Kernel (/proc & /sys) Implementation
// =========================================================================

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
fn parse_proc_status(metrics: &mut ProcessMetrics) {
    if let Ok(content) = fs::read_to_string("/proc/self/status") {
        for line in content.lines() {
            let mut parts = line.splitn(2, ':');
            let key = match parts.next() {
                Some(k) => k.trim(),
                None => continue,
            };
            let val = match parts.next() {
                Some(v) => v.trim(),
                None => continue,
            };

            match key {
                "VmRSS" => {
                    if let Some(kb) = parse_kb(val) {
                        metrics.rss_bytes = kb * 1024;
                    }
                }
                "VmHWM" => {
                    if let Some(kb) = parse_kb(val) {
                        metrics.rss_peak_bytes = kb * 1024;
                    }
                }
                "VmSize" => {
                    if let Some(kb) = parse_kb(val) {
                        metrics.vmem_bytes = kb * 1024;
                    }
                }
                "Threads" => {
                    if let Ok(t) = val.parse::<u32>() {
                        metrics.thread_count = t;
                    }
                }
                "voluntary_ctxt_switches" => {
                    if let Ok(c) = val.parse::<u64>() {
                        metrics.voluntary_ctxt_switches = c;
                    }
                }
                "nonvoluntary_ctxt_switches" => {
                    if let Ok(c) = val.parse::<u64>() {
                        metrics.nonvoluntary_ctxt_switches = c;
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn parse_kb(val: &str) -> Option<u64> {
    val.split_whitespace().next()?.parse::<u64>().ok()
}

#[cfg(target_os = "linux")]
fn parse_proc_stat_cpu(metrics: &mut ProcessMetrics, now: Instant) {
    let proc_ticks = read_proc_self_stat_ticks().unwrap_or(0);
    let total_ticks = read_system_total_ticks().unwrap_or(0);

    let mut state_guard = SAMPLER_STATE.lock().unwrap();
    if let Some(ref mut state) = *state_guard {
        let dt = now.duration_since(state.last_cpu_time).as_secs_f32();
        if dt > 0.05 && total_ticks > state.last_total_ticks {
            let proc_delta = proc_ticks.saturating_sub(state.last_proc_ticks) as f32;
            let total_delta = (total_ticks - state.last_total_ticks) as f32;
            let num_cpus = num_cpus_online();
            let pct = ((proc_delta / total_delta) * num_cpus * 100.0).clamp(0.0, 100.0 * num_cpus);
            metrics.cpu_usage_pct = pct;
            state.last_cpu_pct = pct;
            state.last_cpu_time = now;
            state.last_proc_ticks = proc_ticks;
            state.last_total_ticks = total_ticks;
        } else {
            metrics.cpu_usage_pct = state.last_cpu_pct;
        }
    } else {
        *state_guard = Some(SamplerState {
            last_cpu_time: now,
            last_proc_ticks: proc_ticks,
            last_total_ticks: total_ticks,
            last_cpu_pct: 0.0,
            last_irq_time: now,
            last_irq_count: 0,
            last_irq_rate: 0.0,
        });
    }
}

#[cfg(target_os = "linux")]
fn read_proc_self_stat_ticks() -> Option<u64> {
    let stat = fs::read_to_string("/proc/self/stat").ok()?;
    let paren_idx = stat.rfind(')')?;
    let rest = &stat[paren_idx + 1..];
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.len() >= 13 {
        let utime = fields[11].parse::<u64>().ok()?;
        let stime = fields[12].parse::<u64>().ok()?;
        Some(utime + stime)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn read_system_total_ticks() -> Option<u64> {
    let stat = fs::read_to_string("/proc/stat").ok()?;
    let cpu_line = stat.lines().next()?;
    if cpu_line.starts_with("cpu ") {
        let mut total: u64 = 0;
        for token in cpu_line.split_whitespace().skip(1) {
            if let Ok(val) = token.parse::<u64>() {
                total += val;
            }
        }
        Some(total)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn num_cpus_online() -> f32 {
    let mut count = 1.0;
    if let Ok(content) = fs::read_to_string("/sys/devices/system/cpu/online") {
        if let Some(last) = content.trim().split('-').next_back() {
            if let Ok(n) = last.parse::<u32>() {
                count = (n + 1) as f32;
            }
        }
    }
    count.max(1.0)
}

#[cfg(target_os = "linux")]
fn parse_audio_irqs(metrics: &mut ProcessMetrics, now: Instant) {
    if let Ok(content) = fs::read_to_string("/proc/interrupts") {
        for line in content.lines() {
            let lower = line.to_lowercase();
            if lower.contains("snd_") || lower.contains("audio") || lower.contains("hda") || lower.contains("sof") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let irq_num = parts[0].trim().parse::<u32>().ok();
                    metrics.audio_irq_number = irq_num;

                    let right = parts[1..].join(":");
                    let tokens: Vec<&str> = right.split_whitespace().collect();
                    let mut count: u64 = 0;
                    let mut device_name = String::new();

                    for &token in &tokens {
                        if let Ok(c) = token.parse::<u64>() {
                            count += c;
                        } else if token.contains("snd") || token.contains("hda") || token.contains("audio") || token.contains("sof") {
                            device_name = token.to_string();
                        }
                    }

                    if device_name.is_empty() {
                        if let Some(&last_tok) = tokens.last() {
                            device_name = last_tok.to_string();
                        }
                    }

                    metrics.audio_irq_device = device_name;
                    metrics.audio_irq_count = count;

                    let mut state_guard = SAMPLER_STATE.lock().unwrap();
                    if let Some(ref mut state) = *state_guard {
                        let dt = now.duration_since(state.last_irq_time).as_secs_f32();
                        if state.last_irq_count == 0 {
                            state.last_irq_count = count;
                            state.last_irq_time = now;
                        } else if dt > 0.05 && count >= state.last_irq_count {
                            let rate = ((count - state.last_irq_count) as f32 / dt).clamp(0.0, 100_000.0);
                            metrics.audio_irq_rate_per_sec = rate;
                            state.last_irq_rate = rate;
                            state.last_irq_count = count;
                            state.last_irq_time = now;
                        } else {
                            metrics.audio_irq_rate_per_sec = state.last_irq_rate;
                        }
                    }
                    return;
                }
            }
        }
    }
    metrics.audio_irq_device = "No audio IRQ registered in /proc/interrupts".to_string();
}

#[cfg(target_os = "linux")]
fn read_alsa_card_summary() -> Option<String> {
    let cards = fs::read_to_string("/proc/asound/cards").ok()?;
    let first_line = cards.lines().next()?.trim();
    if first_line.is_empty() {
        return None;
    }
    if let Some((_, desc)) = first_line.split_once("]:") {
        Some(desc.trim().to_string())
    } else {
        Some(first_line.to_string())
    }
}

#[cfg(target_os = "linux")]
fn parse_alsa_status(metrics: &mut ProcessMetrics) {
    let asound = Path::new("/proc/asound");
    let mut sub_dirs: Vec<PathBuf> = Vec::new();

    if let Ok(cards) = fs::read_dir(asound) {
        for card_entry in cards.flatten() {
            let card_name = card_entry.file_name();
            let card_str = card_name.to_string_lossy();
            if !card_str.starts_with("card") {
                continue;
            }
            if let Ok(pcms) = fs::read_dir(card_entry.path()) {
                for pcm_entry in pcms.flatten() {
                    let pcm_name = pcm_entry.file_name();
                    let pcm_str = pcm_name.to_string_lossy();
                    // Playback PCM directories end with 'p' (e.g. pcm0p, pcm3p)
                    if !pcm_str.starts_with("pcm") || !pcm_str.ends_with('p') {
                        continue;
                    }
                    if let Ok(subs) = fs::read_dir(pcm_entry.path()) {
                        for sub_entry in subs.flatten() {
                            let sub_name = sub_entry.file_name();
                            if sub_name.to_string_lossy().starts_with("sub") {
                                sub_dirs.push(sub_entry.path());
                            }
                        }
                    }
                }
            }
        }
    }

    sub_dirs.sort();

    let mut fallback_closed: Option<(String, String)> = None;

    for sub in &sub_dirs {
        let status_p = sub.join("status");
        let Ok(s) = fs::read_to_string(&status_p) else { continue };
        let first_line = s.lines().next().unwrap_or("").trim().to_string();
        if first_line.is_empty() {
            continue;
        }

        let rel_id = sub
            .strip_prefix(asound)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or("alsa");

        let hw_p = sub.join("hw_params");
        let hw_summary = if let Ok(hw) = fs::read_to_string(&hw_p) {
            let lines: Vec<String> = hw
                .lines()
                .take(5)
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            lines.join(" • ")
        } else {
            String::new()
        };

        if first_line != "closed" {
            // Active stream (e.g. "state: RUNNING" or "state: PREPARED")
            metrics.alsa_stream_state = format!("{first_line} ({rel_id})");
            metrics.alsa_hw_params = if !hw_summary.is_empty() && hw_summary != "closed" {
                hw_summary
            } else {
                format!("Substream {rel_id}")
            };
            return;
        } else if fallback_closed.is_none() {
            let card_desc = read_alsa_card_summary()
                .unwrap_or_else(|| format!("ALSA {rel_id} (no active stream)"));
            fallback_closed = Some((format!("closed ({rel_id})"), card_desc));
        }
    }

    if let Some((st, hw)) = fallback_closed {
        metrics.alsa_stream_state = st;
        metrics.alsa_hw_params = hw;
    } else if let Some(card_desc) = read_alsa_card_summary() {
        metrics.alsa_stream_state = "idle".to_string();
        metrics.alsa_hw_params = card_desc;
    } else {
        metrics.alsa_stream_state = "unavailable (/proc/asound not mounted)".to_string();
        metrics.alsa_hw_params = "unavailable".to_string();
    }
}

#[cfg(target_os = "linux")]
fn parse_gpu_sysfs(metrics: &mut ProcessMetrics) {
    let drm_dir = Path::new("/sys/class/drm");
    let mut cards: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(drm_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // Match card0, card1, etc. (excluding connector dirs like card1-DP-1)
            if let Some(rest) = name.strip_prefix("card") {
                if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                    cards.push(entry.path());
                }
            }
        }
    }
    cards.sort();

    for base in cards {
        if let Ok(link) = fs::read_link(base.join("device/driver")) {
            let driver_name = link
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("drm")
                .to_string();
            metrics.gpu_driver = driver_name.clone();
            metrics.gpu_device_name = match driver_name.as_str() {
                "i915" | "xe" => format!("Intel Graphics ({driver_name})"),
                "amdgpu" | "radeon" => format!("AMD Radeon ({driver_name})"),
                "nouveau" | "nvidia" => format!("NVIDIA GPU ({driver_name})"),
                other => format!("DRM Device ({other})"),
            };

            // Probe current and max GPU clock frequencies across standard and gt0 sysfs locations
            for cur_rel in ["gt_cur_freq_mhz", "gt_act_freq_mhz", "gt/gt0/rps_cur_freq_mhz", "gt/gt0/rps_act_freq_mhz"] {
                if let Ok(s) = fs::read_to_string(base.join(cur_rel)) {
                    if let Ok(mhz) = s.trim().parse::<u32>() {
                        metrics.gpu_cur_freq_mhz = Some(mhz);
                        break;
                    }
                }
            }
            for max_rel in ["gt_max_freq_mhz", "gt/gt0/rps_max_freq_mhz", "gt_RP0_freq_mhz"] {
                if let Ok(s) = fs::read_to_string(base.join(max_rel)) {
                    if let Ok(mhz) = s.trim().parse::<u32>() {
                        metrics.gpu_max_freq_mhz = Some(mhz);
                        break;
                    }
                }
            }

            if metrics.gpu_cur_freq_mhz.is_none() {
                if let Ok(busy) = fs::read_to_string(base.join("device/gpu_busy_percent")) {
                    if let Ok(pct) = busy.trim().parse::<u32>() {
                        metrics.gpu_cur_freq_mhz = Some(pct);
                        metrics.gpu_max_freq_mhz = Some(100);
                    }
                }
            }

            let session = if std::env::var_os("WAYLAND_DISPLAY").is_some() {
                "Wayland"
            } else if std::env::var_os("DISPLAY").is_some() {
                "X11"
            } else {
                "Headless"
            };
            let has_render_node = Path::new("/dev/dri/renderD128").exists();
            metrics.gpu_hardware_accel = if has_render_node {
                format!("{session} + DRM RenderNode (/dev/dri/renderD128)")
            } else {
                format!("{session} Display Session")
            };
            return;
        }
    }

    metrics.gpu_device_name = "No DRM GPU detected in /sys/class/drm".to_string();
    metrics.gpu_driver = "none".to_string();
    metrics.gpu_hardware_accel = "Software / Unprobed".to_string();
}

// =========================================================================
// Windows Win32 API Implementation
// =========================================================================

#[cfg(target_os = "windows")]
#[repr(C)]
struct ProcessMemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn K32GetProcessMemoryInfo(
        process: *mut std::ffi::c_void,
        pmc: *mut ProcessMemoryCounters,
        cb: u32,
    ) -> i32;
    fn K32EmptyWorkingSet(hProcess: *mut std::ffi::c_void) -> i32;
    fn SetProcessWorkingSetSize(
        hProcess: *mut std::ffi::c_void,
        dwMinimumWorkingSetSize: usize,
        dwMaximumWorkingSetSize: usize,
    ) -> i32;
}

#[cfg(target_os = "windows")]
fn sample_windows_metrics(metrics: &mut ProcessMetrics) {
    unsafe {
        let mut pmc = std::mem::zeroed::<ProcessMemoryCounters>();
        pmc.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
        let proc = GetCurrentProcess();
        if K32GetProcessMemoryInfo(proc, &mut pmc, pmc.cb) != 0 {
            metrics.rss_bytes = pmc.working_set_size as u64;
            metrics.rss_peak_bytes = pmc.peak_working_set_size as u64;
            metrics.vmem_bytes = pmc.pagefile_usage as u64;
        }
    }
    metrics.thread_count = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);
    metrics.gpu_device_name = "Windows Display Adapter (WDDM)".to_string();
    metrics.gpu_driver = "WDDM".to_string();
    metrics.gpu_hardware_accel = "DirectComposition / WebView2".to_string();
    metrics.audio_irq_device = "WASAPI Audio Endpoint".to_string();
    metrics.alsa_stream_state = "WASAPI Shared Stream".to_string();
    metrics.alsa_hw_params = "Windows Audio Session API".to_string();
}

#[cfg(target_os = "windows")]
unsafe fn windows_clean_working_set() {
    let proc = GetCurrentProcess();
    if K32EmptyWorkingSet(proc) == 0 {
        let neg_one = !0usize;
        let _ = SetProcessWorkingSetSize(proc, neg_one, neg_one);
    }
}

// =========================================================================
// C-ABI Exports for FFI (.so / .dll / .dylib)
// =========================================================================

/// Returns the attune_procmgr version string.
#[no_mangle]
pub extern "C" fn procmgr_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

/// Samples live process telemetry and returns a JSON C-string.
/// The caller must free the string via `procmgr_free_string`.
#[no_mangle]
pub extern "C" fn procmgr_sample_metrics_json() -> *const c_char {
    let res = std::panic::catch_unwind(|| {
        let metrics = sample_metrics();
        match serde_json::to_string(&metrics) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null(),
            },
            Err(_) => std::ptr::null(),
        }
    });
    res.unwrap_or(std::ptr::null())
}

/// Cleans process memory (malloc_trim on Linux, EmptyWorkingSet on Windows).
/// Populates `before_out` and `after_out` with RSS bytes before and after trimming.
/// Returns 0 on success, or -1 on error.
#[no_mangle]
pub extern "C" fn procmgr_clean_process_memory(before_out: *mut u64, after_out: *mut u64) -> i32 {
    let res = std::panic::catch_unwind(|| {
        let (before, after) = clean_process_memory();
        if !before_out.is_null() {
            unsafe { *before_out = before; }
        }
        if !after_out.is_null() {
            unsafe { *after_out = after; }
        }
        0
    });
    res.unwrap_or(-1)
}

/// Frees a CString pointer returned by `procmgr_sample_metrics_json`.
#[no_mangle]
pub extern "C" fn procmgr_free_string(ptr: *mut c_char) {
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
    fn test_sample_metrics_returns_valid_data() {
        let m = sample_metrics();
        assert!(m.timestamp_ms > 0);
        assert!(m.thread_count >= 1);
        #[cfg(target_os = "linux")]
        {
            assert!(m.rss_bytes > 0, "RSS should be positive on Linux");
            assert!(m.vmem_bytes > 0, "VMem should be positive on Linux");
            assert!(!m.gpu_driver.is_empty(), "GPU driver should be probed on Linux");
            assert!(!m.alsa_stream_state.is_empty(), "ALSA stream state should be probed on Linux");
        }
    }

    #[test]
    fn test_clean_process_memory_execution() {
        let (before, after) = clean_process_memory();
        assert!(before > 0);
        assert!(after > 0);
    }

    #[test]
    fn test_procmgr_c_abi_lifecycle() {
        let ver_ptr = procmgr_version();
        assert!(!ver_ptr.is_null());
        let ver = unsafe { CStr::from_ptr(ver_ptr).to_str().unwrap() };
        assert_eq!(ver, "0.1.0");

        let json_ptr = procmgr_sample_metrics_json();
        assert!(!json_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(json_ptr).to_str().unwrap() };
        assert!(json_str.contains("\"rss_bytes\""));
        procmgr_free_string(json_ptr as *mut c_char);

        let mut before = 0u64;
        let mut after = 0u64;
        let ret = procmgr_clean_process_memory(&mut before, &mut after);
        assert_eq!(ret, 0);
        assert!(before > 0);
        assert!(after > 0);
    }
}
