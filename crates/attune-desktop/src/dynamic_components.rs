//! Dynamic Shared Library (.so / .dll) Component Resolver & Preflight Verifier.
//!
//! Attune Desktop strictly decouples platform telemetry, stability monitoring,
//! and track metadata manipulation into modular shared libraries:
//! - `libattune_procmgr.so` / `attune_procmgr.dll`
//! - `libattune_watchguard.so` / `attune_watchguard.dll`
//! - `libattune_music_metadata_changer.so` / `attune_music_metadata_changer.dll`
//!
//! Mandate 4: Zero hardcoded fallback values or silent static fallbacks. If any
//! required shared library or C-ABI symbol is missing or fails at runtime, the
//! error is surfaced transparently.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Details of a missing or failed shared library component.
#[derive(Debug, Clone)]
pub struct MissingComponentError {
    pub component_name: String,
    pub library_filename: String,
    pub searched_paths: Vec<PathBuf>,
    pub error_details: String,
}

// ---------------------------------------------------------------------------
// Component Function Pointer Types
// ---------------------------------------------------------------------------

// 1. attune_procmgr
type FnProcmgrVersion = unsafe extern "C" fn() -> *const c_char;
type FnProcmgrSampleMetricsJson = unsafe extern "C" fn() -> *const c_char;
type FnProcmgrCleanProcessMemory = unsafe extern "C" fn(*mut u64, *mut u64) -> i32;
type FnProcmgrFreeString = unsafe extern "C" fn(*mut c_char);

struct ProcmgrModule {
    _handle: *mut std::ffi::c_void,
    resolved_path: PathBuf,
    fn_version: FnProcmgrVersion,
    fn_sample_metrics_json: FnProcmgrSampleMetricsJson,
    fn_clean_process_memory: FnProcmgrCleanProcessMemory,
    fn_free_string: FnProcmgrFreeString,
}
unsafe impl Send for ProcmgrModule {}
unsafe impl Sync for ProcmgrModule {}

// 2. attune_watchguard
type FnWatchguardInit = unsafe extern "C" fn() -> i32;
type FnWatchguardVersion = unsafe extern "C" fn() -> *const c_char;
type FnWatchguardRecordAudioEvent = unsafe extern "C" fn(f32, i32, i32);
type FnWatchguardRecordUiFrame = unsafe extern "C" fn(f32);
type FnWatchguardGetStatusJson = unsafe extern "C" fn() -> *const c_char;
type FnWatchguardResetMetrics = unsafe extern "C" fn() -> i32;
type FnWatchguardFreeString = unsafe extern "C" fn(*mut c_char);
type FnWatchguardRecordHardwareResources = unsafe extern "C" fn(f32, u64, f32, u32);
type FnWatchguardSetDesktopMode = unsafe extern "C" fn(i32);

struct WatchguardModule {
    _handle: *mut std::ffi::c_void,
    resolved_path: PathBuf,
    fn_init: FnWatchguardInit,
    fn_version: FnWatchguardVersion,
    fn_record_audio_event: FnWatchguardRecordAudioEvent,
    #[allow(dead_code)]
    fn_record_ui_frame: FnWatchguardRecordUiFrame,
    fn_get_status_json: FnWatchguardGetStatusJson,
    fn_reset_metrics: FnWatchguardResetMetrics,
    fn_free_string: FnWatchguardFreeString,
    fn_record_hardware_resources: Option<FnWatchguardRecordHardwareResources>,
    fn_set_desktop_mode: Option<FnWatchguardSetDesktopMode>,
}
unsafe impl Send for WatchguardModule {}
unsafe impl Sync for WatchguardModule {}

// 3. attune_music_metadata_changer
type FnMetadataVersion = unsafe extern "C" fn() -> *const c_char;
type FnMetadataFreeString = unsafe extern "C" fn(*mut c_char);
type FnMetadataReadMetadata = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type FnMetadataWriteMetadata = unsafe extern "C" fn(*const c_char, *const c_char) -> i32;
type FnMetadataUpdateTag = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> i32;

struct MetadataModule {
    _handle: *mut std::ffi::c_void,
    resolved_path: PathBuf,
    fn_version: FnMetadataVersion,
    fn_free_string: FnMetadataFreeString,
    fn_read_metadata: FnMetadataReadMetadata,
    fn_write_metadata: FnMetadataWriteMetadata,
    fn_update_tag: FnMetadataUpdateTag,
}
unsafe impl Send for MetadataModule {}
unsafe impl Sync for MetadataModule {}

// ---------------------------------------------------------------------------
// Global Loaded Modules Registry
// ---------------------------------------------------------------------------

static PROCMGR: Mutex<Option<ProcmgrModule>> = Mutex::new(None);
static WATCHGUARD: Mutex<Option<WatchguardModule>> = Mutex::new(None);
static METADATA: Mutex<Option<MetadataModule>> = Mutex::new(None);

/// Generate cross-platform filename for a component.
pub fn component_filename(base_name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{base_name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{base_name}.dylib")
    } else {
        format!("lib{base_name}.so")
    }
}

/// Compute resolution search candidates for a component library.
/// Respects `ATTUNE_LIB_DIR` environment override first, followed by executable-relative,
/// working-directory-relative, Cargo target directories, system paths, and OS loader path.
pub fn resolve_candidate_paths(base_name: &str) -> Vec<PathBuf> {
    let filename = component_filename(base_name);
    let mut candidates = Vec::new();

    // 0. Environment variable override (Mandate 4: environment-driven resolution)
    if let Some(env_dir) = std::env::var_os("ATTUNE_LIB_DIR") {
        candidates.push(PathBuf::from(env_dir).join(&filename));
    }

    // 1. Executable parent and <exe>/lib
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("lib").join(&filename));
            candidates.push(exe_dir.join(&filename));
        }
    }

    // 2. Working directory ./lib and ./
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("lib").join(&filename));
        candidates.push(cwd.join(&filename));
    }

    // 3. Cargo dev/release build targets
    candidates.push(PathBuf::from(format!("target/release/{filename}")));
    candidates.push(PathBuf::from(format!("target/debug/{filename}")));
    candidates.push(PathBuf::from(format!("../target/release/{filename}")));
    candidates.push(PathBuf::from(format!("../target/debug/{filename}")));
    candidates.push(PathBuf::from(format!("../../target/release/{filename}")));
    candidates.push(PathBuf::from(format!("../../target/debug/{filename}")));

    // 4. Standard Linux system installation paths
    #[cfg(unix)]
    {
        candidates.push(PathBuf::from(format!("/usr/lib/attune/{filename}")));
        candidates.push(PathBuf::from(format!("/usr/local/lib/attune/{filename}")));
    }

    // 5. Bare filename for OS default loader (LD_LIBRARY_PATH / PATH)
    candidates.push(PathBuf::from(&filename));

    // Deduplicate while preserving search order
    let mut unique = Vec::new();
    for c in candidates {
        if !unique.contains(&c) {
            unique.push(c);
        }
    }
    unique
}

// ---------------------------------------------------------------------------
// Dynamic Library OS Loader
// ---------------------------------------------------------------------------

#[cfg(unix)]
unsafe fn os_dlopen(path_str: &str) -> Result<*mut std::ffi::c_void, String> {
    let c_path = CString::new(path_str).map_err(|e| e.to_string())?;
    let handle = libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
    if handle.is_null() {
        let err_ptr = libc::dlerror();
        let err_msg = if !err_ptr.is_null() {
            CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
        } else {
            "Unknown dlopen error".to_string()
        };
        Err(err_msg)
    } else {
        Ok(handle)
    }
}

#[cfg(unix)]
unsafe fn os_dlsym(handle: *mut std::ffi::c_void, symbol: &str) -> Result<*mut std::ffi::c_void, String> {
    let c_symbol = CString::new(symbol).map_err(|e| e.to_string())?;
    let sym = libc::dlsym(handle, c_symbol.as_ptr());
    if sym.is_null() {
        let err_ptr = libc::dlerror();
        let err_msg = if !err_ptr.is_null() {
            CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
        } else {
            format!("Symbol '{symbol}' not found")
        };
        Err(err_msg)
    } else {
        Ok(sym)
    }
}

#[cfg(unix)]
unsafe fn os_dlclose(handle: *mut std::ffi::c_void) {
    if !handle.is_null() {
        libc::dlclose(handle);
    }
}

#[cfg(windows)]
unsafe fn os_dlopen(path_str: &str) -> Result<*mut std::ffi::c_void, String> {
    extern "system" {
        fn LoadLibraryA(lpLibFileName: *const i8) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
    }
    let c_path = CString::new(path_str).map_err(|e| e.to_string())?;
    let handle = LoadLibraryA(c_path.as_ptr() as *const i8);
    if handle.is_null() {
        let code = GetLastError();
        Err(format!("LoadLibraryA failed with error code: {code}"))
    } else {
        Ok(handle)
    }
}

#[cfg(windows)]
unsafe fn os_dlsym(handle: *mut std::ffi::c_void, symbol: &str) -> Result<*mut std::ffi::c_void, String> {
    extern "system" {
        fn GetProcAddress(hModule: *mut std::ffi::c_void, lpProcName: *const i8) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
    }
    let c_symbol = CString::new(symbol).map_err(|e| e.to_string())?;
    let sym = GetProcAddress(handle, c_symbol.as_ptr() as *const i8);
    if sym.is_null() {
        let code = GetLastError();
        Err(format!("GetProcAddress for '{symbol}' failed with error code: {code}"))
    } else {
        Ok(sym)
    }
}

#[cfg(windows)]
unsafe fn os_dlclose(handle: *mut std::ffi::c_void) {
    extern "system" {
        fn FreeLibrary(hLibModule: *mut std::ffi::c_void) -> i32;
    }
    if !handle.is_null() {
        FreeLibrary(handle);
    }
}

// ---------------------------------------------------------------------------
// Preflight Verification & Loader
// ---------------------------------------------------------------------------

/// Verifies and loads all required dynamic shared library components:
/// - `attune_procmgr`
/// - `attune_watchguard`
/// - `attune_music_metadata_changer`
///
/// Returns `Ok(())` if all components loaded and exported their required symbols.
/// Returns `Err(Vec<MissingComponentError>)` if any component could not be loaded.
pub fn preflight_check_and_load() -> Result<(), Vec<MissingComponentError>> {
    let mut errors = Vec::new();

    if let Err(err) = load_procmgr() {
        errors.push(err);
    }

    if let Err(err) = load_watchguard() {
        errors.push(err);
    }

    if let Err(err) = load_metadata_changer() {
        errors.push(err);
    }

    if errors.is_empty() {
        log::info!("Dynamic Components: All required shared libraries successfully loaded and verified.");
        Ok(())
    } else {
        Err(errors)
    }
}

fn format_candidate_errors(specific_errors: &[String], fallback_error: Option<String>) -> String {
    if !specific_errors.is_empty() {
        specific_errors.join(" | ")
    } else {
        fallback_error.unwrap_or_else(|| "File not found in any candidate search path".to_string())
    }
}

fn load_procmgr() -> Result<(), MissingComponentError> {
    let base = "attune_procmgr";
    let filename = component_filename(base);
    let candidates = resolve_candidate_paths(base);
    let mut specific_errors = Vec::new();
    let mut fallback_error = None;

    for path in &candidates {
        let is_bare = path.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true);
        if !is_bare && !path.exists() {
            continue;
        }
        let Some(path_str) = path.to_str() else { continue };

        unsafe {
            match os_dlopen(path_str) {
                Ok(handle) => {
                    let v_sym = os_dlsym(handle, "procmgr_version");
                    let m_sym = os_dlsym(handle, "procmgr_sample_metrics_json");
                    let c_sym = os_dlsym(handle, "procmgr_clean_process_memory");
                    let f_sym = os_dlsym(handle, "procmgr_free_string");

                    match (v_sym, m_sym, c_sym, f_sym) {
                        (Ok(v), Ok(m), Ok(c), Ok(f)) => {
                            let module = ProcmgrModule {
                                _handle: handle,
                                resolved_path: path.clone(),
                                fn_version: std::mem::transmute(v),
                                fn_sample_metrics_json: std::mem::transmute(m),
                                fn_clean_process_memory: std::mem::transmute(c),
                                fn_free_string: std::mem::transmute(f),
                            };
                            let ver_ptr = (module.fn_version)();
                            let ver_str = if !ver_ptr.is_null() {
                                CStr::from_ptr(ver_ptr).to_string_lossy().into_owned()
                            } else {
                                "unknown".to_string()
                            };
                            log::info!(
                                "Dynamic Component Loaded: {} (v{}) from {}",
                                filename,
                                ver_str,
                                path.display()
                            );
                            let mut guard = PROCMGR.lock().unwrap();
                            *guard = Some(module);
                            return Ok(());
                        }
                        (v, m, c, f) => {
                            let mut missing = Vec::new();
                            if let Err(e) = v { missing.push(e); }
                            if let Err(e) = m { missing.push(e); }
                            if let Err(e) = c { missing.push(e); }
                            if let Err(e) = f { missing.push(e); }
                            os_dlclose(handle);
                            specific_errors.push(format!(
                                "{}: symbol resolution failed ({})",
                                path.display(),
                                missing.join(", ")
                            ));
                        }
                    }
                }
                Err(e) => {
                    if !is_bare {
                        specific_errors.push(format!("{}: dlopen failed ({e})", path.display()));
                    } else {
                        fallback_error = Some(format!("dlopen failed: {e}"));
                    }
                }
            }
        }
    }

    Err(MissingComponentError {
        component_name: "Attune Hardware & Process Manager Telemetry".to_string(),
        library_filename: filename,
        searched_paths: candidates,
        error_details: format_candidate_errors(&specific_errors, fallback_error),
    })
}

fn load_watchguard() -> Result<(), MissingComponentError> {
    let base = "attune_watchguard";
    let filename = component_filename(base);
    let candidates = resolve_candidate_paths(base);
    let mut specific_errors = Vec::new();
    let mut fallback_error = None;

    for path in &candidates {
        let is_bare = path.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true);
        if !is_bare && !path.exists() {
            continue;
        }
        let Some(path_str) = path.to_str() else { continue };

        unsafe {
            match os_dlopen(path_str) {
                Ok(handle) => {
                    let init_sym = os_dlsym(handle, "watchguard_init");
                    let ver_sym = os_dlsym(handle, "watchguard_version");
                    let audio_sym = os_dlsym(handle, "watchguard_record_audio_event");
                    let ui_sym = os_dlsym(handle, "watchguard_record_ui_frame");
                    let status_sym = os_dlsym(handle, "watchguard_get_status_json");
                    let reset_sym = os_dlsym(handle, "watchguard_reset_metrics");
                    let free_sym = os_dlsym(handle, "watchguard_free_string");
                    let hw_sym = os_dlsym(handle, "watchguard_record_hardware_resources").ok();
                    let mode_sym = os_dlsym(handle, "watchguard_set_desktop_mode").ok();

                    match (init_sym, ver_sym, audio_sym, ui_sym, status_sym, reset_sym, free_sym) {
                        (Ok(i), Ok(v), Ok(a), Ok(u), Ok(s), Ok(r), Ok(f)) => {
                            let module = WatchguardModule {
                                _handle: handle,
                                resolved_path: path.clone(),
                                fn_init: std::mem::transmute(i),
                                fn_version: std::mem::transmute(v),
                                fn_record_audio_event: std::mem::transmute(a),
                                fn_record_ui_frame: std::mem::transmute(u),
                                fn_get_status_json: std::mem::transmute(s),
                                fn_reset_metrics: std::mem::transmute(r),
                                fn_free_string: std::mem::transmute(f),
                                fn_record_hardware_resources: hw_sym.map(|sym| std::mem::transmute(sym)),
                                fn_set_desktop_mode: mode_sym.map(|sym| std::mem::transmute(sym)),
                            };
                            let init_rc = (module.fn_init)();
                            if init_rc != 0 {
                                os_dlclose(handle);
                                specific_errors.push(format!(
                                    "{}: watchguard_init returned non-zero status ({init_rc})",
                                    path.display()
                                ));
                                continue;
                            }
                            if let Some(set_mode) = module.fn_set_desktop_mode {
                                set_mode(1);
                            }
                            let ver_ptr = (module.fn_version)();
                            let ver_str = if !ver_ptr.is_null() {
                                CStr::from_ptr(ver_ptr).to_string_lossy().into_owned()
                            } else {
                                "unknown".to_string()
                            };
                            log::info!(
                                "Dynamic Component Loaded: {} (v{}) from {}",
                                filename,
                                ver_str,
                                path.display()
                            );
                            let mut guard = WATCHGUARD.lock().unwrap();
                            *guard = Some(module);
                            return Ok(());
                        }
                        (i, v, a, u, s, r, f) => {
                            let mut missing = Vec::new();
                            if let Err(e) = i { missing.push(e); }
                            if let Err(e) = v { missing.push(e); }
                            if let Err(e) = a { missing.push(e); }
                            if let Err(e) = u { missing.push(e); }
                            if let Err(e) = s { missing.push(e); }
                            if let Err(e) = r { missing.push(e); }
                            if let Err(e) = f { missing.push(e); }
                            os_dlclose(handle);
                            specific_errors.push(format!(
                                "{}: symbol resolution failed ({})",
                                path.display(),
                                missing.join(", ")
                            ));
                        }
                    }
                }
                Err(e) => {
                    if !is_bare {
                        specific_errors.push(format!("{}: dlopen failed ({e})", path.display()));
                    } else {
                        fallback_error = Some(format!("dlopen failed: {e}"));
                    }
                }
            }
        }
    }

    Err(MissingComponentError {
        component_name: "Attune Stability & Device Overhead Watchguard".to_string(),
        library_filename: filename,
        searched_paths: candidates,
        error_details: format_candidate_errors(&specific_errors, fallback_error),
    })
}

fn load_metadata_changer() -> Result<(), MissingComponentError> {
    let base = "attune_music_metadata_changer";
    let filename = component_filename(base);
    let candidates = resolve_candidate_paths(base);
    let mut specific_errors = Vec::new();
    let mut fallback_error = None;

    for path in &candidates {
        let is_bare = path.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true);
        if !is_bare && !path.exists() {
            continue;
        }
        let Some(path_str) = path.to_str() else { continue };

        unsafe {
            match os_dlopen(path_str) {
                Ok(handle) => {
                    let v_sym = os_dlsym(handle, "attune_track_tweaks_version");
                    let f_sym = os_dlsym(handle, "attune_track_tweaks_free_string");
                    let r_sym = os_dlsym(handle, "attune_track_tweaks_read_metadata");
                    let w_sym = os_dlsym(handle, "attune_track_tweaks_write_metadata");
                    let u_sym = os_dlsym(handle, "attune_track_tweaks_update_tag");

                    match (v_sym, f_sym, r_sym, w_sym, u_sym) {
                        (Ok(v), Ok(f), Ok(r), Ok(w), Ok(u)) => {
                            let module = MetadataModule {
                                _handle: handle,
                                resolved_path: path.clone(),
                                fn_version: std::mem::transmute(v),
                                fn_free_string: std::mem::transmute(f),
                                fn_read_metadata: std::mem::transmute(r),
                                fn_write_metadata: std::mem::transmute(w),
                                fn_update_tag: std::mem::transmute(u),
                            };
                            let ver_ptr = (module.fn_version)();
                            let ver_str = if !ver_ptr.is_null() {
                                CStr::from_ptr(ver_ptr).to_string_lossy().into_owned()
                            } else {
                                "unknown".to_string()
                            };
                            log::info!(
                                "Dynamic Component Loaded: {} (v{}) from {}",
                                filename,
                                ver_str,
                                path.display()
                            );
                            let mut guard = METADATA.lock().unwrap();
                            *guard = Some(module);
                            return Ok(());
                        }
                        (v, f, r, w, u) => {
                            let mut missing = Vec::new();
                            if let Err(e) = v { missing.push(e); }
                            if let Err(e) = f { missing.push(e); }
                            if let Err(e) = r { missing.push(e); }
                            if let Err(e) = w { missing.push(e); }
                            if let Err(e) = u { missing.push(e); }
                            os_dlclose(handle);
                            specific_errors.push(format!(
                                "{}: symbol resolution failed ({})",
                                path.display(),
                                missing.join(", ")
                            ));
                        }
                    }
                }
                Err(e) => {
                    if !is_bare {
                        specific_errors.push(format!("{}: dlopen failed ({e})", path.display()));
                    } else {
                        fallback_error = Some(format!("dlopen failed: {e}"));
                    }
                }
            }
        }
    }

    Err(MissingComponentError {
        component_name: "Attune Track Metadata & Audio Tweaks Engine".to_string(),
        library_filename: filename,
        searched_paths: candidates,
        error_details: format_candidate_errors(&specific_errors, fallback_error),
    })
}

// ---------------------------------------------------------------------------
// Public Diagnostic Information
// ---------------------------------------------------------------------------

pub struct LoadedComponentSummary {
    pub name: String,
    pub filename: String,
    pub path: PathBuf,
    pub version: String,
}

unsafe fn read_c_version(fn_ver: unsafe extern "C" fn() -> *const c_char) -> String {
    let p = fn_ver();
    if !p.is_null() {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    } else {
        "null-version-ptr".to_string()
    }
}

pub fn get_loaded_components_summary() -> Vec<LoadedComponentSummary> {
    let mut summaries = Vec::new();

    if let Ok(guard) = PROCMGR.lock() {
        if let Some(m) = guard.as_ref() {
            summaries.push(LoadedComponentSummary {
                name: "Attune Process & Hardware Resource Manager".to_string(),
                filename: component_filename("attune_procmgr"),
                path: m.resolved_path.clone(),
                version: unsafe { read_c_version(m.fn_version) },
            });
        }
    }

    if let Ok(guard) = WATCHGUARD.lock() {
        if let Some(m) = guard.as_ref() {
            summaries.push(LoadedComponentSummary {
                name: "Attune Stability & Device Overhead Watchguard".to_string(),
                filename: component_filename("attune_watchguard"),
                path: m.resolved_path.clone(),
                version: unsafe { read_c_version(m.fn_version) },
            });
        }
    }

    if let Ok(guard) = METADATA.lock() {
        if let Some(m) = guard.as_ref() {
            summaries.push(LoadedComponentSummary {
                name: "Attune Track Metadata & Audio Tweaks Engine".to_string(),
                filename: component_filename("attune_music_metadata_changer"),
                path: m.resolved_path.clone(),
                version: unsafe { read_c_version(m.fn_version) },
            });
        }
    }

    summaries
}

// ---------------------------------------------------------------------------
// Component Wrapper: Process Manager
// ---------------------------------------------------------------------------

pub mod procmgr {
    use super::*;
    use crate::process_manager::ProcessMetrics;

    pub fn sample_metrics() -> Result<ProcessMetrics, String> {
        let guard = PROCMGR
            .lock()
            .map_err(|e| format!("procmgr mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_procmgr is not loaded".to_string())?;

        unsafe {
            let ptr = (module.fn_sample_metrics_json)();
            if ptr.is_null() {
                return Err("procmgr_sample_metrics_json returned null pointer".to_string());
            }
            let c_str = CStr::from_ptr(ptr);
            let json_str = c_str.to_string_lossy().into_owned();
            (module.fn_free_string)(ptr as *mut c_char);

            serde_json::from_str::<ProcessMetrics>(&json_str)
                .map_err(|e| format!("Failed to parse metrics JSON from libattune_procmgr: {e}"))
        }
    }

    pub fn clean_process_memory() -> Result<(u64, u64), String> {
        let guard = PROCMGR
            .lock()
            .map_err(|e| format!("procmgr mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_procmgr is not loaded".to_string())?;

        let mut before = 0u64;
        let mut after = 0u64;
        let code = unsafe { (module.fn_clean_process_memory)(&mut before, &mut after) };
        if code == 0 {
            Ok((before, after))
        } else {
            Err(format!("procmgr_clean_process_memory failed with code: {code}"))
        }
    }
}

// ---------------------------------------------------------------------------
// Component Wrapper: Watchguard
// ---------------------------------------------------------------------------

pub mod watchguard {
    use super::*;

    /// Real-time health and stability status snapshot deserialized from `libattune_watchguard`.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

    pub fn record_hardware_resources(
        cpu_usage_pct: f32,
        memory_rss_bytes: u64,
        gpu_usage_pct: f32,
        thread_count: u32,
    ) -> Result<(), String> {
        let guard = WATCHGUARD
            .lock()
            .map_err(|e| format!("watchguard mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_watchguard is not loaded".to_string())?;
        if let Some(fn_ptr) = module.fn_record_hardware_resources {
            unsafe {
                fn_ptr(cpu_usage_pct, memory_rss_bytes, gpu_usage_pct, thread_count);
            }
            Ok(())
        } else {
            Err("watchguard_record_hardware_resources symbol not found in loaded library".to_string())
        }
    }

    pub fn set_desktop_mode(is_desktop: bool) -> Result<(), String> {
        let guard = WATCHGUARD
            .lock()
            .map_err(|e| format!("watchguard mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_watchguard is not loaded".to_string())?;
        if let Some(fn_ptr) = module.fn_set_desktop_mode {
            unsafe {
                fn_ptr(if is_desktop { 1 } else { 0 });
            }
            Ok(())
        } else {
            Err("watchguard_set_desktop_mode symbol not found in loaded library".to_string())
        }
    }

    pub fn record_audio_event(peak_amplitude: f32, is_underrun: bool, is_clipping: bool) -> Result<(), String> {
        let guard = WATCHGUARD
            .lock()
            .map_err(|e| format!("watchguard mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_watchguard is not loaded".to_string())?;
        unsafe {
            (module.fn_record_audio_event)(
                peak_amplitude,
                if is_underrun { 1 } else { 0 },
                if is_clipping { 1 } else { 0 },
            );
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn record_ui_frame(frame_time_ms: f32) -> Result<(), String> {
        let guard = WATCHGUARD
            .lock()
            .map_err(|e| format!("watchguard mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_watchguard is not loaded".to_string())?;
        unsafe {
            (module.fn_record_ui_frame)(frame_time_ms);
        }
        Ok(())
    }

    pub fn get_status() -> Result<StabilityStatus, String> {
        let guard = WATCHGUARD
            .lock()
            .map_err(|e| format!("watchguard mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_watchguard is not loaded".to_string())?;
        unsafe {
            let ptr = (module.fn_get_status_json)();
            if ptr.is_null() {
                return Err("watchguard_get_status_json returned null pointer".to_string());
            }
            let c_str = CStr::from_ptr(ptr);
            let json_str = c_str.to_string_lossy().into_owned();
            (module.fn_free_string)(ptr as *mut c_char);
            serde_json::from_str::<StabilityStatus>(&json_str)
                .map_err(|e| format!("Failed to parse StabilityStatus JSON from libattune_watchguard: {e}"))
        }
    }

    pub fn reset_metrics() -> Result<(), String> {
        let guard = WATCHGUARD
            .lock()
            .map_err(|e| format!("watchguard mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_watchguard is not loaded".to_string())?;
        let rc = unsafe { (module.fn_reset_metrics)() };
        if rc == 0 {
            Ok(())
        } else {
            Err(format!("watchguard_reset_metrics failed with code: {rc}"))
        }
    }
}

// ---------------------------------------------------------------------------
// Component Wrapper: Music Metadata Changer
// ---------------------------------------------------------------------------

pub mod music_metadata_changer {
    use super::*;

    /// Non-destructive metadata model for audio tag updates via dynamic C-ABI.
    #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
    pub struct TrackMetadata {
        pub title: Option<String>,
        pub artist: Option<String>,
        #[serde(default)]
        pub additional_artists: Vec<String>,
        pub album: Option<String>,
        pub album_artist: Option<String>,
        pub genre: Option<String>,
        pub year: Option<u32>,
        pub track_number: Option<u32>,
        pub track_total: Option<u32>,
        pub disc_number: Option<u32>,
        pub disc_total: Option<u32>,
        pub composer: Option<String>,
        pub comment: Option<String>,
        pub lyrics: Option<String>,
        pub bpm: Option<f32>,
        pub musical_key: Option<String>,
        pub record_label: Option<String>,
        pub catalog_number: Option<String>,
        pub album_type: Option<String>,
        pub release_date: Option<String>,
        pub language: Option<String>,
        pub musicbrainz_id: Option<String>,
        pub musicbrainz_track_id: Option<String>,
        pub online_query_url: Option<String>,
    }

    impl TrackMetadata {
        pub fn new() -> Self {
            Self::default()
        }
    }

    pub fn write_track_metadata(path: &Path, meta: &TrackMetadata) -> Result<(), String> {
        let guard = METADATA
            .lock()
            .map_err(|e| format!("metadata_changer mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_music_metadata_changer is not loaded".to_string())?;

        let json = serde_json::to_string(meta)
            .map_err(|e| format!("Failed to serialize metadata to JSON: {e}"))?;

        let path_str = path.to_str().ok_or_else(|| "Path contains non-UTF8 characters".to_string())?;
        let c_path = CString::new(path_str).map_err(|e| e.to_string())?;
        let c_json = CString::new(json).map_err(|e| e.to_string())?;

        let code = unsafe { (module.fn_write_metadata)(c_path.as_ptr(), c_json.as_ptr()) };
        if code == 0 {
            Ok(())
        } else {
            Err(format!("attune_track_tweaks_write_metadata failed with error code: {code}"))
        }
    }

    pub fn read_track_metadata(path: &Path) -> Result<TrackMetadata, String> {
        let guard = METADATA
            .lock()
            .map_err(|e| format!("metadata_changer mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_music_metadata_changer is not loaded".to_string())?;

        let path_str = path.to_str().ok_or_else(|| "Path contains non-UTF8 characters".to_string())?;
        let c_path = CString::new(path_str).map_err(|e| e.to_string())?;

        unsafe {
            let ptr = (module.fn_read_metadata)(c_path.as_ptr());
            if ptr.is_null() {
                return Err("attune_track_tweaks_read_metadata returned null pointer".to_string());
            }
            let c_str = CStr::from_ptr(ptr);
            let json_str = c_str.to_string_lossy().into_owned();
            (module.fn_free_string)(ptr);

            serde_json::from_str::<TrackMetadata>(&json_str)
                .map_err(|e| format!("Failed to parse metadata JSON: {e}"))
        }
    }

    pub fn update_track_tag(path: &Path, tag_name: &str, tag_value: &str) -> Result<(), String> {
        let guard = METADATA
            .lock()
            .map_err(|e| format!("metadata_changer mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_music_metadata_changer is not loaded".to_string())?;

        let path_str = path.to_str().ok_or_else(|| "Path contains non-UTF8 characters".to_string())?;
        let c_path = CString::new(path_str).map_err(|e| e.to_string())?;
        let c_tag = CString::new(tag_name).map_err(|e| e.to_string())?;
        let c_val = CString::new(tag_value).map_err(|e| e.to_string())?;

        let code = unsafe { (module.fn_update_tag)(c_path.as_ptr(), c_tag.as_ptr(), c_val.as_ptr()) };
        if code == 0 {
            Ok(())
        } else {
            Err(format!("attune_track_tweaks_update_tag failed with error code: {code}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_filename_matches_os() {
        let name = component_filename("attune_procmgr");
        if cfg!(target_os = "windows") {
            assert_eq!(name, "attune_procmgr.dll");
        } else if cfg!(target_os = "macos") {
            assert_eq!(name, "libattune_procmgr.dylib");
        } else {
            assert_eq!(name, "libattune_procmgr.so");
        }
    }

    #[test]
    fn test_candidate_paths_resolution() {
        let candidates = resolve_candidate_paths("attune_procmgr");
        assert!(!candidates.is_empty());
        let expected_fn = component_filename("attune_procmgr");
        assert!(candidates.iter().any(|c| c.ends_with(&expected_fn)));
    }

    #[test]
    fn test_missing_component_resolution_fails_transparently() {
        let fake = "attune_nonexistent_component";
        let _filename = component_filename(fake);
        let candidates = resolve_candidate_paths(fake);
        assert!(!candidates.iter().any(|c| c.exists()));
    }

    #[test]
    fn test_preflight_loads_all_workspace_release_components() {
        let res = preflight_check_and_load();
        assert!(res.is_ok(), "Failed to load components: {:?}", res.err());
        let summary = get_loaded_components_summary();
        assert_eq!(summary.len(), 3);
        assert!(summary.iter().any(|s| s.filename == component_filename("attune_procmgr")));
        assert!(summary.iter().any(|s| s.filename == component_filename("attune_watchguard")));
        assert!(summary.iter().any(|s| s.filename == component_filename("attune_music_metadata_changer")));

        // Verify live calls through all 3 dynamic modules
        let metrics = procmgr::sample_metrics().expect("procmgr::sample_metrics must succeed");
        assert!(metrics.timestamp_ms > 0);

        watchguard::record_ui_frame(16.0).expect("watchguard::record_ui_frame must succeed");
        watchguard::record_audio_event(0.5, false, false).expect("watchguard::record_audio_event must succeed");
        watchguard::record_hardware_resources(10.0, 100_000_000, 20.0, 8).expect("record_hardware_resources must succeed");
        let status = watchguard::get_status().expect("watchguard::get_status must succeed");
        assert!(!status.is_overloaded);
        assert!(status.is_desktop_mode);
        assert_eq!(status.cpu_usage_pct, 10.0);
        watchguard::reset_metrics().expect("watchguard::reset_metrics must succeed");
    }
}
