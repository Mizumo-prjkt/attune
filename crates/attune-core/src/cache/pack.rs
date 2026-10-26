use super::db::{CacheDatabase, CacheError, StemBridgeRecord};
use crate::schema::{PairwiseTransitionRecipe, TrackAnalysisProfile};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachePack {
    pub schema_version: u32,
    pub created_at_utc: u64,
    pub generator: String,
    pub taps: Vec<TrackAnalysisProfile>,
    pub ptrs: Vec<PairwiseTransitionRecipe>,
    pub stem_bridges: Vec<StemBridgeRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ImportStats {
    pub taps_imported: usize,
    pub ptrs_imported: usize,
    pub stem_bridges_imported: usize,
}

impl CachePack {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            schema_version: 1,
            created_at_utc: now,
            generator: "Attune External Compute Node v0.3.0".to_string(),
            taps: Vec::new(),
            ptrs: Vec::new(),
            stem_bridges: Vec::new(),
        }
    }

    /// Exports entire CacheDatabase content into a portable CachePack.
    pub fn from_db(db: &CacheDatabase) -> Result<Self, CacheError> {
        let taps = db.list_all_taps()?;
        let ptrs = db.list_all_ptrs()?;
        let stem_bridges = db.list_all_stem_bridges()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(Self {
            schema_version: 1,
            created_at_utc: now,
            generator: "Attune External Compute Node v0.3.0".to_string(),
            taps,
            ptrs,
            stem_bridges,
        })
    }

    /// Imports all records in this pack into the specified CacheDatabase.
    pub fn import_into_db(&self, db: &CacheDatabase) -> Result<ImportStats, CacheError> {
        let mut stats = ImportStats::default();

        for tap in &self.taps {
            db.store_tap(tap)?;
            stats.taps_imported += 1;
        }

        for ptr in &self.ptrs {
            db.store_ptr(ptr)?;
            stats.ptrs_imported += 1;
        }

        for bridge in &self.stem_bridges {
            db.store_stem_bridge(bridge)?;
            stats.stem_bridges_imported += 1;
        }

        Ok(stats)
    }

    /// Saves the pack to a JSON file on disk.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), CacheError> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let f = std::fs::File::create(p)?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }

    /// Loads a pack from a JSON file on disk.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, CacheError> {
        let f = std::fs::File::open(path)?;
        let pack: Self = serde_json::from_reader(f)?;
        Ok(pack)
    }
}

impl Default for CachePack {
    fn default() -> Self {
        Self::new()
    }
}
