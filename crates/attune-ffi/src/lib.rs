//! C-ABI dynamic library export for Attune Engine.
//! Exposes audio profiling, transition recipe generation, and cache repository
//! management to Dart FFI, Flutter, and native mobile/desktop platforms.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Once;

use attune_core::audio::AudioDecoder;
use attune_core::cache::CacheDatabase;
use attune_core::planner::TransitionPlanner;
use attune_core::profiler::AudioProfiler;
use attune_core::schema::{AnalysisTier, TrackAnalysisProfile};

static INIT_LOGGER: Once = Once::new();

/// Initializes runtime logging for Attune Engine.
#[no_mangle]
pub extern "C" fn attune_init() {
    INIT_LOGGER.call_once(|| {
        let _ = env_logger::builder().is_test(false).try_init();
    });
}

/// Returns the Attune Engine version string.
/// The caller must free the returned string using `attune_free_string`.
#[no_mangle]
pub extern "C" fn attune_version() -> *mut c_char {
    let s = CString::new("0.1.0").unwrap();
    s.into_raw()
}

/// Safely deallocates a C string allocated by Attune Engine.
#[no_mangle]
pub extern "C" fn attune_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Analyzes an audio file and returns its TrackAnalysisProfile serialized as JSON.
/// `tier_code`: 0 = JitOnDemand, 1 = FullLocal, 2 = HeavyComputeNode.
/// The caller must free the returned string using `attune_free_string`.
#[no_mangle]
pub extern "C" fn attune_profile_file(path_ptr: *const c_char, tier_code: i32) -> *mut c_char {
    if path_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let c_str = unsafe { CStr::from_ptr(path_ptr) };
    let path_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let tier = match tier_code {
        0 => AnalysisTier::FastHeadTail,
        2 => AnalysisTier::ExternalDeepMl,
        _ => AnalysisTier::FullLocal,
    };

    let buffer = match AudioDecoder::load_file(path_str) {
        Ok(b) => b,
        Err(e) => {
            let err_json = serde_json::json!({ "error": format!("Audio load failed: {}", e) });
            return CString::new(err_json.to_string()).unwrap().into_raw();
        }
    };

    let profiler = AudioProfiler::new();
    let tap = profiler.profile_track(&buffer, path_str, tier);

    match serde_json::to_string(&tap) {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(e) => {
            let err_json = serde_json::json!({ "error": format!("Serialization failed: {}", e) });
            CString::new(err_json.to_string()).unwrap().into_raw()
        }
    }
}

/// Plans a seamless transition between two tracks given their TrackAnalysisProfile JSONs.
/// Returns the TransitionRecipe serialized as JSON.
/// The caller must free the returned string using `attune_free_string`.
#[no_mangle]
pub extern "C" fn attune_plan_transition(
    tap_a_json: *const c_char,
    tap_b_json: *const c_char,
) -> *mut c_char {
    if tap_a_json.is_null() || tap_b_json.is_null() {
        return std::ptr::null_mut();
    }
    let c_str_a = unsafe { CStr::from_ptr(tap_a_json) };
    let c_str_b = unsafe { CStr::from_ptr(tap_b_json) };

    let str_a = match c_str_a.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let str_b = match c_str_b.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let tap_a: TrackAnalysisProfile = match serde_json::from_str(str_a) {
        Ok(t) => t,
        Err(e) => {
            let err_json = serde_json::json!({ "error": format!("TAP A parse error: {}", e) });
            return CString::new(err_json.to_string()).unwrap().into_raw();
        }
    };

    let tap_b: TrackAnalysisProfile = match serde_json::from_str(str_b) {
        Ok(t) => t,
        Err(e) => {
            let err_json = serde_json::json!({ "error": format!("TAP B parse error: {}", e) });
            return CString::new(err_json.to_string()).unwrap().into_raw();
        }
    };

    let planner = TransitionPlanner::new();
    let recipe = planner.plan_transition(&tap_a, &tap_b);

    match serde_json::to_string(&recipe) {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(e) => {
            let err_json =
                serde_json::json!({ "error": format!("Recipe serialization error: {}", e) });
            CString::new(err_json.to_string()).unwrap().into_raw()
        }
    }
}

/// Retrieves statistics from an Attune SQLite cache database.
/// Returns CacheStats serialized as JSON.
/// The caller must free the returned string using `attune_free_string`.
#[no_mangle]
pub extern "C" fn attune_cache_stats(db_path_ptr: *const c_char) -> *mut c_char {
    if db_path_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let c_str = unsafe { CStr::from_ptr(db_path_ptr) };
    let db_path = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let db = match CacheDatabase::open(db_path) {
        Ok(d) => d,
        Err(e) => {
            let err_json = serde_json::json!({ "error": format!("DB open error: {}", e) });
            return CString::new(err_json.to_string()).unwrap().into_raw();
        }
    };

    let stats = match db.get_stats() {
        Ok(s) => s,
        Err(e) => {
            let err_json = serde_json::json!({ "error": format!("DB stats error: {}", e) });
            return CString::new(err_json.to_string()).unwrap().into_raw();
        }
    };

    match serde_json::to_string(&stats) {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(e) => {
            let err_json =
                serde_json::json!({ "error": format!("Stats serialization error: {}", e) });
            CString::new(err_json.to_string()).unwrap().into_raw()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_and_string_lifecycle() {
        attune_init();
        let ver_ptr = attune_version();
        assert!(!ver_ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(ver_ptr) };
        assert_eq!(c_str.to_str().unwrap(), "0.1.0");
        attune_free_string(ver_ptr);
    }
}
