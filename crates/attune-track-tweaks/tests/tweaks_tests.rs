use std::ffi::{CStr, CString};
use std::f32::consts::PI;

use attune_track_tweaks::audio::{decode_audio_file, write_wav_file, AudioData};
use attune_track_tweaks::dsp::{
    apply_speed_factor, detect_silence_bounds, normalize_gain, normalize_gain_copy, slice_region,
    trim_silence,
};
use attune_track_tweaks::ffi::*;
use attune_track_tweaks::metadata::{read_track_metadata, write_track_metadata, TrackMetadata};

/// Generates synthetic stereo audio with a sine tone
fn generate_synthetic_audio(
    sample_rate: u32,
    lead_silence_ms: u32,
    tone_ms: u32,
    tail_silence_ms: u32,
    tone_freq: f32,
    tone_amp: f32,
) -> AudioData {
    let channels = 2;
    let sr = sample_rate as f32;
    let lead_frames = ((lead_silence_ms as f32 / 1000.0) * sr) as usize;
    let tone_frames = ((tone_ms as f32 / 1000.0) * sr) as usize;
    let tail_frames = ((tail_silence_ms as f32 / 1000.0) * sr) as usize;
    let total_frames = lead_frames + tone_frames + tail_frames;

    let mut samples = vec![0.0f32; total_frames * channels];

    for i in 0..tone_frames {
        let frame = lead_frames + i;
        let t = i as f32 / sr;
        let s = (2.0 * PI * tone_freq * t).sin() * tone_amp;
        samples[frame * channels] = s;
        samples[frame * channels + 1] = s;
    }

    AudioData::new(channels, sample_rate, samples)
}

#[test]
fn test_normalize_gain() {
    let mut samples = vec![0.1, -0.4, 0.5, -0.2];
    let target_db = -1.0; // 10^(-1/20) = ~0.89125
    let target_linear = 10.0f32.powf(target_db / 20.0);

    let gain = normalize_gain(&mut samples, target_db).expect("normalization should succeed");
    assert!((gain - (target_linear / 0.5)).abs() < 1e-5);

    let max_peak = samples.iter().copied().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!((max_peak - target_linear).abs() < 1e-5);

    // Test copy variant
    let (copy, gain_copy) =
        normalize_gain_copy(&[0.2, -0.8], 0.0).expect("copy normalization should succeed");
    assert!((gain_copy - (1.0 / 0.8)).abs() < 1e-5);
    assert!((copy[1].abs() - 1.0).abs() < 1e-5);

    // Test silence handling (all zeros)
    let mut silence = vec![0.0; 100];
    let silence_gain = normalize_gain(&mut silence, -1.0).expect("silence should not error");
    assert_eq!(silence_gain, 1.0);

    // Test invalid target parameter
    let mut err_samples = vec![0.1];
    assert!(normalize_gain(&mut err_samples, f32::NAN).is_err());
}

#[test]
fn test_silence_detection_and_trimming() {
    let sr = 44100;
    // 500ms silence + 1000ms tone + 500ms silence = 2000ms total
    let audio = generate_synthetic_audio(sr, 500, 1000, 500, 440.0, 0.7);

    let (start_frame, end_frame) =
        detect_silence_bounds(&audio.samples, audio.channels, audio.sample_rate, -45.0);

    let start_ms = ((start_frame as f64 / sr as f64) * 1000.0) as u64;
    let end_ms = ((end_frame as f64 / sr as f64) * 1000.0) as u64;

    // Detection within 25ms tolerance (analysis window is 20ms)
    assert!(
        (start_ms as i64 - 500).abs() <= 25,
        "start_ms was {}",
        start_ms
    );
    assert!((end_ms as i64 - 1500).abs() <= 25, "end_ms was {}", end_ms);

    // Trim silence
    let trimmed = trim_silence(&audio.samples, audio.channels, audio.sample_rate, -45.0);
    let trimmed_frames = trimmed.len() / audio.channels;
    let trimmed_ms = ((trimmed_frames as f64 / sr as f64) * 1000.0) as u64;

    assert!(
        (trimmed_ms as i64 - 1000).abs() <= 50,
        "trimmed_ms was {}",
        trimmed_ms
    );

    // Test all silence
    let silence = vec![0.0f32; 44100 * 2];
    let (s, e) = detect_silence_bounds(&silence, 2, 44100, -45.0);
    assert_eq!((s, e), (0, 0));
    let trimmed_empty = trim_silence(&silence, 2, 44100, -45.0);
    assert!(trimmed_empty.is_empty());
}

#[test]
fn test_slice_region() {
    let sr = 44100;
    let audio = generate_synthetic_audio(sr, 0, 2000, 0, 440.0, 0.5);

    // Slice from 500ms to 1200ms (700ms duration)
    let sliced = slice_region(&audio.samples, 500, 1200, audio.sample_rate, audio.channels);
    let sliced_frames = sliced.len() / audio.channels;
    let expected_frames = ((0.700 * sr as f64).round()) as usize;

    assert_eq!(sliced_frames, expected_frames);

    // Slice with start >= end returns empty
    let empty = slice_region(&audio.samples, 1000, 500, audio.sample_rate, audio.channels);
    assert!(empty.is_empty());

    // Slice beyond audio length is clamped safely
    let clamped = slice_region(&audio.samples, 1500, 5000, audio.sample_rate, audio.channels);
    let clamped_frames = clamped.len() / audio.channels;
    let expected_clamped = ((0.500 * sr as f64).round()) as usize;
    assert_eq!(clamped_frames, expected_clamped);
}

#[test]
fn test_speed_adjustment_hermite() {
    let sr = 44100;
    let audio = generate_synthetic_audio(sr, 0, 1000, 0, 440.0, 0.5);
    let orig_frames = audio.num_frames();

    // Speed 1.0 should preserve frame count and be virtually identical
    let normal = apply_speed_factor(&audio.samples, audio.channels, 1.0)
        .expect("speed 1.0 should succeed");
    assert_eq!(normal.len(), audio.samples.len());
    for i in 0..normal.len() {
        assert!((normal[i] - audio.samples[i]).abs() < 1e-5);
    }

    // Speed 2.0 (2x speed, half duration)
    let fast = apply_speed_factor(&audio.samples, audio.channels, 2.0)
        .expect("speed 2.0 should succeed");
    let fast_frames = fast.len() / audio.channels;
    assert_eq!(fast_frames, (orig_frames as f64 / 2.0).round() as usize);

    // Speed 0.5 (half speed, double duration)
    let slow = apply_speed_factor(&audio.samples, audio.channels, 0.5)
        .expect("speed 0.5 should succeed");
    let slow_frames = slow.len() / audio.channels;
    assert_eq!(slow_frames, (orig_frames as f64 / 0.5).round() as usize);

    // Invalid speed factor error
    assert!(apply_speed_factor(&audio.samples, audio.channels, 0.0).is_err());
    assert!(apply_speed_factor(&audio.samples, audio.channels, -1.5).is_err());
    assert!(apply_speed_factor(&audio.samples, audio.channels, f32::NAN).is_err());
}

#[test]
fn test_audio_file_decode_and_wav_write() {
    let tmp_dir = std::env::temp_dir();
    let wav_path = tmp_dir.join("attune_test_roundtrip.wav");

    let orig_audio = generate_synthetic_audio(44100, 100, 500, 100, 880.0, 0.6);
    write_wav_file(&wav_path, &orig_audio).expect("write wav file should succeed");

    let decoded = decode_audio_file(&wav_path).expect("decode audio file should succeed");
    assert_eq!(decoded.channels, orig_audio.channels);
    assert_eq!(decoded.sample_rate, orig_audio.sample_rate);
    assert_eq!(decoded.num_frames(), orig_audio.num_frames());

    // Verify sample values within 16-bit quantization margin (1 / 32767 ≈ 0.000035)
    for (s_dec, s_orig) in decoded.samples.iter().zip(orig_audio.samples.iter()) {
        assert!((s_dec - s_orig).abs() < 0.001);
    }

    let _ = std::fs::remove_file(wav_path);
}

#[test]
fn test_c_abi_layer() {
    // 1. Version check
    let v_ptr = attune_track_tweaks_version();
    assert!(!v_ptr.is_null());
    let v_str = unsafe { CStr::from_ptr(v_ptr) }.to_str().unwrap();
    assert_eq!(v_str, "0.1.0");

    let tmp_dir = std::env::temp_dir();
    let in_wav = tmp_dir.join("attune_c_abi_in.wav");
    let out_norm_wav = tmp_dir.join("attune_c_abi_out_norm.wav");
    let out_trim_wav = tmp_dir.join("attune_c_abi_out_trim.wav");
    let out_slice_wav = tmp_dir.join("attune_c_abi_out_slice.wav");
    let out_speed_wav = tmp_dir.join("attune_c_abi_out_speed.wav");

    let audio = generate_synthetic_audio(44100, 200, 600, 200, 440.0, 0.3);
    write_wav_file(&in_wav, &audio).unwrap();

    let in_c = CString::new(in_wav.to_str().unwrap()).unwrap();
    let out_norm_c = CString::new(out_norm_wav.to_str().unwrap()).unwrap();
    let out_trim_c = CString::new(out_trim_wav.to_str().unwrap()).unwrap();
    let out_slice_c = CString::new(out_slice_wav.to_str().unwrap()).unwrap();
    let out_speed_c = CString::new(out_speed_wav.to_str().unwrap()).unwrap();

    // 2. Normalize FFI
    let res_norm = attune_track_tweaks_normalize(in_c.as_ptr(), out_norm_c.as_ptr(), -1.0);
    assert_eq!(res_norm, ERR_SUCCESS);
    let norm_decoded = decode_audio_file(&out_norm_wav).unwrap();
    let norm_peak = norm_decoded
        .samples
        .iter()
        .copied()
        .fold(0.0f32, |m, s| m.max(s.abs()));
    let target_linear = 10.0f32.powf(-1.0 / 20.0);
    assert!((norm_peak - target_linear).abs() < 0.01);

    // 3. Trim silence FFI
    let res_trim = attune_track_tweaks_trim_silence(in_c.as_ptr(), out_trim_c.as_ptr(), -45.0);
    assert_eq!(res_trim, ERR_SUCCESS);
    let trim_decoded = decode_audio_file(&out_trim_wav).unwrap();
    let trim_ms = trim_decoded.duration_ms();
    assert!(
        (trim_ms as i64 - 600).abs() <= 50,
        "trimmed ms was {}",
        trim_ms
    );

    // 4. Slice FFI
    let res_slice = attune_track_tweaks_slice(in_c.as_ptr(), out_slice_c.as_ptr(), 100, 400);
    assert_eq!(res_slice, ERR_SUCCESS);
    let slice_decoded = decode_audio_file(&out_slice_wav).unwrap();
    assert!((slice_decoded.duration_ms() as i64 - 300).abs() <= 10);

    // 5. Speed factor FFI
    let res_speed = attune_track_tweaks_speed(in_c.as_ptr(), out_speed_c.as_ptr(), 1.5);
    assert_eq!(res_speed, ERR_SUCCESS);
    let speed_decoded = decode_audio_file(&out_speed_wav).unwrap();
    let expected_speed_ms = ((1000.0 / 1.5) as f64).round() as u64;
    assert!((speed_decoded.duration_ms() as i64 - expected_speed_ms as i64).abs() <= 10);

    // 6. Error handling checks
    assert_eq!(
        attune_track_tweaks_normalize(std::ptr::null(), out_norm_c.as_ptr(), -1.0),
        ERR_NULL_PTR
    );
    assert_eq!(
        attune_track_tweaks_slice(in_c.as_ptr(), out_slice_c.as_ptr(), 500, 200),
        ERR_INVALID_PARAM
    );
    assert_eq!(
        attune_track_tweaks_speed(in_c.as_ptr(), out_speed_c.as_ptr(), -0.5),
        ERR_INVALID_PARAM
    );

    // Clean up
    let _ = std::fs::remove_file(in_wav);
    let _ = std::fs::remove_file(out_norm_wav);
    let _ = std::fs::remove_file(out_trim_wav);
    let _ = std::fs::remove_file(out_slice_wav);
    let _ = std::fs::remove_file(out_speed_wav);
}

#[test]
fn test_song_metadata_editing_without_touching_audio() {
    let tmp_dir = std::env::temp_dir();
    let wav_path = tmp_dir.join("attune_metadata_test.wav");

    // Generate test audio and write initial WAV file
    let test_audio = generate_synthetic_audio(44100, 100, 400, 100, 523.25, 0.5);
    write_wav_file(&wav_path, &test_audio).unwrap();

    // Decode audio before metadata edit
    let before_audio = decode_audio_file(&wav_path).expect("decode before should succeed");

    // Prepare metadata to write
    let mut meta = TrackMetadata::default();
    meta.title = Some("Gotta Go Fast (Attune Remix)".to_string());
    meta.artist = Some("Mizumo Sound Lab".to_string());
    meta.album = Some("Neon Velocity".to_string());
    meta.genre = Some("Drum & Bass".to_string());
    meta.year = Some(2026);
    meta.track_number = Some(7);
    meta.bpm = Some(174.0);
    meta.replaygain_track_gain_db = Some(-3.25);
    meta.start_trim_ms = Some(100);
    meta.end_trim_ms = Some(500);
    meta.speed_factor = Some(1.05);

    // Write metadata WITHOUT touching the audio
    write_track_metadata(&wav_path, &meta).expect("write metadata should succeed");

    // Decode audio again and assert that audio samples are 100% untouched bit-for-bit
    let after_audio = decode_audio_file(&wav_path).expect("decode audio should succeed");
    assert_eq!(after_audio.channels, before_audio.channels);
    assert_eq!(after_audio.sample_rate, before_audio.sample_rate);
    assert_eq!(after_audio.samples.len(), before_audio.samples.len());
    for (s_after, s_before) in after_audio.samples.iter().zip(before_audio.samples.iter()) {
        assert_eq!(
            s_after, s_before,
            "Audio samples were modified during metadata edit!"
        );
    }

    // Verify metadata read back
    let read_back = read_track_metadata(&wav_path).expect("read metadata should succeed");
    assert_eq!(read_back.title.as_deref(), Some("Gotta Go Fast (Attune Remix)"));
    assert_eq!(read_back.artist.as_deref(), Some("Mizumo Sound Lab"));
    assert_eq!(read_back.album.as_deref(), Some("Neon Velocity"));
    assert_eq!(read_back.genre.as_deref(), Some("Drum & Bass"));
    assert_eq!(read_back.year, Some(2026));
    assert_eq!(read_back.track_number, Some(7));

    // Test C-ABI metadata functions
    let path_c = CString::new(wav_path.to_str().unwrap()).unwrap();

    // Update single tag via C-ABI
    let tag_name = CString::new("title").unwrap();
    let tag_val = CString::new("Gotta Go Faster").unwrap();
    let res_tag = attune_track_tweaks_update_tag(path_c.as_ptr(), tag_name.as_ptr(), tag_val.as_ptr());
    assert_eq!(res_tag, ERR_SUCCESS);

    // Read metadata JSON via C-ABI
    let json_ptr = attune_track_tweaks_read_metadata(path_c.as_ptr());
    assert!(!json_ptr.is_null());
    let json_str = unsafe { CStr::from_ptr(json_ptr) }.to_str().unwrap();
    let parsed: TrackMetadata = serde_json::from_str(json_str).unwrap();
    assert_eq!(parsed.title.as_deref(), Some("Gotta Go Faster"));
    attune_track_tweaks_free_string(json_ptr);

    // Clean up
    let _ = std::fs::remove_file(wav_path);
}

#[test]
fn test_flac_and_mp3_metadata_untouched_audio() {
    let tmp_dir = std::env::temp_dir();

    // 1. Test FLAC byte-for-byte audio stream preservation
    let flac_path = tmp_dir.join("attune_test_untouched.flac");
    let mut flac_bytes = Vec::new();
    flac_bytes.extend_from_slice(b"fLaC");
    // STREAMINFO block header (is_last = 1, type = 0, length = 34)
    flac_bytes.push(0x80);
    flac_bytes.extend_from_slice(&[0x00, 0x00, 0x22]);
    let streaminfo = vec![0x42u8; 34];
    flac_bytes.extend_from_slice(&streaminfo);
    // Exact audio frame bytes that must remain untouched
    let original_flac_audio_frames = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78];
    flac_bytes.extend_from_slice(&original_flac_audio_frames);

    std::fs::write(&flac_path, &flac_bytes).unwrap();

    let mut flac_meta = TrackMetadata::default();
    flac_meta.title = Some("Preserved Audio Track".to_string());
    flac_meta.artist = Some("Lossless Hero".to_string());
    flac_meta.bpm = Some(128.0);
    flac_meta.replaygain_track_gain_db = Some(-4.5);

    write_track_metadata(&flac_path, &flac_meta).expect("write flac metadata should succeed");

    let updated_flac_bytes = std::fs::read(&flac_path).unwrap();
    // Verify audio frame bytes at the end of the file are 100% bit-for-bit identical
    assert!(updated_flac_bytes.ends_with(&original_flac_audio_frames));

    // 2. Test MP3 byte-for-byte audio stream preservation
    let mp3_path = tmp_dir.join("attune_test_untouched.mp3");
    let mut mp3_bytes = Vec::new();
    // Minimal initial ID3v2.3 tag (10 bytes header + 10 bytes empty frame)
    mp3_bytes.extend_from_slice(b"ID3\x03\x00\x00\x00\x00\x00\x00");
    // Raw MP3 audio sync frames that must remain untouched
    let original_mp3_audio_frames = vec![0xFF, 0xFB, 0x90, 0x64, 0xCA, 0xFE, 0xBA, 0xBE];
    mp3_bytes.extend_from_slice(&original_mp3_audio_frames);

    std::fs::write(&mp3_path, &mp3_bytes).unwrap();

    let mut mp3_meta = TrackMetadata::default();
    mp3_meta.title = Some("MP3 Untouched Audio".to_string());
    mp3_meta.artist = Some("Bit Accurate".to_string());
    mp3_meta.year = Some(2026);

    write_track_metadata(&mp3_path, &mp3_meta).expect("write mp3 metadata should succeed");

    let updated_mp3_bytes = std::fs::read(&mp3_path).unwrap();
    assert!(updated_mp3_bytes.ends_with(&original_mp3_audio_frames));

    // Clean up
    let _ = std::fs::remove_file(flac_path);
    let _ = std::fs::remove_file(mp3_path);
}
