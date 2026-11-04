//! Real-time desktop player engine wrapping `DualDeckMixer`, cpal output, and JIT transition planning.
//!
//! Audio decoding happens on a background worker thread; a generation counter
//! discards stale decodes when the user skips quickly between tracks.
//! Seamless DJ transitions between Deck A and Deck B are planned via `JitPlanner`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use attune_core::engine::mixer::MixerSnapshot;
use attune_core::schema::TransitionArchetype;
use attune_core::{
    AudioDecoder, AudioOutputDriver, DeckId, DualDeckMixer, JitPlanner, MasterFxParams,
    TransitionRecipe,
};

/// Fallback mixer rate only used when no output device can be queried (headless).
const HEADLESS_FALLBACK_SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Clone, PartialEq)]
pub enum LoadState {
    Idle,
    Decoding(PathBuf),
    Ready,
    Failed(String),
}

/// Metadata summary of a pending transition recipe primed on Deck B.
#[derive(Debug, Clone)]
pub struct PendingTransitionInfo {
    #[allow(dead_code)]
    pub incoming_path: PathBuf,
    pub incoming_title: String,
    pub incoming_artist: String,
    pub recipe: TransitionRecipe,
    pub archetype_name: &'static str,
    pub duration_sec: f32,
    pub confidence_pct: u32,
    pub scheduled_start_ms: u64,
}

pub struct Player {
    mixer: Arc<Mutex<DualDeckMixer>>,
    // Holds the cpal stream alive; `None` means headless (no output device).
    driver: Option<AudioOutputDriver>,
    driver_error: Option<String>,
    generation: Arc<AtomicU64>,
    load_state: Arc<Mutex<LoadState>>,
    // User gain in dB stored as f32 bits; re-applied after each load (load resets deck gain).
    gain_db_bits: Arc<AtomicU32>,
    pending_transition: Arc<Mutex<Option<PendingTransitionInfo>>>,
}

impl Player {
    pub fn new() -> Self {
        let sample_rate = AudioOutputDriver::default_sample_rate().unwrap_or_else(|e| {
            log::warn!("No default output sample rate ({:?}); using headless fallback", e);
            HEADLESS_FALLBACK_SAMPLE_RATE
        });
        let mixer = Arc::new(Mutex::new(DualDeckMixer::new(sample_rate)));

        // Driver construction can panic inside some cpal backends; fail gracefully.
        let driver_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            AudioOutputDriver::new(Arc::clone(&mixer))
        }));
        let (driver, driver_error) = match driver_result {
            Ok(Ok(d)) => (Some(d), None),
            Ok(Err(e)) => (None, Some(format!("{:?}", e))),
            Err(_) => (None, Some("audio driver panicked during initialization".into())),
        };
        if let Some(err) = &driver_error {
            log::error!("Audio output unavailable, running headless: {}", err);
        }

        Self {
            mixer,
            driver,
            driver_error,
            generation: Arc::new(AtomicU64::new(0)),
            load_state: Arc::new(Mutex::new(LoadState::Idle)),
            gain_db_bits: Arc::new(AtomicU32::new(0f32.to_bits())),
            pending_transition: Arc::new(Mutex::new(None)),
        }
    }

    pub fn has_hardware_output(&self) -> bool {
        self.driver.is_some()
    }

    pub fn driver_error(&self) -> Option<&str> {
        self.driver_error.as_deref()
    }

    pub fn load_state(&self) -> LoadState {
        self.load_state.lock().map(|s| s.clone()).unwrap_or(LoadState::Idle)
    }

    /// Decodes `path` in the background and starts playback on Deck A once ready.
    pub fn play_file(&self, path: PathBuf) {
        let my_gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.clear_pending_transition();

        if let Ok(mut m) = self.mixer.lock() {
            m.transition = None;
            m.deck_a.clear();
            m.deck_b.clear();
            m.active_deck = DeckId::DeckA;
            m.should_be_playing = true;
        }
        if let Ok(mut s) = self.load_state.lock() {
            *s = LoadState::Decoding(path.clone());
        }

        let mixer = Arc::clone(&self.mixer);
        let generation = Arc::clone(&self.generation);
        let load_state = Arc::clone(&self.load_state);
        let gain_bits = Arc::clone(&self.gain_db_bits);
        thread::Builder::new()
            .name("attune-decode".into())
            .spawn(move || {
                let result = AudioDecoder::load_file(&path);
                if generation.load(Ordering::SeqCst) != my_gen {
                    return; // superseded by a newer request
                }
                match result {
                    Ok(buffer) => {
                        if let Ok(mut m) = mixer.lock() {
                            m.load_track_a(buffer);
                            m.deck_a.set_gain_db(f32::from_bits(gain_bits.load(Ordering::Relaxed)));
                            m.play();
                        }
                        if let Ok(mut s) = load_state.lock() {
                            *s = LoadState::Ready;
                        }
                    }
                    Err(e) => {
                        log::warn!("Decode failed for {}: {}", path.display(), e);
                        if let Ok(mut s) = load_state.lock() {
                            *s = LoadState::Failed(e.to_string());
                        }
                    }
                }
            })
            .expect("failed to spawn decode thread");
    }

    /// Preloads the incoming track into Deck B and plans a JIT transition recipe in the background.
    pub fn prepare_transition(
        &self,
        incoming_path: PathBuf,
        incoming_title: String,
        incoming_artist: String,
    ) {
        let buf_a_clone = {
            let Ok(m) = self.mixer.lock() else { return };
            m.deck_a.audio().cloned()
        };
        let Some(buf_a) = buf_a_clone else { return };

        let mixer = Arc::clone(&self.mixer);
        let pending = Arc::clone(&self.pending_transition);

        thread::Builder::new()
            .name("attune-jit-planner".into())
            .spawn(move || {
                let Ok(buf_b) = AudioDecoder::load_file(&incoming_path) else {
                    log::warn!("Could not load incoming track for transition: {}", incoming_path.display());
                    return;
                };

                let planner = JitPlanner::new();
                let recipe = planner.plan_jit(&buf_a, "outgoing", &buf_b, "incoming");

                let archetype_name = match recipe.archetype {
                    TransitionArchetype::SustainBridge => "1 Sustain Bridge",
                    TransitionArchetype::RiffHandoff => "2 Riff Handoff",
                    TransitionArchetype::EqSwap => "3 EQ Swap",
                    TransitionArchetype::TailFreeze => "4 Tail Freeze",
                    TransitionArchetype::ColdCut => "5 Cold Cut",
                    TransitionArchetype::GenerativeMelodicFill => "6 Melodic Fill",
                };

                let duration_sec = (recipe
                    .timing
                    .track_a_exit_end_ms
                    .saturating_sub(recipe.timing.track_a_transition_start_ms) as f32
                    / 1000.0)
                    .max(0.5);
                let confidence_pct = (recipe.confidence_score * 100.0).clamp(10.0, 99.0) as u32;
                let scheduled_start_ms = recipe.timing.track_a_transition_start_ms;

                if let Ok(mut m) = mixer.lock() {
                    m.load_track_b(buf_b);
                }

                if let Ok(mut p) = pending.lock() {
                    *p = Some(PendingTransitionInfo {
                        incoming_path,
                        incoming_title,
                        incoming_artist,
                        recipe,
                        archetype_name,
                        duration_sec,
                        confidence_pct,
                        scheduled_start_ms,
                    });
                }
            })
            .expect("failed to spawn JIT planner thread");
    }

    /// Checks if the playing track has reached the scheduled transition point.
    /// If reached, triggers the transition in `DualDeckMixer` and returns the recipe.
    pub fn check_auto_transition(&self) -> Option<PendingTransitionInfo> {
        let mut pending_guard = self.pending_transition.lock().ok()?;
        let info = pending_guard.as_ref()?;

        let mut mixer = self.mixer.lock().ok()?;
        if mixer.is_transitioning() {
            return None;
        }

        let pos = mixer.deck_a.current_position_ms();
        let audible_end = mixer.deck_a.audible_end_ms;
        let is_scheduled = pos >= info.scheduled_start_ms;
        let is_safety_end = audible_end > 3000 && pos >= audible_end.saturating_sub(2500);

        if is_scheduled || is_safety_end {
            mixer.trigger_transition(info.recipe.clone());
            let triggered = info.clone();
            *pending_guard = None;
            Some(triggered)
        } else {
            None
        }
    }

    /// Manually triggers the primed transition immediately.
    pub fn trigger_transition_now(&self) -> Option<PendingTransitionInfo> {
        let mut pending_guard = self.pending_transition.lock().ok()?;
        let info = pending_guard.take()?;

        let mut mixer = self.mixer.lock().ok()?;
        mixer.trigger_transition(info.recipe.clone());
        Some(info)
    }

    pub fn pending_transition(&self) -> Option<PendingTransitionInfo> {
        self.pending_transition.lock().ok().and_then(|p| p.clone())
    }

    pub fn clear_pending_transition(&self) {
        if let Ok(mut p) = self.pending_transition.lock() {
            *p = None;
        }
        if let Ok(mut m) = self.mixer.lock() {
            if !m.is_transitioning() {
                m.deck_b.clear();
            }
        }
    }

    pub fn set_master_fx_params(&self, params: MasterFxParams) {
        if let Ok(mut m) = self.mixer.lock() {
            m.master_fx.set_params(params);
        }
    }

    pub fn play(&self) {
        if let Ok(mut m) = self.mixer.lock() {
            m.play();
        }
    }

    pub fn pause(&self) {
        if let Ok(mut m) = self.mixer.lock() {
            m.pause();
        }
    }

    pub fn seek_ms(&self, ms: u64) {
        if let Ok(mut m) = self.mixer.lock() {
            match m.active_deck {
                DeckId::DeckA => m.deck_a.seek_ms(ms),
                DeckId::DeckB => m.deck_b.seek_ms(ms),
            }
        }
    }

    pub fn set_gain_db(&self, db: f32) {
        self.gain_db_bits.store(db.to_bits(), Ordering::Relaxed);
        if let Ok(mut m) = self.mixer.lock() {
            m.deck_a.set_gain_db(db);
            m.deck_b.set_gain_db(db);
        }
    }

    /// Sub-microsecond snapshot; never holds the lock while the UI renders.
    pub fn snapshot(&self) -> Option<MixerSnapshot> {
        self.mixer.lock().ok().map(|m| m.snapshot())
    }
}
