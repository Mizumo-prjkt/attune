//! Attune Core: Audio profiling, 5-layer feature extraction, and transition planning.

pub mod audio;
pub mod schema;
pub mod profiler;
pub mod planner;
pub mod renderer;

pub use schema::{TrackAnalysisProfile, TransitionRecipe};
pub use profiler::AudioProfiler;
pub use planner::TransitionPlanner;
pub use renderer::TransitionRenderer;
