//! Attune Core: Audio profiling, 5-layer feature extraction, and transition planning.

pub mod audio;
pub mod schema;
pub mod profiler;
pub mod planner;
pub mod renderer;
pub mod engine;
pub mod cache;
pub mod generative;

pub use audio::{AudioBuffer, AudioDecoder, AudioError, TrackMetadataProbe};
pub use schema::{EndingType, TrackAnalysisProfile, TransitionRecipe};
pub use profiler::AudioProfiler;
pub use planner::TransitionPlanner;
pub use renderer::TransitionRenderer;
pub use engine::{DualDeckMixer, Deck, DeckId, DeckState, JitPlanner, AudioOutputDriver, ExternalDspStatus, probe_external_dsp};
pub use cache::{CacheDatabase, CacheError, CachePack, CacheStats, LibraryScanner, StemBridgeRecord};
pub use generative::{GenerativeFillEngine, GenerativeFillConfig, MelodicFillStyle, NoteEvent};

