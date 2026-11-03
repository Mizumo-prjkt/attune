use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::catch_unwind;

use crate::audio::{decode_audio_file, write_wav_file, AudioData};
use crate::dsp::{apply_speed_factor, normalize_gain, slice_region, trim_silence};
use crate::metadata::{read_track_metadata, write_track_metadata, TrackMetadata};

pub const ERR_SUCCESS: i32 = 0;
pub const ERR_NULL_PTR: i32 = -1;
pub const ERR_INVALID_UTF8: i32 = -2;
pub const ERR_DECODE_FAILED: i32 = -3;
pub const ERR_INVALID_PARAM: i32 = -4;
pub const ERR_WRITE_FAILED: i32 = -5;
pub const ERR_PANIC: i32 = -99;

/// Returns the attune-music-metadata-changer crate version string.
/// Caller does not need to free this static string.
#[no_mangle]
pub extern "C" fn attune_track_tweaks_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

/// Safely frees a heap-allocated C string returned by this crate.
#[no_mangle]
pub extern "C" fn attune_track_tweaks_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = catch_unwind(|| unsafe {
            let _ = CString::from_raw(s);
        });
    }
}

/// Normalizes audio peak gain to `target_peak_db` dBFS and writes output to `output_path` (WAV).
#[no_mangle]
pub extern "C" fn attune_track_tweaks_normalize(
    input_path: *const c_char,
    output_path: *const c_char,
    target_peak_db: f32,
) -> i32 {
    let res = catch_unwind(|| {
        let in_str = match parse_cstr(input_path) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let out_str = match parse_cstr(output_path) {
            Ok(s) => s,
            Err(e) => return e,
        };

        if !target_peak_db.is_finite() {
            return ERR_INVALID_PARAM;
        }

        let mut audio = match decode_audio_file(in_str) {
            Ok(a) => a,
            Err(e) => {
                log::error!("Audio decode failed: {}", e);
                return ERR_DECODE_FAILED;
            }
        };

        if let Err(e) = normalize_gain(&mut audio.samples, target_peak_db) {
            log::error!("Normalize gain failed: {}", e);
            return ERR_INVALID_PARAM;
        }

        if let Err(e) = write_wav_file(out_str, &audio) {
            log::error!("WAV write failed: {}", e);
            return ERR_WRITE_FAILED;
        }

        ERR_SUCCESS
    });

    res.unwrap_or(ERR_PANIC)
}

/// Trims leading and trailing silence below `threshold_db` dBFS and writes output to `output_path` (WAV).
#[no_mangle]
pub extern "C" fn attune_track_tweaks_trim_silence(
    input_path: *const c_char,
    output_path: *const c_char,
    threshold_db: f32,
) -> i32 {
    let res = catch_unwind(|| {
        let in_str = match parse_cstr(input_path) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let out_str = match parse_cstr(output_path) {
            Ok(s) => s,
            Err(e) => return e,
        };

        if !threshold_db.is_finite() {
            return ERR_INVALID_PARAM;
        }

        let audio = match decode_audio_file(in_str) {
            Ok(a) => a,
            Err(e) => {
                log::error!("Audio decode failed: {}", e);
                return ERR_DECODE_FAILED;
            }
        };

        let trimmed_samples =
            trim_silence(&audio.samples, audio.channels, audio.sample_rate, threshold_db);
        let out_audio = AudioData::new(audio.channels, audio.sample_rate, trimmed_samples);

        if let Err(e) = write_wav_file(out_str, &out_audio) {
            log::error!("WAV write failed: {}", e);
            return ERR_WRITE_FAILED;
        }

        ERR_SUCCESS
    });

    res.unwrap_or(ERR_PANIC)
}

/// Extracts a time slice between `start_ms` and `end_ms` and writes output to `output_path` (WAV).
#[no_mangle]
pub extern "C" fn attune_track_tweaks_slice(
    input_path: *const c_char,
    output_path: *const c_char,
    start_ms: u64,
    end_ms: u64,
) -> i32 {
    let res = catch_unwind(|| {
        let in_str = match parse_cstr(input_path) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let out_str = match parse_cstr(output_path) {
            Ok(s) => s,
            Err(e) => return e,
        };

        if start_ms >= end_ms {
            return ERR_INVALID_PARAM;
        }

        let audio = match decode_audio_file(in_str) {
            Ok(a) => a,
            Err(e) => {
                log::error!("Audio decode failed: {}", e);
                return ERR_DECODE_FAILED;
            }
        };

        let sliced_samples = slice_region(
            &audio.samples,
            start_ms,
            end_ms,
            audio.sample_rate,
            audio.channels,
        );
        let out_audio = AudioData::new(audio.channels, audio.sample_rate, sliced_samples);

        if let Err(e) = write_wav_file(out_str, &out_audio) {
            log::error!("WAV write failed: {}", e);
            return ERR_WRITE_FAILED;
        }

        ERR_SUCCESS
    });

    res.unwrap_or(ERR_PANIC)
}

/// Resamples audio with speed factor using Catmull-Rom Hermite interpolation and writes output to `output_path` (WAV).
#[no_mangle]
pub extern "C" fn attune_track_tweaks_speed(
    input_path: *const c_char,
    output_path: *const c_char,
    speed_factor: f32,
) -> i32 {
    let res = catch_unwind(|| {
        let in_str = match parse_cstr(input_path) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let out_str = match parse_cstr(output_path) {
            Ok(s) => s,
            Err(e) => return e,
        };

        if !speed_factor.is_finite() || speed_factor <= 0.0 {
            return ERR_INVALID_PARAM;
        }

        let audio = match decode_audio_file(in_str) {
            Ok(a) => a,
            Err(e) => {
                log::error!("Audio decode failed: {}", e);
                return ERR_DECODE_FAILED;
            }
        };

        let resampled_samples = match apply_speed_factor(&audio.samples, audio.channels, speed_factor) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Hermite resampling failed: {}", e);
                return ERR_INVALID_PARAM;
            }
        };

        let out_audio = AudioData::new(audio.channels, audio.sample_rate, resampled_samples);

        if let Err(e) = write_wav_file(out_str, &out_audio) {
            log::error!("WAV write failed: {}", e);
            return ERR_WRITE_FAILED;
        }

        ERR_SUCCESS
    });

    res.unwrap_or(ERR_PANIC)
}

// =========================================================================
// Non-Destructive Song Metadata C-ABI Layer (Audio Stream Untouched)
// =========================================================================

/// Reads song metadata and returns it as a JSON string.
/// The caller must free the returned string using `attune_track_tweaks_free_string`.
#[no_mangle]
pub extern "C" fn attune_track_tweaks_read_metadata(path: *const c_char) -> *mut c_char {
    let res = catch_unwind(|| {
        let path_str = match parse_cstr(path) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };

        match read_track_metadata(path_str) {
            Ok(meta) => match serde_json::to_string(&meta) {
                Ok(json) => match CString::new(json) {
                    Ok(cs) => cs.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                },
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        }
    });

    res.unwrap_or(std::ptr::null_mut())
}

/// Writes song metadata from a JSON string into the audio file WITHOUT TOUCHING the audio stream.
/// Returns 0 on success, or a negative error code.
#[no_mangle]
pub extern "C" fn attune_track_tweaks_write_metadata(
    path: *const c_char,
    metadata_json: *const c_char,
) -> i32 {
    let res = catch_unwind(|| {
        let path_str = match parse_cstr(path) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let json_str = match parse_cstr(metadata_json) {
            Ok(s) => s,
            Err(e) => return e,
        };

        let meta: TrackMetadata = match serde_json::from_str(json_str) {
            Ok(m) => m,
            Err(e) => {
                log::error!("Invalid metadata JSON: {}", e);
                return ERR_INVALID_PARAM;
            }
        };

        if let Err(e) = write_track_metadata(path_str, &meta) {
            log::error!("Failed to write track metadata: {}", e);
            return ERR_WRITE_FAILED;
        }

        ERR_SUCCESS
    });

    res.unwrap_or(ERR_PANIC)
}

/// Updates a single song metadata tag without touching the audio payload.
/// E.g. tag_name = "title", "artist", "album", "genre", "year", "track_number", "bpm", etc.
#[no_mangle]
pub extern "C" fn attune_track_tweaks_update_tag(
    path: *const c_char,
    tag_name: *const c_char,
    tag_value: *const c_char,
) -> i32 {
    let res = catch_unwind(|| {
        let path_str = match parse_cstr(path) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let name_str = match parse_cstr(tag_name) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let val_str = match parse_cstr(tag_value) {
            Ok(s) => s,
            Err(e) => return e,
        };

        let mut meta = read_track_metadata(path_str).unwrap_or_default();
        meta.set_tag(name_str, val_str);

        if let Err(e) = write_track_metadata(path_str, &meta) {
            log::error!("Failed to write tag: {}", e);
            return ERR_WRITE_FAILED;
        }

        ERR_SUCCESS
    });

    res.unwrap_or(ERR_PANIC)
}

/// Helper function to parse C strings safely.
fn parse_cstr<'a>(ptr: *const c_char) -> Result<&'a str, i32> {
    if ptr.is_null() {
        return Err(ERR_NULL_PTR);
    }
    unsafe {
        match CStr::from_ptr(ptr).to_str() {
            Ok(s) => Ok(s),
            Err(_) => Err(ERR_INVALID_UTF8),
        }
    }
}

// =========================================================================
// Canonical `attune_metadata_changer_*` C-ABI Entrypoints
// =========================================================================

#[no_mangle]
pub extern "C" fn attune_metadata_changer_version() -> *const c_char {
    attune_track_tweaks_version()
}

#[no_mangle]
pub extern "C" fn attune_metadata_changer_free_string(s: *mut c_char) {
    attune_track_tweaks_free_string(s);
}

#[no_mangle]
pub extern "C" fn attune_metadata_changer_normalize(
    input_path: *const c_char,
    output_path: *const c_char,
    target_peak_db: f32,
) -> i32 {
    attune_track_tweaks_normalize(input_path, output_path, target_peak_db)
}

#[no_mangle]
pub extern "C" fn attune_metadata_changer_trim_silence(
    input_path: *const c_char,
    output_path: *const c_char,
    threshold_db: f32,
) -> i32 {
    attune_track_tweaks_trim_silence(input_path, output_path, threshold_db)
}

#[no_mangle]
pub extern "C" fn attune_metadata_changer_slice(
    input_path: *const c_char,
    output_path: *const c_char,
    start_ms: u64,
    end_ms: u64,
) -> i32 {
    attune_track_tweaks_slice(input_path, output_path, start_ms, end_ms)
}

#[no_mangle]
pub extern "C" fn attune_metadata_changer_speed(
    input_path: *const c_char,
    output_path: *const c_char,
    speed_factor: f32,
) -> i32 {
    attune_track_tweaks_speed(input_path, output_path, speed_factor)
}

#[no_mangle]
pub extern "C" fn attune_metadata_changer_read_metadata_json(file_path: *const c_char) -> *mut c_char {
    attune_track_tweaks_read_metadata(file_path)
}

#[no_mangle]
pub extern "C" fn attune_metadata_changer_write_metadata_json(
    file_path: *const c_char,
    metadata_json: *const c_char,
) -> i32 {
    attune_track_tweaks_write_metadata(file_path, metadata_json)
}

#[no_mangle]
pub extern "C" fn attune_metadata_changer_set_tag(
    file_path: *const c_char,
    tag_name: *const c_char,
    tag_value: *const c_char,
) -> i32 {
    attune_track_tweaks_update_tag(file_path, tag_name, tag_value)
}
