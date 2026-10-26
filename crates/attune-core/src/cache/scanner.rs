use super::db::{CacheDatabase, CacheError};
use crate::audio::AudioDecoder;
use crate::planner::TransitionPlanner;
use crate::profiler::AudioProfiler;
use crate::schema::{AnalysisTier, TrackAnalysisProfile};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ScanReport {
    pub total_audio_files: usize,
    pub newly_profiled_tracks: usize,
    pub precomputed_transitions: usize,
    pub errors: Vec<String>,
}

pub struct LibraryScanner {
    db: CacheDatabase,
    profiler: AudioProfiler,
    planner: TransitionPlanner,
}

impl LibraryScanner {
    pub fn new(db: CacheDatabase) -> Self {
        Self {
            db,
            profiler: AudioProfiler::new(),
            planner: TransitionPlanner::new(),
        }
    }

    /// Recursively discovers all supported audio files in the specified directory.
    pub fn discover_audio_files(dir: impl AsRef<Path>) -> Vec<PathBuf> {
        let mut results = Vec::new();
        Self::walk_dir(dir.as_ref(), &mut results);
        results.sort();
        results
    }

    fn walk_dir(dir: &Path, results: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    Self::walk_dir(&p, results);
                } else if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                        let lower = ext.to_lowercase();
                        if matches!(
                            lower.as_str(),
                            "flac" | "mp3" | "wav" | "ogg" | "m4a" | "aac" | "aiff" | "aif"
                        ) {
                            results.push(p);
                        }
                    }
                }
            }
        }
    }

    /// Scans a library directory, profiles missing tracks, and optionally precomputes transitions.
    pub fn scan_library(
        &self,
        root_dir: impl AsRef<Path>,
        precompute_pairs: bool,
    ) -> Result<ScanReport, CacheError> {
        let audio_files = Self::discover_audio_files(root_dir);
        let mut report = ScanReport {
            total_audio_files: audio_files.len(),
            ..Default::default()
        };

        let mut current_profiles: Vec<TrackAnalysisProfile> = Vec::new();

        // Pass 1: Profile tracks
        for path in &audio_files {
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Load audio buffer
            let audio_buf = match AudioDecoder::load_file(path) {
                Ok(b) => b,
                Err(e) => {
                    report
                        .errors
                        .push(format!("Failed to decode {}: {}", path.display(), e));
                    continue;
                }
            };

            let audio_hash = audio_buf.compute_hash();

            // Check if already in SQLite cache
            match self.db.get_tap(&audio_hash) {
                Ok(Some(existing)) => {
                    current_profiles.push(existing);
                }
                Ok(None) => {
                    // Not cached, run 5-layer profiler
                    let slug = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let profile = self.profiler.profile_track(&audio_buf, &slug, AnalysisTier::FullLocal);

                    if let Err(e) = self.db.store_tap(&profile) {
                        report.errors.push(format!(
                            "Failed to store profile in DB for {}: {}",
                            filename, e
                        ));
                    } else {
                        report.newly_profiled_tracks += 1;
                        current_profiles.push(profile);
                    }
                }
                Err(e) => {
                    report.errors.push(format!(
                        "Database read error for {}: {}",
                        filename, e
                    ));
                }
            }
        }

        // Pass 2: Pairwise transition precomputation (Smart Shuffle Matrix)
        if precompute_pairs && current_profiles.len() >= 2 {
            for i in 0..current_profiles.len() {
                for j in 0..current_profiles.len() {
                    if i == j {
                        continue;
                    }

                    let track_a = &current_profiles[i];
                    let track_b = &current_profiles[j];

                    let has_ptr = self
                        .db
                        .has_ptr(&track_a.track_id.audio_hash, &track_b.track_id.audio_hash)
                        .unwrap_or(false);

                    if !has_ptr {
                        let recipe = self.planner.plan_transition(track_a, track_b);
                        if let Err(e) = self.db.store_ptr(&recipe) {
                            report.errors.push(format!(
                                "Failed to store transition {} -> {}: {}",
                                track_a.track_id.canonical_slug,
                                track_b.track_id.canonical_slug,
                                e
                            ));
                        } else {
                            report.precomputed_transitions += 1;
                        }
                    }
                }
            }
        }

        Ok(report)
    }
}
