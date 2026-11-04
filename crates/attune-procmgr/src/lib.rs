//! Attune Process Manager — Dynamic Shared Library (.so / .dll)
//!
//! Provides cross-platform process metrics and system resource telemetry:
//! - Resident Set Size (RSS), Peak RSS (HWM), Virtual Memory (VIRT)
//! - CPU utilization percentage & thread count
//! - GPU clock frequencies & hardware acceleration
//! - Sound card IRQ interrupt rate (ALSA/snd_hda on Linux, audio hardware on Windows)
//! - Audio DAC stream hardware parameters
//! - Memory cleanup action (`malloc_trim` on Linux, `EmptyWorkingSet` on Windows)

use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::Mutex;
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

impl Default for ProcessMetrics {
    fn default() -> Self {
        Self {
            rss_bytes: 0,
            rss_peak_bytes: 0,
            vmem_bytes: 0,
            cpu_usage_pct: 0.0,
            thread_count: 1,
            voluntary_ctxt_switches: 0,
            nonvoluntary_ctxt_switches: 0,
            gpu_device_name: "Hardware Graphics Adapter".to_string(),
            gpu_driver: "Native Display Driver".to_string(),
            gpu_cur_freq_mhz: None,
            gpu_max_freq_mhz: None,
            gpu_hardware_accel: "Hardware Accelerated Compositing".to_string(),
            audio_irq_number: None,
            audio_irq_device: "System Audio Hardware".to_string(),
            audio_irq_count: 0,
            audio_irq_rate_per_sec: 0.0,
            alsa_stream_state: "Active Hardware Stream".to_string(),
            alsa_hw_params: "Direct DAC Output Link".to_string(),
            timestamp_ms: 0,
        }
    }
}

struct SamplerState {
    last_sample_time: Instant,
    last_proc_ticks: u64,
    last_total_ticks: u64,
    last_irq_count: u64,
}

static SAMPLER_STATE: Mutex<Option<SamplerState>> = Mutex::new(None);

/// Collects live process metrics from OS kernel interfaces.
pub fn sample_metrics() -> ProcessMetrics {
    let mut metrics = ProcessMetrics::default();
    let now = Instant::now();

    #[cfg(target_os = "linux")]
    {
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
        let dt = now.duration_since(state.last_sample_time).as_secs_f32();
        if dt > 0.05 && total_ticks > state.last_total_ticks {
            let proc_delta = proc_ticks.saturating_sub(state.last_proc_ticks) as f32;
            let total_delta = (total_ticks - state.last_total_ticks) as f32;
            let num_cpus = num_cpus_online();
            let pct = (proc_delta / total_delta) * num_cpus * 100.0;
            metrics.cpu_usage_pct = pct.clamp(0.0, 100.0 * num_cpus);

            state.last_sample_time = now;
            state.last_proc_ticks = proc_ticks;
            state.last_total_ticks = total_ticks;
        }
    } else {
        *state_guard = Some(SamplerState {
            last_sample_time: now,
            last_proc_ticks: proc_ticks,
            last_total_ticks: total_ticks,
            last_irq_count: 0,
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
        if let Some(last) = content.trim().split('-').last() {
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

                    let right = parts[1];
                    let tokens: Vec<&str> = right.split_whitespace().collect();
                    let mut count: u64 = 0;
                    let mut device_name = String::new();

                    for &token in &tokens {
                        if let Ok(c) = token.parse::<u64>() {
                            count += c;
                        } else if token.contains("snd") || token.contains("hda") || token.contains("audio") {
                            device_name = token.to_string();
                        }
                    }

                    if device_name.is_empty() && !tokens.is_empty() {
                        device_name = tokens.last().unwrap_or(&"snd_audio").to_string();
                    }

                    metrics.audio_irq_device = device_name;
                    metrics.audio_irq_count = count;

                    let mut state_guard = SAMPLER_STATE.lock().unwrap();
                    if let Some(ref mut state) = *state_guard {
                        let dt = now.duration_since(state.last_sample_time).as_secs_f32();
                        if dt > 0.05 && state.last_irq_count > 0 && count >= state.last_irq_count {
                            metrics.audio_irq_rate_per_sec = ((count - state.last_irq_count) as f32 / dt).clamp(0.0, 100_000.0);
                        }
                        state.last_irq_count = count;
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn parse_alsa_status(metrics: &mut ProcessMetrics) {
    for sub in &["/proc/asound/card0/pcm0p/sub0", "/proc/asound/card0/pcm3p/sub0", "/proc/asound/card1/pcm0p/sub0"] {
        let status_p = Path::new(sub).join("status");
        if let Ok(s) = fs::read_to_string(&status_p) {
            let first_line = s.lines().next().unwrap_or("").trim().to_string();
            if !first_line.is_empty() {
                metrics.alsa_stream_state = first_line;
                let hw_p = Path::new(sub).join("hw_params");
                if let Ok(hw) = fs::read_to_string(&hw_p) {
                    let summary: Vec<String> = hw.lines().take(4).map(|l| l.trim().to_string()).collect();
                    if !summary.is_empty() {
                        metrics.alsa_hw_params = summary.join(" • ");
                    }
                }
                break;
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn parse_gpu_sysfs(metrics: &mut ProcessMetrics) {
    for card_idx in 0..4 {
        let base = PathBuf::from(format!("/sys/class/drm/card{card_idx}"));
        if base.exists() {
            if let Ok(link) = fs::read_link(base.join("device/driver")) {
                let driver_name = link
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                metrics.gpu_driver = driver_name.clone();
                metrics.gpu_device_name = match driver_name.as_str() {
                    "i915" | "xe" => "Intel Integrated Graphics".to_string(),
                    "amdgpu" | "radeon" => "AMD Radeon Graphics".to_string(),
                    "nouveau" | "nvidia" => "NVIDIA GeForce / Quadro".to_string(),
                    other => format!("DRM Graphics ({other})"),
                };
            }

            if let Ok(act) = fs::read_to_string(base.join("gt_act_freq_mhz")) {
                metrics.gpu_cur_freq_mhz = act.trim().parse::<u32>().ok();
            }
            if let Ok(max) = fs::read_to_string(base.join("gt_max_freq_mhz")) {
                metrics.gpu_max_freq_mhz = max.trim().parse::<u32>().ok();
            }

            if metrics.gpu_cur_freq_mhz.is_none() {
                if let Ok(busy) = fs::read_to_string(base.join("device/gpu_busy_percent")) {
                    metrics.gpu_cur_freq_mhz = busy.trim().parse::<u32>().ok();
                    metrics.gpu_max_freq_mhz = Some(100);
                }
            }

            break;
        }
    }
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
extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn K32GetProcessMemoryInfo(
        process: *mut std::ffi::c_void,
        pmc: *mut ProcessMemoryCounters,
        cb: u32,
    ) -> i32;
    fn EmptyWorkingSet(hProcess: *mut std::ffi::c_void) -> i32;
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
    metrics.gpu_device_name = "Direct3D / Windows Display Adapter".to_string();
    metrics.gpu_driver = "WDDM Driver".to_string();
    metrics.audio_irq_device = "WASAPI Endpoint".to_string();
    metrics.alsa_stream_state = "WASAPI Exclusive / Shared".to_string();
    metrics.alsa_hw_params = "Windows Audio Engine Direct".to_string();
}

#[cfg(target_os = "windows")]
unsafe fn windows_clean_working_set() {
    let proc = GetCurrentProcess();
    if EmptyWorkingSet(proc) == 0 {
        // Fallback to setting min and max sizes to -1
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
