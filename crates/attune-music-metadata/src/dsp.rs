use crate::error::TrackTweaksError;

/// Normalizes the peak amplitude of interleaved audio samples to `target_peak_db` dBFS in-place.
/// Returns the applied linear gain multiplier.
/// If all samples are zero or target_peak_db is invalid, returns an error or 1.0.
pub fn normalize_gain(samples: &mut [f32], target_peak_db: f32) -> Result<f32, TrackTweaksError> {
    if !target_peak_db.is_finite() {
        return Err(TrackTweaksError::InvalidParameter(
            "target_peak_db must be finite".to_string(),
        ));
    }
    if samples.is_empty() {
        return Ok(1.0);
    }

    let mut current_peak = 0.0f32;
    for &s in samples.iter() {
        let abs = s.abs();
        if abs > current_peak {
            current_peak = abs;
        }
    }

    if current_peak <= 1e-9 {
        // Complete silence, nothing to amplify
        return Ok(1.0);
    }

    let target_linear = 10.0f32.powf(target_peak_db / 20.0);
    let gain = target_linear / current_peak;

    for s in samples.iter_mut() {
        *s *= gain;
    }

    Ok(gain)
}

/// Normalizes the peak amplitude of interleaved audio samples to `target_peak_db` dBFS,
/// returning a newly allocated vector and the applied linear gain multiplier.
pub fn normalize_gain_copy(
    samples: &[f32],
    target_peak_db: f32,
) -> Result<(Vec<f32>, f32), TrackTweaksError> {
    let mut copy = samples.to_vec();
    let gain = normalize_gain(&mut copy, target_peak_db)?;
    Ok((copy, gain))
}

/// Detects audible start and end frame indices based on an RMS energy threshold in dBFS.
/// Returns `(start_frame, end_frame)`. If the entire track is below the threshold, returns `(0, 0)`.
pub fn detect_silence_bounds(
    samples: &[f32],
    channels: usize,
    sample_rate: u32,
    threshold_db: f32,
) -> (usize, usize) {
    if samples.is_empty() || channels == 0 || sample_rate == 0 {
        return (0, 0);
    }

    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return (0, 0);
    }

    // 20ms analysis window (minimum 1 frame)
    let window_frames = ((sample_rate as usize) / 50).max(1);
    let thresh_linear = 10.0f32.powf(threshold_db / 20.0);
    let inv_ch = 1.0f32 / (channels as f32);

    let mut start_frame = 0;
    let mut found_start = false;

    // Scan forward for audible start
    for chunk_start in (0..total_frames).step_by(window_frames) {
        let chunk_end = (chunk_start + window_frames).min(total_frames);
        let chunk_len = chunk_end - chunk_start;
        let mut sum_sq = 0.0f32;

        for f in chunk_start..chunk_end {
            let offset = f * channels;
            for ch in 0..channels {
                let s = samples[offset + ch];
                sum_sq += s * s;
            }
        }

        let rms = (sum_sq * inv_ch / (chunk_len as f32)).sqrt();
        if rms >= thresh_linear {
            start_frame = chunk_start;
            found_start = true;
            break;
        }
    }

    if !found_start {
        return (0, 0);
    }

    // Scan backward for audible end
    let mut end_frame = total_frames;
    let last_chunk_start = (total_frames / window_frames) * window_frames;
    let mut chunk_start = last_chunk_start;

    loop {
        let chunk_end = (chunk_start + window_frames).min(total_frames);
        let chunk_len = chunk_end - chunk_start;
        if chunk_len > 0 {
            let mut sum_sq = 0.0f32;
            for f in chunk_start..chunk_end {
                let offset = f * channels;
                for ch in 0..channels {
                    let s = samples[offset + ch];
                    sum_sq += s * s;
                }
            }
            let rms = (sum_sq * inv_ch / (chunk_len as f32)).sqrt();
            if rms >= thresh_linear {
                end_frame = chunk_end;
                break;
            }
        }
        if chunk_start < window_frames {
            break;
        }
        chunk_start -= window_frames;
    }

    (start_frame, end_frame)
}

/// Trims leading and trailing silence based on threshold_db, returning a newly allocated vector of interleaved samples.
pub fn trim_silence(
    samples: &[f32],
    channels: usize,
    sample_rate: u32,
    threshold_db: f32,
) -> Vec<f32> {
    if samples.is_empty() || channels == 0 || sample_rate == 0 {
        return Vec::new();
    }
    let (start_frame, end_frame) =
        detect_silence_bounds(samples, channels, sample_rate, threshold_db);
    if start_frame >= end_frame {
        return Vec::new();
    }

    let start_idx = start_frame * channels;
    let end_idx = (end_frame * channels).min(samples.len());
    samples[start_idx..end_idx].to_vec()
}

/// Extracts a subregion of interleaved audio between `start_ms` and `end_ms`.
/// Clamps bounds safely to the audio duration.
pub fn slice_region(
    samples: &[f32],
    start_ms: u64,
    end_ms: u64,
    sample_rate: u32,
    channels: usize,
) -> Vec<f32> {
    if samples.is_empty() || channels == 0 || sample_rate == 0 || start_ms >= end_ms {
        return Vec::new();
    }

    let total_frames = samples.len() / channels;
    let start_frame = (((start_ms as f64) / 1000.0) * (sample_rate as f64)).round() as usize;
    let end_frame = (((end_ms as f64) / 1000.0) * (sample_rate as f64)).round() as usize;

    let start_frame = start_frame.min(total_frames);
    let end_frame = end_frame.min(total_frames);

    if start_frame >= end_frame {
        return Vec::new();
    }

    let start_idx = start_frame * channels;
    let end_idx = end_frame * channels;
    samples[start_idx..end_idx].to_vec()
}

/// Catmull-Rom cubic Hermite 4-point spline interpolation for fractional sample reconstruction.
#[inline]
pub fn interpolate_hermite(y0: f32, y1: f32, y2: f32, y3: f32, frac: f32) -> f32 {
    let c0 = y1;
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * frac + c2) * frac + c1) * frac + c0
}

/// Changes the playback speed of interleaved audio samples using Catmull-Rom / Hermite fractional resampling.
/// A `speed_factor > 1.0` speeds up the audio (shorter duration), while `< 1.0` slows down (longer duration).
pub fn apply_speed_factor(
    samples: &[f32],
    channels: usize,
    speed_factor: f32,
) -> Result<Vec<f32>, TrackTweaksError> {
    if !speed_factor.is_finite() || speed_factor <= 0.0 {
        return Err(TrackTweaksError::InvalidParameter(
            "speed_factor must be positive and finite".to_string(),
        ));
    }
    if channels == 0 {
        return Err(TrackTweaksError::InvalidParameter(
            "channels must be > 0".to_string(),
        ));
    }
    if samples.is_empty() {
        return Ok(Vec::new());
    }

    let total_in_frames = samples.len() / channels;
    if total_in_frames == 0 {
        return Ok(Vec::new());
    }

    let total_out_frames = ((total_in_frames as f64) / (speed_factor as f64)).round() as usize;
    if total_out_frames == 0 {
        return Ok(Vec::new());
    }

    let mut out = Vec::with_capacity(total_out_frames * channels);

    for out_frame in 0..total_out_frames {
        let in_pos = (out_frame as f64) * (speed_factor as f64);
        let idx1 = in_pos.floor() as usize;
        let frac = (in_pos - idx1 as f64) as f32;

        let idx0 = idx1.saturating_sub(1);
        let idx2 = (idx1 + 1).min(total_in_frames - 1);
        let idx3 = (idx1 + 2).min(total_in_frames - 1);

        for ch in 0..channels {
            let y0 = samples[idx0 * channels + ch];
            let y1 = if idx1 < total_in_frames {
                samples[idx1 * channels + ch]
            } else {
                y0
            };
            let y2 = samples[idx2 * channels + ch];
            let y3 = samples[idx3 * channels + ch];

            let sample = interpolate_hermite(y0, y1, y2, y3, frac);
            out.push(sample);
        }
    }

    Ok(out)
}
