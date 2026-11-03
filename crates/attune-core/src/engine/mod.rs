//! Real-Time Dual-Deck Native Playback Engine

pub mod deck;
pub mod mixer;
pub mod jit;
pub mod driver;
pub mod external_dsp;

pub use deck::{Deck, DeckId, DeckState};
pub use mixer::{DualDeckMixer, ActiveTransition};
pub use jit::JitPlanner;
pub use driver::{AudioOutputDriver, DriverError};
pub use external_dsp::{ExternalDspStatus, probe_external_dsp};
pub use attune_dsp_ffi::master_fx::{MasterFxParams, MasterFxRack, EQ_BANDS_COUNT, EQ_CENTER_FREQS};

