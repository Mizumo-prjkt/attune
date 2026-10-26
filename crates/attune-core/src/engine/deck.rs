//! Deck component representing an individual playback deck (Deck A or Deck B)

use attune_dsp_ffi::eq::ThreeBandEq;
use attune_dsp_ffi::fader::db_to_linear;
use attune_dsp_ffi::pitch_shift::interpolate_hermite;
use attune_dsp_ffi::reverb_freeze::ReverbFreeze;
use crate::audio::AudioBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckId {
    DeckA,
    DeckB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckState {
    Stopped,
    Playing,
    Paused,
    Transitioning,
    Finished,
}

#[derive(Debug, Clone)]
pub struct Deck {
    pub deck_id: DeckId,
    pub state: DeckState,
    sample_rate: u32,
    audio: Option<AudioBuffer>,
    playhead_frame: f64,
    // DSP State
    gain_db: f32,
    gain_linear: f32,
    stretch_ratio: f32,
    pitch_cents: i32,
    reverb_freeze_send: f32,
    eq: ThreeBandEq,
    reverb_freeze: ReverbFreeze,
}

impl Deck {
    pub fn new(deck_id: DeckId, sample_rate: u32) -> Self {
        Self {
            deck_id,
            state: DeckState::Stopped,
            sample_rate,
            audio: None,
            playhead_frame: 0.0,
            gain_db: 0.0,
            gain_linear: 1.0,
            stretch_ratio: 1.0,
            pitch_cents: 0,
            reverb_freeze_send: 0.0,
            eq: ThreeBandEq::new(sample_rate as f32),
            reverb_freeze: ReverbFreeze::new(sample_rate as f32),
        }
    }

    pub fn load_audio(&mut self, audio: AudioBuffer) {
        self.audio = Some(audio);
        self.playhead_frame = 0.0;
        self.state = DeckState::Stopped;
        self.eq.reset_state();
        self.reverb_freeze.reset();
    }

    pub fn play(&mut self) {
        if self.audio.is_some() {
            self.state = DeckState::Playing;
        }
    }

    pub fn pause(&mut self) {
        if self.state == DeckState::Playing {
            self.state = DeckState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.state = DeckState::Stopped;
        self.playhead_frame = 0.0;
        self.eq.reset_state();
        self.reverb_freeze.reset();
    }

    pub fn seek_ms(&mut self, ms: u64) {
        let frame = (ms as f64 / 1000.0) * self.sample_rate as f64;
        let total = self.num_frames() as f64;
        self.playhead_frame = frame.clamp(0.0, total);
    }

    pub fn current_position_ms(&self) -> u64 {
        ((self.playhead_frame / self.sample_rate as f64) * 1000.0) as u64
    }

    pub fn duration_ms(&self) -> u64 {
        self.audio.as_ref().map(|a| a.duration_ms()).unwrap_or(0)
    }

    pub fn num_frames(&self) -> usize {
        self.audio.as_ref().map(|a| a.num_frames()).unwrap_or(0)
    }

    pub fn is_playing(&self) -> bool {
        self.state == DeckState::Playing || self.state == DeckState::Transitioning
    }

    // DSP Setters
    pub fn set_gain_db(&mut self, db: f32) {
        self.gain_db = db;
        self.gain_linear = db_to_linear(db);
    }

    pub fn set_eq_sub_db(&mut self, db: f32) {
        self.eq.set_low_db(db);
    }

    pub fn set_eq_mid_db(&mut self, db: f32) {
        self.eq.set_mid_db(db);
    }

    pub fn set_eq_high_db(&mut self, db: f32) {
        self.eq.set_high_db(db);
    }

    pub fn set_stretch_ratio(&mut self, ratio: f32) {
        self.stretch_ratio = ratio.clamp(0.5, 2.0);
    }

    pub fn set_pitch_cents(&mut self, cents: i32) {
        self.pitch_cents = cents;
    }

    pub fn set_reverb_freeze_send(&mut self, send: f32) {
        self.reverb_freeze_send = send.clamp(0.0, 1.0);
        let active = send > 0.05;
        self.reverb_freeze.set_freeze(active, send);
    }

    /// Read next stereo audio frame with sample interpolation, EQ, gain, and freeze wash
    #[inline]
    pub fn read_stereo_frame(&mut self) -> (f32, f32) {
        if !self.is_playing() {
            // Still process lingering freeze tail if active
            if self.reverb_freeze_send > 0.01 {
                return self.reverb_freeze.process_stereo(0.0, 0.0);
            }
            return (0.0, 0.0);
        }

        let audio = match self.audio.as_ref() {
            Some(a) => a,
            None => return (0.0, 0.0),
        };

        let total_frames = audio.num_frames();
        if self.playhead_frame >= total_frames as f64 {
            self.state = DeckState::Finished;
            if self.reverb_freeze_send > 0.01 {
                return self.reverb_freeze.process_stereo(0.0, 0.0);
            }
            return (0.0, 0.0);
        }

        let base_idx = self.playhead_frame.floor() as usize;
        let frac = (self.playhead_frame - base_idx as f64) as f32;

        let get_sample = |ch_idx: usize, idx: isize| -> f32 {
            if idx < 0 {
                return 0.0;
            }
            let u = idx as usize;
            audio.channels.get(ch_idx).and_then(|c| c.get(u)).copied().unwrap_or(0.0)
        };

        let raw_l = if frac.abs() < 1e-4 {
            get_sample(0, base_idx as isize)
        } else {
            let y0 = get_sample(0, base_idx as isize - 1);
            let y1 = get_sample(0, base_idx as isize);
            let y2 = get_sample(0, base_idx as isize + 1);
            let y3 = get_sample(0, base_idx as isize + 2);
            interpolate_hermite(y0, y1, y2, y3, frac)
        };

        let raw_r = if frac.abs() < 1e-4 {
            let ch1 = if audio.num_channels() > 1 { 1 } else { 0 };
            get_sample(ch1, base_idx as isize)
        } else {
            let ch1 = if audio.num_channels() > 1 { 1 } else { 0 };
            let y0 = get_sample(ch1, base_idx as isize - 1);
            let y1 = get_sample(ch1, base_idx as isize);
            let y2 = get_sample(ch1, base_idx as isize + 1);
            let y3 = get_sample(ch1, base_idx as isize + 2);
            interpolate_hermite(y0, y1, y2, y3, frac)
        };

        // Advance playhead by stretch_ratio
        self.playhead_frame += self.stretch_ratio as f64;

        // Apply 3-band Isolator EQ
        let (eq_l, eq_r) = self.eq.process_stereo(raw_l, raw_r);

        // Apply gain fader
        let dry_l = eq_l * self.gain_linear;
        let dry_r = eq_r * self.gain_linear;

        // Process Reverb Freeze Wash if send > 0
        if self.reverb_freeze_send > 0.01 {
            let (wet_l, wet_r) = self.reverb_freeze.process_stereo(raw_l, raw_r);
            (dry_l + wet_l, dry_r + wet_r)
        } else {
            (dry_l, dry_r)
        }
    }
}
