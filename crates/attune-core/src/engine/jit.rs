//! On-Demand JIT Mode 1 Fast-Pass Transition Planner
//!
//! Analyzes only the Cue Zones (first 45s of Track B and last 45s of Track A)
//! in <150 ms to plan transitions on-the-fly for newly queued tracks.

use crate::audio::AudioBuffer;
use crate::planner::TransitionPlanner;
use crate::profiler::AudioProfiler;
use crate::schema::{AnalysisTier, TransitionRecipe};

pub struct JitPlanner {
    profiler: AudioProfiler,
    planner: TransitionPlanner,
}

impl JitPlanner {
    pub fn new() -> Self {
        Self {
            profiler: AudioProfiler::new(),
            planner: TransitionPlanner::new(),
        }
    }

    /// Fast-pass transition planning between two audio buffers (<150 ms)
    pub fn plan_jit(
        &self,
        buf_a: &AudioBuffer,
        slug_a: &str,
        buf_b: &AudioBuffer,
        slug_b: &str,
    ) -> TransitionRecipe {
        let prof_a = self.profiler.profile_track(buf_a, slug_a, AnalysisTier::FastHeadTail);
        let prof_b = self.profiler.profile_track(buf_b, slug_b, AnalysisTier::FastHeadTail);

        self.planner.plan_transition(&prof_a, &prof_b)
    }
}
