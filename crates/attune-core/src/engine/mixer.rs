//! Dual-Deck Playback Mixer with sample-accurate transition automation

use super::deck::{Deck, DeckId, DeckState};
use crate::audio::AudioBuffer;
use crate::schema::{DeckAutomation, DspAutomationKeyframe, TransitionRecipe};
use attune_dsp_ffi::master_fx::MasterFxRack;

#[derive(Debug, Clone)]
pub struct ActiveTransition {
    pub recipe: TransitionRecipe,
    pub elapsed_frames: usize,
    pub total_transition_frames: usize,
}

#[derive(Debug, Clone)]
pub struct DualDeckMixer {
    pub sample_rate: u32,
    pub deck_a: Deck,
    pub deck_b: Deck,
    pub active_deck: DeckId,
    pub should_be_playing: bool,
    pub transition: Option<ActiveTransition>,
    pub master_fx: MasterFxRack,
    master_gain_linear: f32,
}

impl DualDeckMixer {
    pub fn new(sample_rate: u32) -> Self {
        let sr = if sample_rate > 0 { sample_rate } else { 44100 };
        Self {
            sample_rate: sr,
            deck_a: Deck::new(DeckId::DeckA, sr),
            deck_b: Deck::new(DeckId::DeckB, sr),
            active_deck: DeckId::DeckA,
            should_be_playing: false,
            transition: None,
            master_fx: MasterFxRack::new(sr as f32),
            master_gain_linear: 1.0,
        }
    }

    pub fn set_output_sample_rate(&mut self, sample_rate: u32) {
        if sample_rate > 0 && sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
            self.deck_a.set_output_sample_rate(sample_rate);
            self.deck_b.set_output_sample_rate(sample_rate);
            self.master_fx.set_sample_rate(sample_rate as f32);
        }
    }

    pub fn load_track_a(&mut self, audio: AudioBuffer) {
        self.transition = None;
        self.active_deck = DeckId::DeckA;
        self.deck_a.load_audio(audio);
        self.deck_a.set_gain_db(0.0);
        self.deck_a.set_eq_sub_db(0.0);
        self.deck_a.set_eq_mid_db(0.0);
        self.deck_a.set_eq_high_db(0.0);
        self.deck_a.set_stretch_ratio(1.0);
        self.deck_a.set_pitch_cents(0);
        self.deck_a.set_reverb_freeze_send(0.0);
        if self.should_be_playing {
            self.deck_a.play();
        }
    }

    pub fn load_track_b(&mut self, audio: AudioBuffer) {
        self.deck_b.load_audio(audio);
        self.deck_b.set_gain_db(0.0);
        self.deck_b.set_eq_sub_db(0.0);
        self.deck_b.set_eq_mid_db(0.0);
        self.deck_b.set_eq_high_db(0.0);
        self.deck_b.set_stretch_ratio(1.0);
        self.deck_b.set_pitch_cents(0);
        self.deck_b.set_reverb_freeze_send(0.0);
    }

    pub fn play(&mut self) {
        self.should_be_playing = true;
        match self.active_deck {
            DeckId::DeckA => self.deck_a.play(),
            DeckId::DeckB => self.deck_b.play(),
        }
    }

    pub fn pause(&mut self) {
        self.should_be_playing = false;
        self.deck_a.pause();
        self.deck_b.pause();
    }

    pub fn is_playing(&self) -> bool {
        self.deck_a.is_playing() || self.deck_b.is_playing() || self.transition.is_some()
    }

    pub fn is_transitioning(&self) -> bool {
        self.transition.is_some()
    }

    pub fn current_position_ms(&self) -> u64 {
        match self.active_deck {
            DeckId::DeckA => self.deck_a.current_position_ms(),
            DeckId::DeckB => self.deck_b.current_position_ms(),
        }
    }

    /// Trigger a transition between outgoing Deck A and incoming Deck B using a .ptr recipe
    pub fn trigger_transition(&mut self, recipe: TransitionRecipe) {
        let duration_ms = recipe
            .timing
            .track_a_exit_end_ms
            .saturating_sub(recipe.timing.track_a_transition_start_ms)
            .max(500);

        let total_frames = ((duration_ms as f64 / 1000.0) * self.sample_rate as f64) as usize;

        // Position incoming deck at its entry point
        let b_entry_ms = recipe.timing.track_b_entry_start_ms;
        self.deck_b.seek_ms(b_entry_ms);
        self.deck_b.play();

        // Mark both decks as transitioning
        self.deck_a.state = DeckState::Transitioning;
        self.deck_b.state = DeckState::Transitioning;

        self.transition = Some(ActiveTransition {
            recipe,
            elapsed_frames: 0,
            total_transition_frames: total_frames,
        });
    }

    /// Check if the outgoing track has reached the scheduled transition start time and trigger automatically
    pub fn check_auto_transition(&mut self, recipe: &TransitionRecipe) {
        if self.transition.is_some() {
            return;
        }

        let current_ms = self.deck_a.current_position_ms();
        let start_ms = recipe.timing.track_a_transition_start_ms;

        if current_ms >= start_ms && self.deck_a.is_playing() {
            self.trigger_transition(recipe.clone());
        }
    }

    /// Process next stereo frame (left, right) with sample-accurate DSP automation and soft limiter
    #[inline]
    pub fn process_stereo_frame(&mut self) -> (f32, f32) {
        let (raw_l, raw_r) = if let Some(ref mut trans) = self.transition {
            let offset_ms = ((trans.elapsed_frames as f64 / self.sample_rate as f64) * 1000.0) as u64;
            let (auto_a, auto_b) = interpolate_keyframes(&trans.recipe.dsp_automation_keyframes, offset_ms);

            // Apply automation to Deck A
            self.deck_a.set_gain_db(auto_a.gain_db);
            self.deck_a.set_eq_sub_db(auto_a.eq_sub_db);
            self.deck_a.set_eq_mid_db(auto_a.eq_mid_db);
            self.deck_a.set_eq_high_db(auto_a.eq_high_db);
            self.deck_a.set_stretch_ratio(auto_a.stretch_ratio);
            self.deck_a.set_pitch_cents(auto_a.pitch_cents);
            self.deck_a.set_reverb_freeze_send(auto_a.reverb_freeze_send);

            // Apply automation to Deck B
            self.deck_b.set_gain_db(auto_b.gain_db);
            self.deck_b.set_eq_sub_db(auto_b.eq_sub_db);
            self.deck_b.set_eq_mid_db(auto_b.eq_mid_db);
            self.deck_b.set_eq_high_db(auto_b.eq_high_db);
            self.deck_b.set_stretch_ratio(auto_b.stretch_ratio);
            self.deck_b.set_pitch_cents(auto_b.pitch_cents);
            self.deck_b.set_reverb_freeze_send(auto_b.reverb_freeze_send);

            // Read both decks
            let (a_l, a_r) = self.deck_a.read_stereo_frame();
            let (b_l, b_r) = self.deck_b.read_stereo_frame();

            trans.elapsed_frames += 1;

            if trans.elapsed_frames >= trans.total_transition_frames {
                // Transition finished: Deck B is now our playing track!
                // Swap Deck B into Deck A so Deck A is consistently the active deck!
                std::mem::swap(&mut self.deck_a, &mut self.deck_b);
                self.deck_a.deck_id = DeckId::DeckA;
                self.deck_b.deck_id = DeckId::DeckB;
                self.deck_a.state = DeckState::Playing;
                self.deck_a.set_gain_db(0.0);
                self.deck_a.set_eq_sub_db(0.0);
                self.deck_a.set_eq_mid_db(0.0);
                self.deck_a.set_eq_high_db(0.0);
                self.deck_a.set_stretch_ratio(1.0);
                self.deck_a.set_pitch_cents(0);
                self.deck_a.set_reverb_freeze_send(0.0);

                // Deck B is cleared and primed for the next upcoming transition
                self.deck_b.clear();
                self.active_deck = DeckId::DeckA;
                self.transition = None;
            }

            (a_l + b_l, a_r + b_r)
        } else {
            match self.active_deck {
                DeckId::DeckA => self.deck_a.read_stereo_frame(),
                DeckId::DeckB => {
                    let (b_l, b_r) = self.deck_b.read_stereo_frame();
                    let (a_l, a_r) = self.deck_a.read_stereo_frame();
                    (b_l + a_l, b_r + a_r)
                }
            }
        };

        // Master FX Rack mastering chain (pre-amp, 10-band EQ, ViPER Bass & Clarity, stereo widen, brickwall limiter)
        let (fx_l, fx_r) = self.master_fx.process_stereo_frame((raw_l * self.master_gain_linear, raw_r * self.master_gain_linear));

        (fx_l, fx_r)
    }

    /// Process a block of interleaved stereo samples `[L, R, L, R, ...]` (real-time audio callback safe)
    pub fn process_interleaved_block(&mut self, output: &mut [f32]) {
        for frame in output.chunks_exact_mut(2) {
            let (l, r) = self.process_stereo_frame();
            frame[0] = l;
            frame[1] = r;
        }
    }
}

fn interpolate_keyframes(
    keyframes: &[DspAutomationKeyframe],
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
