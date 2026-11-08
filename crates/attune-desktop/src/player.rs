//! Real-time desktop player engine wrapping `DualDeckMixer`, cpal output, and JIT transition planning.
//!
//! Audio decoding happens on a background worker thread; a generation counter
//! discards stale decodes when the user skips quickly between tracks.
//! Seamless DJ transitions between Deck A and Deck B are planned via `JitPlanner`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use attune_core::engine::mixer::MixerSnapshot;
use attune_core::schema::{AnalysisTier, TransitionArchetype};
use attune_core::{
    AudioDecoder, AudioOutputDriver, AudioProfiler, DeckId, DualDeckMixer, JitPlanner,
    MasterFxParams, TransitionRecipe,
};

/// Fallback mixer rate only used when no output device can be queried (headless).
const HEADLESS_FALLBACK_SAMPLE_RATE: u32 = 48_000;

#[derive(Clone, Debug, PartialEq)]
pub struct DriverDiagnostics {
    pub has_hardware_output: bool,
    pub driver_error: Option<String>,
    pub sample_rate: u32,
    pub precision: String,
    pub is_forced_32bit: bool,
    pub device_name: String,
}

impl Default for DriverDiagnostics {
    fn default() -> Self {
        Self {
            has_hardware_output: false,
            driver_error: None,
            sample_rate: 44100,
            precision: "Headless Fallback".into(),
            is_forced_32bit: false,
            device_name: "Default Output".into(),
        }
    }
}

static DRIVER_DIAGNOSTICS: Mutex<Option<DriverDiagnostics>> = Mutex::new(None);
static ACTIVE_PLAYER: Mutex<Option<std::sync::Weak<Player>>> = Mutex::new(None);

pub fn get_driver_diagnostics() -> DriverDiagnostics {
    DRIVER_DIAGNOSTICS.lock().unwrap().clone().unwrap_or_default()
}

pub fn update_driver_diagnostics(diag: DriverDiagnostics) {
    if let Ok(mut guard) = DRIVER_DIAGNOSTICS.lock() {
        *guard = Some(diag);
    }
}

pub fn register_active_player(player: &Arc<Player>) {
    if let Ok(mut guard) = ACTIVE_PLAYER.lock() {
        *guard = Some(Arc::downgrade(player));
    }
}

pub fn reconfigure_active_player(force_32bit: bool) -> Result<(), String> {
    if let Ok(guard) = ACTIVE_PLAYER.lock() {
        if let Some(ref weak) = *guard {
            if let Some(player) = weak.upgrade() {
                return player.reconfigure_output(force_32bit);
            }
        }
    }
    Err("No active player registered".into())
}

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
    pub incoming_bpm: f32,
    pub incoming_key: String,
}

pub struct Player {
    mixer: Arc<Mutex<DualDeckMixer>>,
    // Holds the cpal stream alive; `None` means headless (no output device).
    driver: Arc<Mutex<Option<AudioOutputDriver>>>,
    driver_error: Arc<Mutex<Option<String>>>,
    generation: Arc<AtomicU64>,
    load_state: Arc<Mutex<LoadState>>,
    // User gain in dB stored as f32 bits; re-applied after each load (load resets deck gain).
    gain_db_bits: Arc<AtomicU32>,
    pending_transition: Arc<Mutex<Option<PendingTransitionInfo>>>,
    current_profile: Arc<Mutex<Option<(f32, String)>>>,
    current_track_path: Arc<Mutex<Option<PathBuf>>>,
    active_transition: Arc<Mutex<Option<PendingTransitionInfo>>>,
    force_32bit_float: Arc<AtomicBool>,
}

impl Player {
    pub fn new() -> Self {
        Self::new_with_options(false)
    }

    pub fn new_with_options(force_32bit: bool) -> Self {
        let sample_rate = AudioOutputDriver::default_sample_rate().unwrap_or_else(|e| {
            log::warn!("No default output sample rate ({:?}); using headless fallback", e);
            HEADLESS_FALLBACK_SAMPLE_RATE
        });
        let mixer = Arc::new(Mutex::new(DualDeckMixer::new(sample_rate)));

        // Driver construction can panic inside some cpal backends; fail gracefully.
        let driver_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            AudioOutputDriver::new_with_options(Arc::clone(&mixer), force_32bit)
        }));
        let (driver, driver_error) = match driver_result {
            Ok(Ok(d)) => (Some(d), None),
            Ok(Err(e)) => (None, Some(format!("{:?}", e))),
            Err(_) => (None, Some("audio driver panicked during initialization".into())),
        };
        if let Some(err) = &driver_error {
            log::error!("Audio output unavailable, running headless: {}", err);
        }

        let p = Self {
            mixer,
            driver: Arc::new(Mutex::new(driver)),
            driver_error: Arc::new(Mutex::new(driver_error)),
            generation: Arc::new(AtomicU64::new(0)),
            load_state: Arc::new(Mutex::new(LoadState::Idle)),
            gain_db_bits: Arc::new(AtomicU32::new(0f32.to_bits())),
            pending_transition: Arc::new(Mutex::new(None)),
            current_profile: Arc::new(Mutex::new(None)),
            current_track_path: Arc::new(Mutex::new(None)),
            active_transition: Arc::new(Mutex::new(None)),
            force_32bit_float: Arc::new(AtomicBool::new(force_32bit)),
        };
        p.sync_diagnostics_state();
        p
    }

    pub fn reconfigure_output(&self, force_32bit: bool) -> Result<(), String> {
        self.force_32bit_float.store(force_32bit, Ordering::SeqCst);

        // 1. Drop existing driver to release hardware stream cleanly
        if let Ok(mut guard) = self.driver.lock() {
            *guard = None;
        }

        // 2. Rebuild driver with forced 32-bit output mode
        let mixer_cb = Arc::clone(&self.mixer);
        let driver_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            AudioOutputDriver::new_with_options(mixer_cb, force_32bit)
        }));

        let res = match driver_res {
            Ok(Ok(new_driver)) => {
                log::info!(
                    "AudioOutputDriver reconfigured successfully: format={:?}, forced_32bit={}",
                    new_driver.sample_format(),
                    new_driver.is_forced_32bit()
                );
                if let Ok(mut guard) = self.driver.lock() {
                    *guard = Some(new_driver);
                }
                if let Ok(mut guard) = self.driver_error.lock() {
                    *guard = None;
                }
                Ok(())
            }
            Ok(Err(e)) => {
                let err_str = format!("{:?}", e);
                log::warn!("Audio driver reconfiguration error: {}", err_str);
                if let Ok(mut guard) = self.driver_error.lock() {
                    *guard = Some(err_str.clone());
                }
                Err(err_str)
            }
            Err(_) => {
                let err_str = "Audio driver panicked during reconfiguration".to_string();
                log::error!("{}", err_str);
                if let Ok(mut guard) = self.driver_error.lock() {
                    *guard = Some(err_str.clone());
                }
                Err(err_str)
            }
        };

        self.sync_diagnostics_state();
        res
    }

    pub fn has_hardware_output(&self) -> bool {
        self.driver.lock().map(|d| d.is_some()).unwrap_or(false)
    }

    pub fn driver_error(&self) -> Option<String> {
        self.driver_error.lock().ok().and_then(|e| e.clone())
    }

    pub fn stream_precision(&self) -> String {
        if let Ok(guard) = self.driver.lock() {
            if let Some(ref d) = *guard {
                return d.precision_label();
            }
        }
        if let Ok(guard) = self.driver_error.lock() {
            if let Some(ref err) = *guard {
                return format!("Headless ({err})");
            }
        }
        "Headless Fallback".to_string()
    }

    pub fn sample_rate(&self) -> u32 {
        if let Ok(guard) = self.driver.lock() {
            if let Some(ref d) = *guard {
                return d.sample_rate();
            }
        }
        self.mixer.lock().map(|m| m.sample_rate).unwrap_or(HEADLESS_FALLBACK_SAMPLE_RATE)
    }

    pub fn is_forced_32bit(&self) -> bool {
        self.force_32bit_float.load(Ordering::SeqCst)
    }

    pub fn sync_diagnostics_state(&self) {
        let has_hw = self.has_hardware_output();
        let err = self.driver_error();
        let rate = self.sample_rate();
        let precision = self.stream_precision();
        let is_forced = self.is_forced_32bit();
        let dev_name = if let Ok(guard) = self.driver.lock() {
            guard.as_ref().map(|d| d.device_name().to_string()).unwrap_or_else(|| "Default Audio Device".to_string())
        } else {
            "Default Audio Device".to_string()
        };

        update_driver_diagnostics(DriverDiagnostics {
            has_hardware_output: has_hw,
            driver_error: err,
            sample_rate: rate,
            precision,
            is_forced_32bit: is_forced,
            device_name: dev_name,
        });
    }

    pub fn load_state(&self) -> LoadState {
        self.load_state.lock().map(|s| s.clone()).unwrap_or(LoadState::Idle)
    }

    /// Decodes `path` in the background and starts playback on Deck A once ready.
    pub fn play_file(&self, path: PathBuf) {
        let my_gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.clear_pending_transition();
        if let Ok(mut prof) = self.current_profile.lock() {
            *prof = None;
        }
        if let Ok(mut cur) = self.current_track_path.lock() {
            *cur = Some(path.clone());
        }
        if let Ok(mut act) = self.active_transition.lock() {
            *act = None;
        }

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
        let current_profile = Arc::clone(&self.current_profile);
        thread::Builder::new()
            .name("attune-decode".into())
            .spawn(move || {
                let result = AudioDecoder::load_file(&path);
                if generation.load(Ordering::SeqCst) != my_gen {
                    return; // superseded by a newer request
                }
                match result {
                    Ok(buffer) => {
                        // Extract detected BPM and Key via fast head/tail AudioProfiler
                        let profiler = AudioProfiler::new();
                        let tap = profiler.profile_track(&buffer, &path.to_string_lossy(), AnalysisTier::FastHeadTail);
                        if let Ok(mut prof) = current_profile.lock() {
                            *prof = Some((tap.global_summary.nominal_bpm, tap.global_summary.primary_key));
                        }

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
            if m.is_transitioning() {
                return;
            }
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

                let profiler = AudioProfiler::new();
                let tap_b = profiler.profile_track(&buf_b, &incoming_path.to_string_lossy(), AnalysisTier::FastHeadTail);
                let incoming_bpm = tap_b.global_summary.nominal_bpm;
                let incoming_key = tap_b.global_summary.primary_key;

                if let Ok(mut m) = mixer.lock() {
                    if m.is_transitioning() {
                        return;
                    }
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
                        incoming_bpm,
                        incoming_key,
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
            if let Ok(mut act) = self.active_transition.lock() {
                *act = Some(triggered.clone());
            }
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
        if let Ok(mut act) = self.active_transition.lock() {
            *act = Some(info.clone());
        }
        Some(info)
    }

    /// Polls whether an active transition has officially finished in `DualDeckMixer`.
    ///
    /// When `DualDeckMixer` completes blending, Deck B is swapped into Deck A and
    /// `mixer.is_transitioning()` becomes false. This method atomically consumes
    /// the active transition, updates `current_track_path` to the incoming track,
    /// and returns `Some(completed)` EXACTLY ONCE.
    pub fn poll_transition_completed(&self) -> Option<PendingTransitionInfo> {
        let mut active_guard = self.active_transition.lock().ok()?;
        if active_guard.is_some() {
            let is_trans = self.mixer.lock().ok()?.is_transitioning();
            if !is_trans {
                let completed = active_guard.take();
                if let Some(ref c) = completed {
                    if let Ok(mut cur) = self.current_track_path.lock() {
                        *cur = Some(c.incoming_path.clone());
                    }
                }
                return completed;
            }
        }
        None
    }

    /// Returns true if `DualDeckMixer` is currently in the middle of a transition blend.
    pub fn is_transitioning(&self) -> bool {
        self.mixer.lock().map(|m| m.is_transitioning()).unwrap_or(false)
    }

    /// Returns the path of the track currently assigned to Deck A.
    pub fn current_track_path(&self) -> Option<PathBuf> {
        self.current_track_path.lock().ok().and_then(|p| p.clone())
    }

    pub fn pending_transition(&self) -> Option<PendingTransitionInfo> {
        self.pending_transition.lock().ok().and_then(|p| p.clone())
    }

    pub fn current_profile(&self) -> Option<(f32, String)> {
        self.current_profile.lock().ok().and_then(|p| p.clone())
    }

    pub fn set_current_profile(&self, bpm: f32, key: String) {
        if let Ok(mut prof) = self.current_profile.lock() {
            *prof = Some((bpm, key));
        }
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

    pub fn stop(&self) {
        self.clear_pending_transition();
        if let Ok(mut m) = self.mixer.lock() {
            m.stop();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_transition_lifecycle_and_poll_completed() {
        let player = Player::new();
        assert_eq!(player.current_track_path(), None);
        assert!(!player.is_transitioning());
        assert!(player.poll_transition_completed().is_none());

        let path_a = PathBuf::from("/music/song_a.flac");
        let path_b = PathBuf::from("/music/song_b.flac");

        if let Ok(mut cur) = player.current_track_path.lock() {
            *cur = Some(path_a.clone());
        }
        assert_eq!(player.current_track_path(), Some(path_a.clone()));

        // Simulate priming active transition
        let dummy_recipe = attune_core::TransitionRecipe {
            schema_version: 1,
            track_a_hash: "dummy_a".into(),
            track_b_hash: "dummy_b".into(),
            archetype: TransitionArchetype::EqSwap,
            confidence_score: 0.94,
            timing: attune_core::schema::TransitionTiming {
                track_a_transition_start_ms: 1000,
                track_a_exit_end_ms: 3000,
                track_b_entry_start_ms: 0,
                track_b_native_lock_ms: 2000,
            },
            optional_stem_bridge_uri: None,
            dsp_automation_keyframes: Vec::new(),
        };

        let info = PendingTransitionInfo {
            incoming_path: path_b.clone(),
            incoming_title: "Song B".into(),
            incoming_artist: "Artist B".into(),
            recipe: dummy_recipe,
            archetype_name: "3 EQ Swap",
            duration_sec: 2.0,
            confidence_pct: 94,
            scheduled_start_ms: 1000,
            incoming_bpm: 120.0,
            incoming_key: "8B".into(),
        };

        // When transition is active in mixer, poll_transition_completed MUST return None
        if let Ok(mut act) = player.active_transition.lock() {
            *act = Some(info.clone());
        }
        if let Ok(mut m) = player.mixer.lock() {
            m.transition = Some(attune_core::engine::ActiveTransition {
                recipe: info.recipe.clone(),
                elapsed_frames: 0,
                total_transition_frames: 48000,
            });
        }
        assert!(player.is_transitioning());
        assert!(player.poll_transition_completed().is_none());

        // When mixer finishes transition, poll_transition_completed MUST return Some(info) EXACTLY ONCE
        if let Ok(mut m) = player.mixer.lock() {
            m.transition = None;
        }
        assert!(!player.is_transitioning());

        let completed = player.poll_transition_completed();
        assert!(completed.is_some());
        let c = completed.unwrap();
        assert_eq!(c.incoming_path, path_b);
        assert_eq!(player.current_track_path(), Some(path_b));

        // Subsequent poll MUST return None (atomic one-shot consumption)
        assert!(player.poll_transition_completed().is_none());
    }
}
