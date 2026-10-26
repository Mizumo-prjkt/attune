use crate::audio::AudioBuffer;
use crate::schema::{DeckAutomation, TransitionRecipe};
use hound::{WavSpec, WavWriter};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("I/O or WAV error: {0}")]
    Wav(#[from] hound::Error),
    #[error("Render processing error: {0}")]
    Process(String),
}

pub struct TransitionRenderer;

impl TransitionRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Renders the transition window between Track A and Track B into an output WAV file (with default 5s pre-roll and 7s post-roll)
    pub fn render_to_wav<P: AsRef<Path>>(
        &self,
        track_a: &AudioBuffer,
        track_b: &AudioBuffer,
        recipe: &TransitionRecipe,
        output_path: P,
    ) -> Result<(), RenderError> {
        self.render_with_context(track_a, track_b, recipe, output_path, 5000, 7000)
    }

    /// Renders the transition with configurable pre-roll and post-roll musical context
    pub fn render_with_context<P: AsRef<Path>>(
        &self,
        track_a: &AudioBuffer,
        track_b: &AudioBuffer,
        recipe: &TransitionRecipe,
        output_path: P,
        pre_roll_ms: u64,
        post_roll_ms: u64,
    ) -> Result<(), RenderError> {
        let sr = track_a.sample_rate;
        let timing = &recipe.timing;

        let a_start = timing.track_a_transition_start_ms;
        let a_exit = timing.track_a_exit_end_ms;
        let b_entry = timing.track_b_entry_start_ms;

        let transition_duration_ms = a_exit.saturating_sub(a_start).max(500);

        let pre_roll_start = a_start.saturating_sub(pre_roll_ms);
        let actual_pre_roll_ms = a_start.saturating_sub(pre_roll_start);

        let post_roll_end = (b_entry + transition_duration_ms + post_roll_ms).min(track_b.duration_ms());
        let post_roll_start = b_entry + transition_duration_ms;
        let actual_post_roll_ms = post_roll_end.saturating_sub(post_roll_start);

        let total_ms = actual_pre_roll_ms + transition_duration_ms + actual_post_roll_ms;
        let total_frames = ((total_ms as f64 / 1000.0) * sr as f64) as usize;

        let slice_a_pre = track_a.slice_ms(pre_roll_start, a_start);
        let slice_a_trans = track_a.slice_ms(a_start, a_exit);

        let slice_b_trans = track_b.slice_ms(b_entry, b_entry + transition_duration_ms);
        let slice_b_post = track_b.slice_ms(post_roll_start, post_roll_end);

        let pre_frames = ((actual_pre_roll_ms as f64 / 1000.0) * sr as f64) as usize;
        let trans_frames = ((transition_duration_ms as f64 / 1000.0) * sr as f64) as usize;

        let mut out_left = vec![0.0f32; total_frames];
        let mut out_right = vec![0.0f32; total_frames];

        // Reverb freeze state for Archetype 4
        let mut freeze_buf = vec![0.0f32; (sr as f32 * 0.25) as usize];
        let mut freeze_idx = 0;

        for frame_idx in 0..total_frames {
            let (l, r) = if frame_idx < pre_frames {
                // Pre-roll: Track A playing cleanly
                let l = slice_a_pre.channels.get(0).and_then(|c| c.get(frame_idx)).copied().unwrap_or(0.0);
                let r = slice_a_pre.channels.get(1).and_then(|c| c.get(frame_idx)).copied().unwrap_or(l);
                (l, r)
            } else if frame_idx < pre_frames + trans_frames {
                // Transition window
                let trans_idx = frame_idx - pre_frames;
                let offset_ms = ((trans_idx as f64 / sr as f64) * 1000.0) as u64;
                let (auto_a, auto_b) = interpolate_keyframes(&recipe.dsp_automation_keyframes, offset_ms);

                let a_l = slice_a_trans.channels.get(0).and_then(|c| c.get(trans_idx)).copied().unwrap_or(0.0);
                let a_r = slice_a_trans.channels.get(1).and_then(|c| c.get(trans_idx)).copied().unwrap_or(a_l);

                let b_l = slice_b_trans.channels.get(0).and_then(|c| c.get(trans_idx)).copied().unwrap_or(0.0);
                let b_r = slice_b_trans.channels.get(1).and_then(|c| c.get(trans_idx)).copied().unwrap_or(b_l);

                let gain_a = db_to_linear(auto_a.gain_db);
                let mut processed_a_l = a_l * gain_a * db_to_linear(auto_a.eq_mid_db);
                let mut processed_a_r = a_r * gain_a * db_to_linear(auto_a.eq_mid_db);

                if auto_a.reverb_freeze_send > 0.01 {
                    if trans_idx < freeze_buf.len() {
                        freeze_buf[trans_idx] = (a_l + a_r) * 0.5;
                    }
                    let freeze_sample = freeze_buf[freeze_idx % freeze_buf.len()];
                    freeze_idx += 1;
                    let decay = (-3.0 * (trans_idx as f32 / trans_frames as f32)).exp();
                    let wash = freeze_sample * auto_a.reverb_freeze_send * decay;
                    processed_a_l += wash;
                    processed_a_r += wash;
                }

                let gain_b = db_to_linear(auto_b.gain_db);
                let processed_b_l = b_l * gain_b * db_to_linear(auto_b.eq_mid_db);
                let processed_b_r = b_r * gain_b * db_to_linear(auto_b.eq_mid_db);

                (processed_a_l + processed_b_l, processed_a_r + processed_b_r)
            } else {
                // Post-roll: Track B playing cleanly
                let post_idx = frame_idx - (pre_frames + trans_frames);
                let l = slice_b_post.channels.get(0).and_then(|c| c.get(post_idx)).copied().unwrap_or(0.0);
                let r = slice_b_post.channels.get(1).and_then(|c| c.get(post_idx)).copied().unwrap_or(l);
                (l, r)
            };

            out_left[frame_idx] = l.clamp(-1.0, 1.0);
            out_right[frame_idx] = r.clamp(-1.0, 1.0);
        }

        // Write WAV
        let spec = WavSpec {
            channels: 2,
            sample_rate: sr,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, spec)?;
        for i in 0..total_frames {
            let s_l = (out_left[i] * 32767.0) as i16;
            let s_r = (out_right[i] * 32767.0) as i16;
            writer.write_sample(s_l)?;
            writer.write_sample(s_r)?;
        }
        writer.finalize()?;

        Ok(())
    }
}

fn db_to_linear(db: f32) -> f32 {
    if db <= -59.0 {
        0.0
    } else {
        10.0f32.powf(db / 20.0)
    }
}

fn interpolate_keyframes(
    keyframes: &[crate::schema::DspAutomationKeyframe],
    offset_ms: u64,
) -> (DeckAutomation, DeckAutomation) {
    if keyframes.is_empty() {
        return (DeckAutomation::default(), DeckAutomation::default());
    }

    if offset_ms <= keyframes[0].offset_ms {
        return (keyframes[0].deck_a.clone(), keyframes[0].deck_b.clone());
    }

    for window in keyframes.windows(2) {
        let k1 = &window[0];
        let k2 = &window[1];
        if offset_ms >= k1.offset_ms && offset_ms <= k2.offset_ms {
            let t = if k2.offset_ms > k1.offset_ms {
                (offset_ms - k1.offset_ms) as f32 / (k2.offset_ms - k1.offset_ms) as f32
            } else {
                0.0
            };

            let interp_deck = |d1: &DeckAutomation, d2: &DeckAutomation| DeckAutomation {
                gain_db: d1.gain_db + t * (d2.gain_db - d1.gain_db),
                eq_sub_db: d1.eq_sub_db + t * (d2.eq_sub_db - d1.eq_sub_db),
                eq_mid_db: d1.eq_mid_db + t * (d2.eq_mid_db - d1.eq_mid_db),
                eq_high_db: d1.eq_high_db + t * (d2.eq_high_db - d1.eq_high_db),
                stretch_ratio: d1.stretch_ratio + t * (d2.stretch_ratio - d1.stretch_ratio),
                pitch_cents: (d1.pitch_cents as f32 + t * (d2.pitch_cents - d1.pitch_cents) as f32) as i32,
                reverb_freeze_send: d1.reverb_freeze_send + t * (d2.reverb_freeze_send - d1.reverb_freeze_send),
            };

            return (interp_deck(&k1.deck_a, &k2.deck_a), interp_deck(&k1.deck_b, &k2.deck_b));
        }
    }

    let last = keyframes.last().unwrap();
    (last.deck_a.clone(), last.deck_b.clone())
}
