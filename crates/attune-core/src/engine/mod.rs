//! Real-Time Dual-Deck Native Playback Engine

pub mod deck;
pub mod mixer;
pub mod jit;
pub mod driver;

pub use deck::{Deck, DeckId, DeckState};
pub use mixer::{DualDeckMixer, ActiveTransition};
pub use jit::JitPlanner;
pub use driver::{AudioOutputDriver, DriverError};
