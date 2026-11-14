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

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde::Deserialize;
use sha2::{Digest, Sha256};

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

// 3. attune_music_metadata_changer / attune_music_metadata
type FnMetadataVersion = unsafe extern "C" fn() -> *const c_char;
type FnMetadataFreeString = unsafe extern "C" fn(*mut c_char);
type FnMetadataReadMetadata = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type FnMetadataWriteMetadata = unsafe extern "C" fn(*const c_char, *const c_char) -> i32;
type FnMetadataUpdateTag = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> i32;
type FnMetadataFetchArtistOnline = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type FnMetadataSearchYoutubeThumbnails = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type FnMetadataDownloadYoutubeThumbnail = unsafe extern "C" fn(*const c_char, *const c_char) -> i32;
type FnMetadataFetchHqThumbnail = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

struct MetadataModule {
    _handle: *mut std::ffi::c_void,
    resolved_path: PathBuf,
    fn_version: FnMetadataVersion,
    fn_free_string: FnMetadataFreeString,
    fn_read_metadata: FnMetadataReadMetadata,
    fn_write_metadata: FnMetadataWriteMetadata,
    fn_update_tag: FnMetadataUpdateTag,
    fn_fetch_artist_online: Option<FnMetadataFetchArtistOnline>,
    fn_search_youtube_thumbnails: Option<FnMetadataSearchYoutubeThumbnails>,
    fn_download_youtube_thumbnail: Option<FnMetadataDownloadYoutubeThumbnail>,
    #[allow(dead_code)]
    fn_fetch_hq_thumbnail: Option<FnMetadataFetchHqThumbnail>,
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
// Dynamic Library Anti-Tamper Integrity Verification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentIntegrityRecord {
    pub sha256: String,
    #[serde(default)]
    pub blake3: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct IntegrityManifest {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub dynamic_version: Option<String>,
    #[serde(default)]
    pub build_number: Option<String>,
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub components: HashMap<String, ComponentIntegrityRecord>,
}

/// Verifies cryptographic hashes (SHA-256 and BLAKE3) of a candidate shared library
/// against `attune_integrity.json` if available, preventing code tampering or corrupted loads.
pub fn verify_component_integrity(path: &Path, _base_name: &str) -> Result<(), String> {
    if !path.exists() || !path.is_file() {
        return Ok(());
    }

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    if file_name.is_empty() {
        return Ok(());
    }

    // Search for attune_integrity.json in component parent hierarchy and executable directory
    let mut manifest_candidates = Vec::new();
    if let Some(parent) = path.parent() {
        manifest_candidates.push(parent.join("attune_integrity.json"));
        if let Some(grandparent) = parent.parent() {
            manifest_candidates.push(grandparent.join("attune_integrity.json"));
            manifest_candidates.push(grandparent.join("lib/attune_integrity.json"));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            manifest_candidates.push(exe_dir.join("attune_integrity.json"));
            manifest_candidates.push(exe_dir.join("lib/attune_integrity.json"));
            if let Some(exe_parent) = exe_dir.parent() {
                manifest_candidates.push(exe_parent.join("attune_integrity.json"));
            }
        }
    }

    let found_manifest_path = manifest_candidates.into_iter().find(|p| p.is_file());

    let Some(manifest_file) = found_manifest_path else {
        if std::env::var("ATTUNE_ENFORCE_INTEGRITY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            return Err(format!(
                "Security policy violation: ATTUNE_ENFORCE_INTEGRITY is active but attune_integrity.json manifest is missing for '{}'",
                path.display()
            ));
        }
        return Ok(());
    };

    let manifest_bytes = std::fs::read(&manifest_file)
        .map_err(|e| format!("Failed to read integrity manifest at {}: {e}", manifest_file.display()))?;

    let manifest: IntegrityManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("Failed to parse integrity manifest at {}: {e}", manifest_file.display()))?;

    let Some(record) = manifest.components.get(&file_name) else {
        if std::env::var("ATTUNE_ENFORCE_INTEGRITY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            return Err(format!(
                "Component '{}' is not registered in integrity manifest {}",
                file_name,
                manifest_file.display()
            ));
        }
        return Ok(());
    };

    let bytes = std::fs::read(path)
        .map_err(|e| format!("Failed to read component binary for verification at {}: {e}", path.display()))?;

    if let Some(expected_size) = record.size_bytes {
        if (bytes.len() as u64) != expected_size {
            return Err(format!(
                "Component size mismatch for '{}': expected {} bytes, got {} bytes (possible tampering or truncation)",
                path.display(),
                expected_size,
                bytes.len()
            ));
        }
    }

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let computed_sha256 = format!("{:x}", hasher.finalize());

    if !computed_sha256.eq_ignore_ascii_case(record.sha256.trim()) {
        return Err(format!(
            "CRITICAL: SHA-256 signature mismatch on '{}'! Expected {}, computed {}. Component load rejected to prevent code tampering.",
            path.display(),
            record.sha256,
            computed_sha256
        ));
    }

    if let Some(ref expected_blake3) = record.blake3 {
        let computed_blake3 = blake3::hash(&bytes).to_hex().to_string();
        if !computed_blake3.eq_ignore_ascii_case(expected_blake3.trim()) {
            return Err(format!(
                "CRITICAL: BLAKE3 hash mismatch on '{}'! Expected {}, computed {}. Component load rejected.",
                path.display(),
                expected_blake3,
                computed_blake3
            ));
        }
    }

    log::info!(
        "Security: Verified anti-tamper integrity for '{}' (SHA-256: {}...)",
        file_name,
        &computed_sha256[..12.min(computed_sha256.len())]
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Windows Dynamic Library Search Directory Configuration & Diagnostics
// ---------------------------------------------------------------------------

/// Prepares a directory path for `SetDllDirectoryW`.
/// Strips verbatim prefixes (`\\?\` and `\\?\UNC\`) which cause `SetDllDirectoryW` to fail with Error 87,
/// strips trailing path separators ('/' and '\\'), and ensures the path is non-empty.
#[allow(dead_code)]
pub fn sanitize_dll_directory_path(path: &Path) -> Option<PathBuf> {
    let path_str = path.to_string_lossy();
    let trimmed_input = path_str.trim();
    if trimmed_input.is_empty() {
        return None;
    }

    // Strip verbatim \\?\UNC\ or \\?\ prefix
    let stripped = if let Some(rest) = trimmed_input.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = trimmed_input.strip_prefix("//?/UNC/") {
        format!(r"\\{rest}")
    } else if let Some(rest) = trimmed_input.strip_prefix(r"\\?\") {
        rest.to_string()
    } else if let Some(rest) = trimmed_input.strip_prefix("//?/") {
        rest.to_string()
    } else {
        trimmed_input.to_string()
    };

    // Strip trailing slashes/backslashes
    let trimmed = stripped.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

/// Decodes common Win32 dynamic loader error codes into human-readable descriptions (Mandate 4).
#[allow(dead_code)]
pub fn format_windows_error(code: u32) -> &'static str {
    match code {
        5 => "ERROR_ACCESS_DENIED (5): Access is denied (check file permissions or antivirus)",
        87 => "ERROR_INVALID_PARAMETER (87): The parameter is incorrect (path format or \\\\?\\ prefix invalid)",
        126 => "ERROR_MOD_NOT_FOUND (126): The specified module or one of its dependencies could not be found",
        127 => "ERROR_PROC_NOT_FOUND (127): The specified procedure or export symbol could not be found",
        193 => "ERROR_BAD_EXE_FORMAT (193): Not a valid Win32 application (32-bit vs 64-bit architecture mismatch or corrupt file)",
        998 => "ERROR_NOACCESS (998): Invalid access to memory location",
        1114 => "ERROR_DLL_INIT_FAILED (1114): DLL initialization routine (DllMain) failed",
        14001 => "ERROR_SXS_CANT_GEN_ACTCTX (14001): Side-by-side configuration failed (missing Microsoft Visual C++ CRT redistributable)",
        _ => "Unknown Win32 system error",
    }
}

/// Configures Windows DLL search directory so that libraries located in `{app}\lib`
/// or `ATTUNE_LIB_DIR` are discoverable by the Windows dynamic linker.
#[cfg(windows)]
pub fn init_windows_dll_directories() {
    use std::sync::Once;
    use std::os::windows::ffi::OsStrExt;

    static INIT: Once = Once::new();
    INIT.call_once(|| {
        #[link(name = "kernel32")]
        extern "system" {
            fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
            fn GetLastError() -> u32;
        }

        let raw_target = if let Some(env_dir) = std::env::var_os("ATTUNE_LIB_DIR") {
            PathBuf::from(env_dir)
        } else if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let lib_dir = exe_dir.join("lib");
                if lib_dir.is_dir() {
                    lib_dir
                } else {
                    exe_dir.to_path_buf()
                }
            } else {
                PathBuf::from(".")
            }
        } else {
            PathBuf::from(".")
        };

        if let Some(sanitized) = sanitize_dll_directory_path(&raw_target) {
            let wide_path: Vec<u16> = sanitized
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            unsafe {
                let rc = SetDllDirectoryW(wide_path.as_ptr());
                if rc == 0 {
                    let err = GetLastError();
                    log::warn!(
                        "SetDllDirectoryW failed for '{}' with error code {err}: {}",
                        sanitized.display(),
                        format_windows_error(err)
                    );
                } else {
                    log::info!(
                        "Windows dynamic library search path configured to: {}",
                        sanitized.display()
                    );
                }
            }
        }
    });
}

#[cfg(not(windows))]
pub fn init_windows_dll_directories() {
    // No-op on non-Windows platforms (RPATH / LD_LIBRARY_PATH handles this)
}

// ---------------------------------------------------------------------------
// Dynamic Library OS Loader
// ---------------------------------------------------------------------------

#[cfg(unix)]
unsafe fn os_dlopen(path: impl AsRef<Path>) -> Result<*mut std::ffi::c_void, String> {
    use std::os::unix::ffi::OsStrExt;

    let p = path.as_ref();
    let c_path = CString::new(p.as_os_str().as_bytes())
        .map_err(|e| format!("Path contains interior null byte: {e}"))?;
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
unsafe fn os_dlopen(path: impl AsRef<Path>) -> Result<*mut std::ffi::c_void, String> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(lpLibFileName: *const u16) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
    }

    let p = path.as_ref();
    let wide_path: Vec<u16> = p
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let handle = LoadLibraryW(wide_path.as_ptr());
    if handle.is_null() {
        let code = GetLastError();
        let desc = format_windows_error(code);
        let extra = if code == 126 && p.exists() {
            " (Note: file exists on disk; a dependent DLL such as vcruntime140.dll or a workspace DLL in lib/ failed to resolve)"
        } else {
            ""
        };
        Err(format!(
            "LoadLibraryW failed on '{}' with code {code}: {desc}{extra}",
            p.display()
        ))
    } else {
        Ok(handle)
    }
}

#[cfg(windows)]
unsafe fn os_dlsym(handle: *mut std::ffi::c_void, symbol: &str) -> Result<*mut std::ffi::c_void, String> {
    #[link(name = "kernel32")]
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
    #[link(name = "kernel32")]
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
    init_windows_dll_directories();
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

        if let Err(e) = verify_component_integrity(path, base) {
            specific_errors.push(format!("{}: anti-tamper check failed ({e})", path.display()));
            log::warn!("Component integrity check rejected {}: {e}", path.display());
            continue;
        }

        unsafe {
            match os_dlopen(path) {
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

        if let Err(e) = verify_component_integrity(path, base) {
            specific_errors.push(format!("{}: anti-tamper check failed ({e})", path.display()));
            log::warn!("Component integrity check rejected {}: {e}", path.display());
            continue;
        }

        unsafe {
            match os_dlopen(path) {
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
    let base = "attune_music_metadata";
    let filename = component_filename(base);
    let mut candidates = resolve_candidate_paths("attune_music_metadata");
    candidates.extend(resolve_candidate_paths("attune_music_metadata_changer"));
    let mut specific_errors = Vec::new();
    let mut fallback_error = None;

    for path in &candidates {
        let is_bare = path.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true);
        if !is_bare && !path.exists() {
            continue;
        }

        if let Err(e) = verify_component_integrity(path, base) {
            specific_errors.push(format!("{}: anti-tamper check failed ({e})", path.display()));
            log::warn!("Component integrity check rejected {}: {e}", path.display());
            continue;
        }

        unsafe {
            match os_dlopen(path) {
                Ok(handle) => {
                    let v_sym = os_dlsym(handle, "attune_track_tweaks_version");
                    let f_sym = os_dlsym(handle, "attune_track_tweaks_free_string");
                    let r_sym = os_dlsym(handle, "attune_track_tweaks_read_metadata");
                    let w_sym = os_dlsym(handle, "attune_track_tweaks_write_metadata");
                    let u_sym = os_dlsym(handle, "attune_track_tweaks_update_tag");
                    let fetch_sym = os_dlsym(handle, "attune_metadata_fetch_artist_online")
                        .or_else(|_| os_dlsym(handle, "attune_metadata_changer_fetch_artist_online"));
                    let search_yt_sym = os_dlsym(handle, "attune_metadata_search_youtube_thumbnails")
                        .or_else(|_| os_dlsym(handle, "attune_metadata_changer_search_youtube_thumbnails"));
                    let dl_yt_sym = os_dlsym(handle, "attune_metadata_download_youtube_thumbnail")
                        .or_else(|_| os_dlsym(handle, "attune_metadata_changer_download_youtube_thumbnail"));
                    let fetch_hq_sym = os_dlsym(handle, "attune_metadata_fetch_hq_thumbnail")
                        .or_else(|_| os_dlsym(handle, "attune_metadata_changer_fetch_hq_thumbnail"));

                    match (v_sym, f_sym, r_sym, w_sym, u_sym, search_yt_sym, dl_yt_sym, fetch_hq_sym) {
                        (Ok(v), Ok(f), Ok(r), Ok(w), Ok(u), Ok(s_yt), Ok(d_yt), Ok(f_hq)) => {
                            let module = MetadataModule {
                                _handle: handle,
                                resolved_path: path.clone(),
                                fn_version: std::mem::transmute(v),
                                fn_free_string: std::mem::transmute(f),
                                fn_read_metadata: std::mem::transmute(r),
                                fn_write_metadata: std::mem::transmute(w),
                                fn_update_tag: std::mem::transmute(u),
                                fn_fetch_artist_online: fetch_sym.ok().map(|s| std::mem::transmute(s)),
                                fn_search_youtube_thumbnails: Some(std::mem::transmute(s_yt)),
                                fn_download_youtube_thumbnail: Some(std::mem::transmute(d_yt)),
                                fn_fetch_hq_thumbnail: Some(std::mem::transmute(f_hq)),
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
                        (v, f, r, w, u, s_yt, d_yt, f_hq) => {
                            let mut missing = Vec::new();
                            if let Err(e) = v { missing.push(e); }
                            if let Err(e) = f { missing.push(e); }
                            if let Err(e) = r { missing.push(e); }
                            if let Err(e) = w { missing.push(e); }
                            if let Err(e) = u { missing.push(e); }
                            if let Err(e) = s_yt { missing.push(e); }
                            if let Err(e) = d_yt { missing.push(e); }
                            if let Err(e) = f_hq { missing.push(e); }
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

#[allow(dead_code)]
pub struct LoadedComponentSummary {
    pub name: String,
    pub filename: String,
    pub path: PathBuf,
    pub version: String,
}

#[allow(dead_code)]
unsafe fn read_c_version(fn_ver: unsafe extern "C" fn() -> *const c_char) -> String {
    let p = fn_ver();
    if !p.is_null() {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    } else {
        "null-version-ptr".to_string()
    }
}

#[allow(dead_code)]
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
            let actual_fn = m
                .resolved_path
                .file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| component_filename("attune_music_metadata"));
            summaries.push(LoadedComponentSummary {
                name: "Attune Track Metadata & Audio Tweaks Engine".to_string(),
                filename: actual_fn,
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
        #[serde(default)]
        pub writer: Option<String>,
        #[serde(default)]
        pub vocalist: Option<String>,
        #[serde(default)]
        pub producer: Option<String>,
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
        #[serde(default)]
        pub picture_path: Option<String>,
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

    /// Queries online artist metadata via dynamic shared library C-ABI.
    /// Returns raw JSON string containing `{ name, avatar_url, banner_url, bio, subscribers, views, browse_id }`.
    pub fn fetch_artist_online(artist_name: &str) -> Result<String, String> {
        let guard = METADATA
            .lock()
            .map_err(|e| format!("metadata mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_music_metadata is not loaded".to_string())?;

        let fn_fetch = module
            .fn_fetch_artist_online
            .ok_or_else(|| "attune_metadata_fetch_artist_online symbol not available".to_string())?;

        let c_artist = CString::new(artist_name).map_err(|e| e.to_string())?;
        unsafe {
            let ptr = (fn_fetch)(c_artist.as_ptr());
            if ptr.is_null() {
                return Err("attune_metadata_fetch_artist_online returned null pointer".to_string());
            }
            let c_str = CStr::from_ptr(ptr);
            let json_str = c_str.to_string_lossy().into_owned();
            (module.fn_free_string)(ptr);
            Ok(json_str)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct YouTubeThumbnailCandidate {
        pub id: String,
        pub title: String,
        pub author: String,
        pub thumbnail_url: String,
        pub preview_url: String,
        pub width: u32,
        pub height: u32,
        pub source: String,
    }

    /// Searches YouTube & YouTube Music for candidate high quality artwork thumbnails.
    pub fn search_youtube_thumbnails(query: &str) -> Result<Vec<YouTubeThumbnailCandidate>, String> {
        let guard = METADATA
            .lock()
            .map_err(|e| format!("metadata mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_music_metadata is not loaded".to_string())?;

        let fn_search = module
            .fn_search_youtube_thumbnails
            .ok_or_else(|| "attune_metadata_search_youtube_thumbnails symbol not available".to_string())?;

        let c_query = CString::new(query).map_err(|e| e.to_string())?;
        unsafe {
            let ptr = (fn_search)(c_query.as_ptr());
            if ptr.is_null() {
                return Err("attune_metadata_search_youtube_thumbnails returned null pointer".to_string());
            }
            let c_str = CStr::from_ptr(ptr);
            let json_str = c_str.to_string_lossy().into_owned();
            (module.fn_free_string)(ptr);

            let cands: Vec<YouTubeThumbnailCandidate> = serde_json::from_str(&json_str)
                .map_err(|e| format!("Failed parsing YouTube thumbnail candidates JSON: {e}"))?;
            Ok(cands)
        }
    }

    /// Downloads a thumbnail from `url` and saves it to `dest_path`.
    pub fn download_youtube_thumbnail(url: &str, dest_path: &Path) -> Result<(), String> {
        let guard = METADATA
            .lock()
            .map_err(|e| format!("metadata mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_music_metadata is not loaded".to_string())?;

        let fn_download = module
            .fn_download_youtube_thumbnail
            .ok_or_else(|| "attune_metadata_download_youtube_thumbnail symbol not available".to_string())?;

        let c_url = CString::new(url).map_err(|e| e.to_string())?;
        let c_dest = CString::new(dest_path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        unsafe {
            let code = (fn_download)(c_url.as_ptr(), c_dest.as_ptr());
            if code == 0 {
                Ok(())
            } else {
                Err(format!("attune_metadata_download_youtube_thumbnail failed with error code: {code}"))
            }
        }
    }

    /// Automatically searches for the top HQ thumbnail for a song and downloads it to `dest_path`.
    #[allow(dead_code)]
    pub fn fetch_hq_thumbnail(title: &str, artist: &str, dest_path: &Path) -> Result<YouTubeThumbnailCandidate, String> {
        let guard = METADATA
            .lock()
            .map_err(|e| format!("metadata mutex poisoned: {e}"))?;
        let module = guard
            .as_ref()
            .ok_or_else(|| "libattune_music_metadata is not loaded".to_string())?;

        let fn_fetch = module
            .fn_fetch_hq_thumbnail
            .ok_or_else(|| "attune_metadata_fetch_hq_thumbnail symbol not available".to_string())?;

        let c_title = CString::new(title).map_err(|e| e.to_string())?;
        let c_artist = CString::new(artist).map_err(|e| e.to_string())?;
        let c_dest = CString::new(dest_path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        unsafe {
            let ptr = (fn_fetch)(c_title.as_ptr(), c_artist.as_ptr(), c_dest.as_ptr());
            if ptr.is_null() {
                return Err("attune_metadata_fetch_hq_thumbnail returned null pointer".to_string());
            }
            let c_str = CStr::from_ptr(ptr);
            let json_str = c_str.to_string_lossy().into_owned();
            (module.fn_free_string)(ptr);

            let cand: YouTubeThumbnailCandidate = serde_json::from_str(&json_str)
                .map_err(|e| format!("Failed parsing HQ thumbnail result JSON: {e}"))?;
            Ok(cand)
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
        assert!(summary.iter().any(|s| {
            s.filename == component_filename("attune_music_metadata")
                || s.filename == component_filename("attune_music_metadata_changer")
        }));

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

        // Verify YouTube thumbnail search symbol is available and callable through dynamic C-ABI
        let yt_res = music_metadata_changer::search_youtube_thumbnails("Nyan Cat");
        if let Err(ref e) = yt_res {
            assert!(
                !e.contains("symbol not available"),
                "attune_metadata_search_youtube_thumbnails symbol must be dynamically available: {e}"
            );
        }
    }

    #[test]
    fn test_integrity_verification_tamper_detection() {
        let unique_dir = std::env::temp_dir().join(format!(
            "attune_test_integrity_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&unique_dir).expect("create test dir");
        let fake_dll_path = unique_dir.join("attune_watchguard.dll");
        let fake_dll_content = b"ATTUNE_FAKE_DLL_PAYLOAD_VALID";
        std::fs::write(&fake_dll_path, fake_dll_content).expect("write fake dll");

        let mut hasher = Sha256::new();
        hasher.update(fake_dll_content);
        let valid_sha256 = format!("{:x}", hasher.finalize());
        let valid_blake3 = blake3::hash(fake_dll_content).to_hex().to_string();

        let manifest_content = format!(
            r#"{{
                "version": "0.1.0",
                "components": {{
                    "attune_watchguard.dll": {{
                        "sha256": "{}",
                        "blake3": "{}",
                        "size_bytes": {}
                    }}
                }}
            }}"#,
            valid_sha256,
            valid_blake3,
            fake_dll_content.len()
        );

        let manifest_path = unique_dir.join("attune_integrity.json");
        std::fs::write(&manifest_path, manifest_content).expect("write manifest");

        // 1. Untampered component must pass verification
        assert!(verify_component_integrity(&fake_dll_path, "attune_watchguard").is_ok());

        // 2. Tampered component must be detected and rejected
        let tampered_content = b"ATTUNE_FAKE_DLL_PAYLOAD_TAMPERED";
        std::fs::write(&fake_dll_path, tampered_content).expect("write tampered dll");
        let err = verify_component_integrity(&fake_dll_path, "attune_watchguard")
            .expect_err("Tampered component must be rejected");
        assert!(err.contains("CRITICAL") || err.contains("mismatch"), "Error was: {err}");

        let _ = std::fs::remove_dir_all(&unique_dir);
    }

    #[test]
    fn test_sanitize_dll_directory_path() {
        // Standard path with no prefixes or trailing slashes
        assert_eq!(
            sanitize_dll_directory_path(Path::new(r"C:\Program Files\Attune\lib")),
            Some(PathBuf::from(r"C:\Program Files\Attune\lib"))
        );

        // Path with trailing backslash
        assert_eq!(
            sanitize_dll_directory_path(Path::new(r"C:\Program Files\Attune\lib\")),
            Some(PathBuf::from(r"C:\Program Files\Attune\lib"))
        );

        // Path with multiple trailing slashes
        assert_eq!(
            sanitize_dll_directory_path(Path::new(r"C:\Program Files\Attune\lib\\")),
            Some(PathBuf::from(r"C:\Program Files\Attune\lib"))
        );

        // Path with forward slash and multiple trailing slashes
        assert_eq!(
            sanitize_dll_directory_path(Path::new("C:/Program Files/Attune/lib///")),
            Some(PathBuf::from("C:/Program Files/Attune/lib"))
        );

        // Verbatim \\?\ prefix
        assert_eq!(
            sanitize_dll_directory_path(Path::new(r"\\?\C:\Program Files\Attune\lib")),
            Some(PathBuf::from(r"C:\Program Files\Attune\lib"))
        );

        // Verbatim \\?\ prefix with trailing backslash
        assert_eq!(
            sanitize_dll_directory_path(Path::new(r"\\?\C:\Program Files\Attune\lib\")),
            Some(PathBuf::from(r"C:\Program Files\Attune\lib"))
        );

        // Verbatim UNC \\?\UNC\ prefix
        assert_eq!(
            sanitize_dll_directory_path(Path::new(r"\\?\UNC\server\share\lib")),
            Some(PathBuf::from(r"\\server\share\lib"))
        );

        // Verbatim UNC \\?\UNC\ prefix with trailing backslash
        assert_eq!(
            sanitize_dll_directory_path(Path::new(r"\\?\UNC\server\share\lib\")),
            Some(PathBuf::from(r"\\server\share\lib"))
        );

        // Forward slash verbatim prefixes
        assert_eq!(
            sanitize_dll_directory_path(Path::new("//?/C:/Program Files/Attune/lib")),
            Some(PathBuf::from("C:/Program Files/Attune/lib"))
        );
        assert_eq!(
            sanitize_dll_directory_path(Path::new("//?/UNC/server/share/lib/")),
            Some(PathBuf::from(r"\\server/share/lib"))
        );

        // Empty and whitespace paths
        assert_eq!(sanitize_dll_directory_path(Path::new("")), None);
        assert_eq!(sanitize_dll_directory_path(Path::new("   ")), None);

        // Verbatim prefixes only (no path body)
        assert_eq!(sanitize_dll_directory_path(Path::new(r"\\?\")), None);
        assert_eq!(sanitize_dll_directory_path(Path::new(r"\\?\UNC\")), None);
        assert_eq!(sanitize_dll_directory_path(Path::new("//?/")), None);
        assert_eq!(sanitize_dll_directory_path(Path::new("//?/UNC/")), None);

        // Trailing slashes only
        assert_eq!(sanitize_dll_directory_path(Path::new(r"\\\\")), None);
        assert_eq!(sanitize_dll_directory_path(Path::new("///")), None);
    }

    #[test]
    fn test_format_windows_error_codes() {
        assert!(format_windows_error(5).contains("ERROR_ACCESS_DENIED"));
        assert!(format_windows_error(87).contains("ERROR_INVALID_PARAMETER"));
        assert!(format_windows_error(126).contains("ERROR_MOD_NOT_FOUND"));
        assert!(format_windows_error(127).contains("ERROR_PROC_NOT_FOUND"));
        assert!(format_windows_error(193).contains("ERROR_BAD_EXE_FORMAT"));
        assert!(format_windows_error(998).contains("ERROR_NOACCESS"));
        assert!(format_windows_error(1114).contains("ERROR_DLL_INIT_FAILED"));
        assert!(format_windows_error(14001).contains("ERROR_SXS_CANT_GEN_ACTCTX"));

        let unknown = format_windows_error(999999);
        assert!(unknown.contains("Unknown Win32") || unknown.contains("error"));
    }

    #[test]
    fn test_init_windows_dll_directories_idempotency() {
        // Calling init_windows_dll_directories multiple times must be safe, idempotent, and non-panicking
        init_windows_dll_directories();
        init_windows_dll_directories();
        init_windows_dll_directories();
    }
}
