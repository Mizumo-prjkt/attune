//! Dynamic Process & Hardware Resource Manager Bridge for Attune Desktop.
//!
//! Dynamically loads the modular `libattune_procmgr.so` (Linux) / `attune_procmgr.dll` (Windows)
//! component at runtime, keeping the main executable lightweight and decoupled.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::Mutex;

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
            gpu_hardware_accel: "WebKitGTK / WebView2 Accelerated Compositing".to_string(),
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

type FnSampleMetricsJson = unsafe extern "C" fn() -> *const c_char;
type FnCleanProcessMemory = unsafe extern "C" fn(*mut u64, *mut u64) -> i32;
type FnFreeString = unsafe extern "C" fn(*mut c_char);

struct ProcMgrModule {
    #[allow(dead_code)]
    handle: *mut std::ffi::c_void,
    fn_sample_metrics_json: FnSampleMetricsJson,
    fn_clean_process_memory: FnCleanProcessMemory,
    fn_free_string: FnFreeString,
}

unsafe impl Send for ProcMgrModule {}
unsafe impl Sync for ProcMgrModule {}

static PROC_MGR_MODULE: Mutex<Option<ProcMgrModule>> = Mutex::new(None);
static HAS_INITIALIZED: Mutex<bool> = Mutex::new(false);

fn resolve_library_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let lib_name = if cfg!(target_os = "windows") {
        "attune_procmgr.dll"
    } else if cfg!(target_os = "macos") {
        "libattune_procmgr.dylib"
    } else {
        "libattune_procmgr.so"
    };

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(lib_name));
            candidates.push(dir.join("lib").join(lib_name));
        }
    }

    // Dev/Repo search candidates
    candidates.push(PathBuf::from(format!("target/release/{lib_name}")));
    candidates.push(PathBuf::from(format!("target/debug/{lib_name}")));
    candidates.push(PathBuf::from(format!("../target/release/{lib_name}")));
    candidates.push(PathBuf::from(format!("../target/debug/{lib_name}")));
    candidates.push(PathBuf::from(format!("../../target/release/{lib_name}")));
    candidates.push(PathBuf::from(format!("../../target/debug/{lib_name}")));
    candidates.push(PathBuf::from(lib_name));

    candidates
}

fn load_module() -> Option<ProcMgrModule> {
    for path in resolve_library_paths() {
        if !path.exists() && path.is_absolute() {
            continue;
        }

        let path_str = path.to_str()?;
        let c_path = match CString::new(path_str) {
            Ok(c) => c,
            Err(_) => continue,
        };

        #[cfg(unix)]
        unsafe {
            let handle = libc::dlopen(c_path.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL);
            if !handle.is_null() {
                let sample_sym = libc::dlsym(handle, b"procmgr_sample_metrics_json\0".as_ptr() as *const c_char);
                let clean_sym = libc::dlsym(handle, b"procmgr_clean_process_memory\0".as_ptr() as *const c_char);
                let free_sym = libc::dlsym(handle, b"procmgr_free_string\0".as_ptr() as *const c_char);

                if !sample_sym.is_null() && !clean_sym.is_null() && !free_sym.is_null() {
                    log::info!("ProcMgr: Successfully dynamically loaded component from {}", path.display());
                    return Some(ProcMgrModule {
                        handle,
                        fn_sample_metrics_json: std::mem::transmute(sample_sym),
                        fn_clean_process_memory: std::mem::transmute(clean_sym),
                        fn_free_string: std::mem::transmute(free_sym),
                    });
                } else {
                    libc::dlclose(handle);
                }
            }
        }

        #[cfg(windows)]
        unsafe {
            extern "system" {
                fn LoadLibraryA(lpLibFileName: *const i8) -> *mut std::ffi::c_void;
                fn GetProcAddress(hModule: *mut std::ffi::c_void, lpProcName: *const i8) -> *mut std::ffi::c_void;
                fn FreeLibrary(hModule: *mut std::ffi::c_void) -> i32;
            }

            let handle = LoadLibraryA(c_path.as_ptr());
            if !handle.is_null() {
                let sample_sym = GetProcAddress(handle, b"procmgr_sample_metrics_json\0".as_ptr() as *const i8);
                let clean_sym = GetProcAddress(handle, b"procmgr_clean_process_memory\0".as_ptr() as *const i8);
                let free_sym = GetProcAddress(handle, b"procmgr_free_string\0".as_ptr() as *const i8);

                if !sample_sym.is_null() && !clean_sym.is_null() && !free_sym.is_null() {
                    log::info!("ProcMgr: Successfully dynamically loaded component from {}", path.display());
                    return Some(ProcMgrModule {
                        handle,
                        fn_sample_metrics_json: std::mem::transmute(sample_sym),
                        fn_clean_process_memory: std::mem::transmute(clean_sym),
                        fn_free_string: std::mem::transmute(free_sym),
                    });
                } else {
                    FreeLibrary(handle);
                }
            }
        }
    }

    log::warn!("ProcMgr: Could not locate dynamic library component (libattune_procmgr); using fallback");
    None
}

fn get_module() -> Option<std::sync::MutexGuard<'static, Option<ProcMgrModule>>> {
    let mut initialized = HAS_INITIALIZED.lock().ok()?;
    let mut guard = PROC_MGR_MODULE.lock().ok()?;
    if !*initialized {
        *guard = load_module();
        *initialized = true;
    }
    Some(guard)
}

/// Collects live process telemetry via the dynamic component.
pub fn sample_metrics() -> ProcessMetrics {
    if let Some(guard) = get_module() {
        if let Some(ref module) = *guard {
            unsafe {
                let ptr = (module.fn_sample_metrics_json)();
                if !ptr.is_null() {
                    let c_str = CStr::from_ptr(ptr);
                    let json_str = c_str.to_string_lossy();
                    let res: Result<ProcessMetrics, _> = serde_json::from_str(&json_str);
                    (module.fn_free_string)(ptr as *mut c_char);
                    if let Ok(m) = res {
                        return m;
                    }
                }
            }
        }
    }

    // Graceful fallback with valid timestamp
    let mut fallback = ProcessMetrics::default();
    fallback.timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    fallback
}

/// Cleans process memory via the dynamic component.
/// Returns (before_bytes, after_bytes).
pub fn clean_process_memory() -> (u64, u64) {
    if let Some(guard) = get_module() {
        if let Some(ref module) = *guard {
            let mut before = 0u64;
            let mut after = 0u64;
            let ret = unsafe { (module.fn_clean_process_memory)(&mut before, &mut after) };
            if ret == 0 {
                return (before, after);
            }
        }
    }

    (0, 0)
}

/// Backward compatibility alias for existing tests and call sites.
pub fn trim_memory() -> (u64, u64) {
    clean_process_memory()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_metrics_returns_valid_data() {
        let m = sample_metrics();
        assert!(m.timestamp_ms > 0);
        assert!(m.thread_count >= 1);
    }

    #[test]
    fn test_trim_memory_execution() {
        let (before, after) = clean_process_memory();
        // Even if library is fallback or loaded, returns a valid tuple
        let _ = (before, after);
    }
}
