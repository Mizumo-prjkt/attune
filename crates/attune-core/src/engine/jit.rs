//! On-Demand JIT Mode 1 Fast-Pass Transition Planner
//!
//! Analyzes only the Cue Zones (first 45s of Track B and last 45s of Track A)
//! in <150 ms to plan transitions on-the-fly for newly queued tracks, with
//! transparent SQLite cache lookup & persistence.

use crate::audio::AudioBuffer;
use crate::cache::CacheDatabase;
use crate::planner::TransitionPlanner;
use crate::profiler::AudioProfiler;
use crate::schema::{AnalysisTier, TransitionRecipe};

pub struct JitPlanner {
    profiler: AudioProfiler,
    planner: TransitionPlanner,
}

impl Default for JitPlanner {
    fn default() -> Self {
        Self::new()
    }
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

    /// Fast-pass planning with SQLite cache lookup and persistence.
    /// Returns cached recipe if available in microseconds, else runs JIT planning and caches the result.
    pub fn plan_cached_or_jit(
        &self,
        buf_a: &AudioBuffer,
        slug_a: &str,
        buf_b: &AudioBuffer,
        slug_b: &str,
        db: Option<&CacheDatabase>,
    ) -> TransitionRecipe {
        let hash_a = buf_a.compute_hash();
        let hash_b = buf_b.compute_hash();

        if let Some(db) = db {
            if let Ok(Some(cached_ptr)) = db.get_ptr(&hash_a, &hash_b) {
                return cached_ptr;
            }
        }

        // Retrieve or compute Track A profile
        let prof_a = if let Some(db) = db {
            if let Ok(Some(existing_a)) = db.get_tap(&hash_a) {
                existing_a
            } else {
                let p = self.profiler.profile_track(buf_a, slug_a, AnalysisTier::FastHeadTail);
                let _ = db.store_tap(&p);
                p
            }
        } else {
            self.profiler.profile_track(buf_a, slug_a, AnalysisTier::FastHeadTail)
        };

        // Retrieve or compute Track B profile
        let prof_b = if let Some(db) = db {
            if let Ok(Some(existing_b)) = db.get_tap(&hash_b) {
                existing_b
            } else {
                let p = self.profiler.profile_track(buf_b, slug_b, AnalysisTier::FastHeadTail);
                let _ = db.store_tap(&p);
                p
            }
        } else {
            self.profiler.profile_track(buf_b, slug_b, AnalysisTier::FastHeadTail)
        };

        let recipe = self.planner.plan_transition(&prof_a, &prof_b);

        if let Some(db) = db {
            let _ = db.store_ptr(&recipe);
        }

        recipe
    }
}
