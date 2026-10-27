//! C-ABI dynamic library export for Attune Engine.
//! Exposes audio profiling, transition recipe generation, and cache repository
//! management to Dart FFI, Flutter, and native mobile/desktop platforms.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

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
use attune_core::engine::{AudioOutputDriver, DeckId, DualDeckMixer, probe_external_dsp};
use attune_core::schema::TransitionRecipe;
use attune_dsp_ffi::master_fx::{MasterFxParams, EQ_BANDS_COUNT};

struct PlaybackSession {
    mixer: Arc<Mutex<DualDeckMixer>>,
    #[allow(dead_code)]
    driver: Option<AudioOutputDriver>,
}

static PLAYBACK_SESSION: Mutex<Option<PlaybackSession>> = Mutex::new(None);
static MASTER_FX_PARAMS: Mutex<Option<MasterFxParams>> = Mutex::new(None);

fn get_master_fx_params() -> MasterFxParams {
    if let Ok(guard) = MASTER_FX_PARAMS.lock() {
        guard.clone().unwrap_or_default()
    } else {
        MasterFxParams::default()
    }
}

fn update_master_fx_params<F>(f: F)
where
    F: FnOnce(&mut MasterFxParams),
{
    let mut params = get_master_fx_params();
    f(&mut params);

    if let Ok(mut guard) = MASTER_FX_PARAMS.lock() {
        *guard = Some(params.clone());
    }

    if let Some(mixer) = get_session_mixer() {
        if let Ok(mut m) = mixer.lock() {
            m.master_fx.set_params(params);
        }
    }
}

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
    let mut mixer_obj = DualDeckMixer::new(sample_rate);
    mixer_obj.master_fx.set_params(get_master_fx_params());
    let mixer = Arc::new(Mutex::new(mixer_obj));

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

fn get_session_mixer() -> Option<Arc<Mutex<DualDeckMixer>>> {
    if let Ok(guard) = PLAYBACK_SESSION.lock() {
        guard.as_ref().map(|s| Arc::clone(&s.mixer))
    } else {
        None
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

    let path = path_str.to_string();

    // Synchronously reset target deck under lock so polling state immediately sees
    // position 0 and stopped state while the background worker decodes the audio.
    if let Some(mixer) = get_session_mixer() {
        if let Ok(mut m) = mixer.lock() {
            if deck_id == 1 {
                m.deck_b.clear();
            } else {
                m.transition = None;
                m.deck_a.clear();
                m.active_deck = DeckId::DeckA;
            }
        }
    }

    // Multi-core worker offloading: Decodes audio without blocking caller UI thread
    std::thread::spawn(move || {
        if let Ok(audio) = AudioDecoder::load_file(&path) {
            if let Some(mixer) = get_session_mixer() {
                if let Ok(mut m) = mixer.lock() {
                    if deck_id == 1 {
                        m.load_track_b(audio);
                    } else {
                        m.load_track_a(audio);
                    }
                }
            }
        } else {
            log::warn!("AudioDecoder failed to load file for deck {}: {}", deck_id, path);
        }
    });

    0
}

/// Starts audio playback on the active deck.
#[no_mangle]
pub extern "C" fn attune_player_play() -> i32 {
    if let Some(mixer) = get_session_mixer() {
        if let Ok(mut m) = mixer.lock() {
            m.play();
            return 0;
        }
    }
    -1
}

/// Pauses audio playback.
#[no_mangle]
pub extern "C" fn attune_player_pause() -> i32 {
    if let Some(mixer) = get_session_mixer() {
        if let Ok(mut m) = mixer.lock() {
            m.pause();
            return 0;
        }
    }
    -1
}

/// Seeks a deck: deck_id 0 = Deck A, 1 = Deck B, -1 = active deck.
#[no_mangle]
pub extern "C" fn attune_player_seek(deck_id: i32, position_ms: u64) -> i32 {
    if let Some(mixer) = get_session_mixer() {
        if let Ok(mut m) = mixer.lock() {
            match deck_id {
                0 => m.deck_a.seek_ms(position_ms),
                1 => m.deck_b.seek_ms(position_ms),
                _ => match m.active_deck {
                    DeckId::DeckA => m.deck_a.seek_ms(position_ms),
                    DeckId::DeckB => m.deck_b.seek_ms(position_ms),
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

    if let Some(mixer) = get_session_mixer() {
        if let Ok(mut m) = mixer.lock() {
            m.trigger_transition(recipe);
            return 0;
        }
    }
    -4
}

/// Queries real-time playback, deck, and transition state as a JSON string.
/// The caller must free the returned string using `attune_free_string`.
#[no_mangle]
pub extern "C" fn attune_player_get_state() -> *mut c_char {
    let mixer_arc = match get_session_mixer() {
        Some(m) => m,
        None => {
            let empty = serde_json::json!({
                "initialized": false,
                "is_playing": false,
            });
            return CString::new(empty.to_string()).unwrap().into_raw();
        }
    };

    let mixer = match mixer_arc.lock() {
        Ok(m) => m,
        Err(_) => return std::ptr::null_mut(),
    };

    let deck_a_json = serde_json::json!({
        "position_ms": mixer.deck_a.current_position_ms(),
        "duration_ms": mixer.deck_a.duration_ms(),
        "audible_end_ms": mixer.deck_a.audible_end_ms(),
        "state": format!("{:?}", mixer.deck_a.state),
        "is_playing": mixer.deck_a.is_playing(),
        "is_audible_finished": mixer.deck_a.is_audible_finished(),
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
        "audible_end_ms": mixer.deck_b.audible_end_ms(),
        "state": format!("{:?}", mixer.deck_b.state),
        "is_playing": mixer.deck_b.is_playing(),
        "is_audible_finished": mixer.deck_b.is_audible_finished(),
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

/// Enables or disables the Master FX Rack (0 = disabled, 1 = enabled).
#[no_mangle]
pub extern "C" fn attune_master_fx_set_enabled(enabled: i32) -> i32 {
    update_master_fx_params(|p| p.enabled = enabled != 0);
    0
}

/// Sets the master pre-amp gain in decibels (-12 dB to +6 dB).
#[no_mangle]
pub extern "C" fn attune_master_fx_set_preamp_db(preamp_db: f32) -> i32 {
    update_master_fx_params(|p| p.preamp_db = preamp_db.clamp(-12.0, 6.0));
    0
}

/// Sets the gain in dB for a specific EQ band (0 to 9, -12 dB to +12 dB).
#[no_mangle]
pub extern "C" fn attune_master_fx_set_band_gain(band_idx: i32, gain_db: f32) -> i32 {
    if band_idx >= 0 && (band_idx as usize) < EQ_BANDS_COUNT {
        update_master_fx_params(|p| p.band_gains_db[band_idx as usize] = gain_db.clamp(-12.0, 12.0));
        0
    } else {
        -1
    }
}

/// Sets all 10 EQ band gains from an array of floats.
#[no_mangle]
pub extern "C" fn attune_master_fx_set_all_bands(bands_ptr: *const f32, count: i32) -> i32 {
    if bands_ptr.is_null() || count <= 0 {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts(bands_ptr, (count as usize).min(EQ_BANDS_COUNT)) };
    update_master_fx_params(|p| {
        for (i, &db) in slice.iter().enumerate() {
            p.band_gains_db[i] = db.clamp(-12.0, 12.0);
        }
    });
    0
}

/// Configures ViPER Bass psychoacoustic sub-harmonic synthesizer.
#[no_mangle]
pub extern "C" fn attune_master_fx_set_bass(enabled: i32, gain_db: f32, freq_hz: f32) -> i32 {
    update_master_fx_params(|p| {
        p.bass_enabled = enabled != 0;
        p.bass_gain_db = gain_db.clamp(0.0, 14.0);
        p.bass_freq_hz = freq_hz.clamp(40.0, 150.0);
    });
    0
}

/// Configures ViPER Clarity & Vocal Air harmonic exciter.
#[no_mangle]
pub extern "C" fn attune_master_fx_set_clarity(enabled: i32, wet: f32) -> i32 {
    update_master_fx_params(|p| {
        p.clarity_enabled = enabled != 0;
        p.clarity_wet = wet.clamp(0.0, 0.40);
    });
    0
}

/// Sets stereo field expansion width (0.0 = mono, 1.0 = normal, 2.0 = ultra-wide).
#[no_mangle]
pub extern "C" fn attune_master_fx_set_stereo_width(width: f32) -> i32 {
    update_master_fx_params(|p| {
        p.stereo_width = width.clamp(0.0, 2.5);
    });
    0
}

/// Sets the entire Master FX Rack state from a JSON string.
#[no_mangle]
pub extern "C" fn attune_master_fx_set_params_json(json_ptr: *const c_char) -> i32 {
    if json_ptr.is_null() {
        return -1;
    }
    let c_str = unsafe { CStr::from_ptr(json_ptr) };
    let json_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let params: MasterFxParams = match serde_json::from_str(json_str) {
        Ok(p) => p,
        Err(_) => return -1,
    };
    update_master_fx_params(|p| *p = params);
    0
}

/// Returns the current Master FX Rack state serialized as JSON.
/// The caller must free the returned string using `attune_free_string`.
#[no_mangle]
pub extern "C" fn attune_master_fx_get_params_json() -> *mut c_char {
    let params = get_master_fx_params();
    match serde_json::to_string(&params) {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Probes the host OS for active or installed external audio DSPs (ViPER, EasyEffects, JamesDSP, Equalizer APO).
/// Returns an ExternalDspStatus object serialized as JSON.
/// The caller must free the returned string using `attune_free_string`.
#[no_mangle]
pub extern "C" fn attune_probe_external_dsp() -> *mut c_char {
    let status = probe_external_dsp();
    match serde_json::to_string(&status) {
        Ok(json) => CString::new(json).unwrap().into_raw(),
        Err(_) => std::ptr::null_mut(),
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

    #[test]
    fn test_master_fx_ffi_lifecycle() {
        attune_init();
        assert_eq!(attune_master_fx_set_enabled(1), 0);
        assert_eq!(attune_master_fx_set_preamp_db(-2.5), 0);
        assert_eq!(attune_master_fx_set_band_gain(0, 3.5), 0);
        assert_eq!(attune_master_fx_set_band_gain(9, -4.0), 0);
        assert_eq!(attune_master_fx_set_bass(1, 5.0, 90.0), 0);
        assert_eq!(attune_master_fx_set_clarity(1, 0.15), 0);
        assert_eq!(attune_master_fx_set_stereo_width(1.4), 0);

        let json_ptr = attune_master_fx_get_params_json();
        assert!(!json_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(json_ptr) }.to_str().unwrap();
        let parsed: MasterFxParams = serde_json::from_str(json_str).unwrap();
        assert!(parsed.enabled);
        assert!((parsed.preamp_db - (-2.5)).abs() < 1e-4);
        assert!((parsed.band_gains_db[0] - 3.5).abs() < 1e-4);
        assert!((parsed.band_gains_db[9] - (-4.0)).abs() < 1e-4);
        assert!(parsed.bass_enabled);
        assert!((parsed.bass_gain_db - 5.0).abs() < 1e-4);
        assert!((parsed.bass_freq_hz - 90.0).abs() < 1e-4);
        assert!(parsed.clarity_enabled);
        assert!((parsed.clarity_wet - 0.15).abs() < 1e-4);
        assert!((parsed.stereo_width - 1.4).abs() < 1e-4);
        attune_free_string(json_ptr);
    }

    #[test]
    fn test_probe_external_dsp_ffi() {
        attune_init();
        let dsp_ptr = attune_probe_external_dsp();
        assert!(!dsp_ptr.is_null());
        let dsp_str = unsafe { CStr::from_ptr(dsp_ptr) }.to_str().unwrap();
        let val: serde_json::Value = serde_json::from_str(dsp_str).unwrap();
        assert!(val.get("detected").is_some());
        assert!(val.get("engine_name").is_some());
        attune_free_string(dsp_ptr);
    }
}

