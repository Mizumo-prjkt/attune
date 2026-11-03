//! Thin desktop player wrapper around the real-time `DualDeckMixer` + cpal driver.
//!
//! Audio decoding happens on a worker thread; a generation counter discards
//! stale decodes when the user skips quickly between tracks.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use attune_core::engine::mixer::MixerSnapshot;
use attune_core::{AudioDecoder, AudioOutputDriver, DeckId, DualDeckMixer};

/// Fallback mixer rate only used when no output device can be queried (headless).
const HEADLESS_FALLBACK_SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Clone, PartialEq)]
pub enum LoadState {
    Idle,
    Decoding(PathBuf),
    Ready,
    Failed(String),
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
    pub fn play_file(&self, path: PathBuf, ctx: eframe::egui::Context) {
        let my_gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
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
                ctx.request_repaint();
            })
            .expect("failed to spawn decode thread");
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
