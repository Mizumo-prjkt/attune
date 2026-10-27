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

use std::sync::{Arc, Mutex};
use attune_core::engine::{AudioOutputDriver, DeckId, DualDeckMixer};
use attune_core::schema::TransitionRecipe;

struct PlaybackSession {
    mixer: Arc<Mutex<DualDeckMixer>>,
    #[allow(dead_code)]
    driver: Option<AudioOutputDriver>,
}

static PLAYBACK_SESSION: Mutex<Option<PlaybackSession>> = Mutex::new(None);

/// Initializes the real-time DualDeckMixer and hardware AudioOutputDriver.
/// Returns 0 for hardware audio active, 1 for headless/fallback mode, -1 on failure.
#[no_mangle]
pub extern "C" fn attune_player_start() -> i32 {
    let mut session_guard = match PLAYBACK_SESSION.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };

    if session_guard.is_some() {
        return 0;
    }

    let sample_rate = AudioOutputDriver::default_sample_rate().unwrap_or(44100);
    let mixer = Arc::new(Mutex::new(DualDeckMixer::new(sample_rate)));

    // Attempt to start hardware output stream (gracefully fallback if headless)
    let driver = AudioOutputDriver::new(Arc::clone(&mixer)).ok();
    let has_driver = driver.is_some();

    *session_guard = Some(PlaybackSession { mixer, driver });

    if has_driver {
        0
    } else {
        1
    }
}

/// Loads an audio file into Deck A (0) or Deck B (1).
#[no_mangle]
pub extern "C" fn attune_player_load_track(deck_id: i32, path_ptr: *const c_char) -> i32 {
    if path_ptr.is_null() {
        return -1;
    }
    let c_str = unsafe { CStr::from_ptr(path_ptr) };
    let path_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let audio = match AudioDecoder::load_file(path_str) {
        Ok(a) => a,
        Err(_) => return -2,
    };

    let session_guard = match PLAYBACK_SESSION.lock() {
        Ok(g) => g,
        Err(_) => return -3,
    };

    let session = match session_guard.as_ref() {
        Some(s) => s,
        None => return -4,
    };

    let mut mixer = match session.mixer.lock() {
        Ok(m) => m,
        Err(_) => return -5,
    };

    if deck_id == 1 {
        mixer.load_track_b(audio);
    } else {
        mixer.load_track_a(audio);
    }

    0
}

/// Starts audio playback on the active deck.
#[no_mangle]
pub extern "C" fn attune_player_play() -> i32 {
    let session_guard = match PLAYBACK_SESSION.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };
    if let Some(session) = session_guard.as_ref() {
        if let Ok(mut mixer) = session.mixer.lock() {
            mixer.play();
            return 0;
        }
    }
    -1
}

/// Pauses audio playback.
#[no_mangle]
pub extern "C" fn attune_player_pause() -> i32 {
    let session_guard = match PLAYBACK_SESSION.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };
    if let Some(session) = session_guard.as_ref() {
        if let Ok(mut mixer) = session.mixer.lock() {
            mixer.pause();
            return 0;
        }
    }
    -1
}

/// Seeks a deck: deck_id 0 = Deck A, 1 = Deck B, -1 = active deck.
#[no_mangle]
pub extern "C" fn attune_player_seek(deck_id: i32, position_ms: u64) -> i32 {
    let session_guard = match PLAYBACK_SESSION.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };
    if let Some(session) = session_guard.as_ref() {
        if let Ok(mut mixer) = session.mixer.lock() {
            match deck_id {
                0 => mixer.deck_a.seek_ms(position_ms),
                1 => mixer.deck_b.seek_ms(position_ms),
                _ => match mixer.active_deck {
                    DeckId::DeckA => mixer.deck_a.seek_ms(position_ms),
                    DeckId::DeckB => mixer.deck_b.seek_ms(position_ms),
                },
            }
            return 0;
        }
    }
    -1
}

/// Triggers a transition between Deck A and Deck B given a TransitionRecipe JSON string.
#[no_mangle]
pub extern "C" fn attune_player_trigger_transition(recipe_json_ptr: *const c_char) -> i32 {
    if recipe_json_ptr.is_null() {
        return -1;
    }
    let c_str = unsafe { CStr::from_ptr(recipe_json_ptr) };
    let json_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let recipe: TransitionRecipe = match serde_json::from_str(json_str) {
        Ok(r) => r,
        Err(_) => return -2,
    };

    let session_guard = match PLAYBACK_SESSION.lock() {
        Ok(g) => g,
        Err(_) => return -3,
    };
    if let Some(session) = session_guard.as_ref() {
        if let Ok(mut mixer) = session.mixer.lock() {
            mixer.trigger_transition(recipe);
            return 0;
        }
    }
    -4
}

/// Queries real-time playback, deck, and transition state as a JSON string.
/// The caller must free the returned string using `attune_free_string`.
#[no_mangle]
pub extern "C" fn attune_player_get_state() -> *mut c_char {
    let session_guard = match PLAYBACK_SESSION.lock() {
        Ok(g) => g,
        Err(_) => return std::ptr::null_mut(),
    };

    let session = match session_guard.as_ref() {
        Some(s) => s,
        None => {
            let empty = serde_json::json!({
                "initialized": false,
                "is_playing": false,
            });
            return CString::new(empty.to_string()).unwrap().into_raw();
        }
    };

    let mixer = match session.mixer.lock() {
        Ok(m) => m,
        Err(_) => return std::ptr::null_mut(),
    };

    let deck_a_json = serde_json::json!({
        "position_ms": mixer.deck_a.current_position_ms(),
        "duration_ms": mixer.deck_a.duration_ms(),
        "state": format!("{:?}", mixer.deck_a.state),
        "is_playing": mixer.deck_a.is_playing(),
        "gain_db": mixer.deck_a.gain_db(),
        "stretch_ratio": mixer.deck_a.stretch_ratio(),
        "pitch_cents": mixer.deck_a.pitch_cents(),
        "reverb_freeze_send": mixer.deck_a.reverb_freeze_send(),
        "eq_low_db": mixer.deck_a.eq_low_db(),
        "eq_mid_db": mixer.deck_a.eq_mid_db(),
        "eq_high_db": mixer.deck_a.eq_high_db(),
    });

    let deck_b_json = serde_json::json!({
        "position_ms": mixer.deck_b.current_position_ms(),
        "duration_ms": mixer.deck_b.duration_ms(),
        "state": format!("{:?}", mixer.deck_b.state),
        "is_playing": mixer.deck_b.is_playing(),
        "gain_db": mixer.deck_b.gain_db(),
        "stretch_ratio": mixer.deck_b.stretch_ratio(),
        "pitch_cents": mixer.deck_b.pitch_cents(),
        "reverb_freeze_send": mixer.deck_b.reverb_freeze_send(),
        "eq_low_db": mixer.deck_b.eq_low_db(),
        "eq_mid_db": mixer.deck_b.eq_mid_db(),
        "eq_high_db": mixer.deck_b.eq_high_db(),
    });

    let transition_json = mixer.transition.as_ref().map(|t| {
        let progress = if t.total_transition_frames > 0 {
            t.elapsed_frames as f32 / t.total_transition_frames as f32
        } else {
            1.0
        };
        serde_json::json!({
            "in_progress": true,
            "elapsed_frames": t.elapsed_frames,
            "total_transition_frames": t.total_transition_frames,
            "progress": progress.clamp(0.0, 1.0),
            "archetype": format!("{:?}", t.recipe.archetype),
        })
    });

    let active_deck_str = match mixer.active_deck {
        DeckId::DeckA => "DeckA",
        DeckId::DeckB => "DeckB",
    };

    let result = serde_json::json!({
        "initialized": true,
        "is_playing": mixer.is_playing(),
        "active_deck": active_deck_str,
        "sample_rate": mixer.sample_rate,
        "deck_a": deck_a_json,
        "deck_b": deck_b_json,
        "transition": transition_json,
    });

    match serde_json::to_string(&result) {
        Ok(s) => CString::new(s).unwrap().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Stops the real-time player and releases audio hardware.
#[no_mangle]
pub extern "C" fn attune_player_stop() -> i32 {
    let mut session_guard = match PLAYBACK_SESSION.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };
    *session_guard = None;
    0
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

    #[test]
    fn test_player_lifecycle() {
        attune_init();
        let code = attune_player_start();
        assert!(code >= 0);

        let state_ptr = attune_player_get_state();
        assert!(!state_ptr.is_null());
        let state_str = unsafe { CStr::from_ptr(state_ptr) }.to_str().unwrap();
        let state_val: serde_json::Value = serde_json::from_str(state_str).unwrap();
        assert_eq!(state_val["initialized"], true);
        assert_eq!(state_val["is_playing"], false);
        attune_free_string(state_ptr);

        assert_eq!(attune_player_play(), 0);
        assert_eq!(attune_player_pause(), 0);
        assert_eq!(attune_player_seek(-1, 5000), 0);
        assert_eq!(attune_player_stop(), 0);
    }
}
