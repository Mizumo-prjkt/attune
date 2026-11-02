//! # attune-track-tweaks
//!
//! Audio track tweaks and metadata management library for the Attune music player and engine.
//!
//! This crate provides:
//! - Song metadata editing without touching or re-encoding the audio payload (FLAC Vorbis, MP3 ID3v2.4, WAV INFO, sidecars).
//! - Peak gain normalization (`normalize_gain`).
//! - Dynamic silence boundary detection and trimming (`detect_silence_bounds`, `trim_silence`).
//! - Cue-point slice / region extraction (`slice_region`).
//! - Speed / tempo adjustments via Catmull-Rom cubic Hermite fractional resampling (`apply_speed_factor`).
//! - Audio decoding via Symphonia and WAV writing via Hound.
//! - Thread-safe C-ABI FFI layer with catch_unwind and status codes for foreign platform interop.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod audio;
pub mod dsp;
pub mod error;
pub mod ffi;
pub mod metadata;

// Re-exports
pub use audio::{decode_audio_file, write_wav_file, AudioData};
pub use dsp::{
    apply_speed_factor, detect_silence_bounds, interpolate_hermite, normalize_gain,
    normalize_gain_copy, slice_region, trim_silence,
};
pub use error::TrackTweaksError;
pub use ffi::*;
pub use metadata::{read_track_metadata, write_track_metadata, TrackMetadata};
