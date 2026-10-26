use crate::schema::{PairwiseTransitionRecipe, TrackAnalysisProfile, TrackId};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("SQLite database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Thread lock poisoned: {0}")]
    LockPoisoned(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StemBridgeRecord {
    pub track_a_hash: String,
    pub track_b_hash: String,
    pub stem_type: String,
    pub uri: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheStats {
    pub tracks_count: usize,
    pub transitions_count: usize,
    pub stem_bridges_count: usize,
    pub database_path: Option<String>,
}

#[derive(Clone)]
pub struct CacheDatabase {
    conn: Arc<Mutex<Connection>>,
    db_path: Option<PathBuf>,
}

impl CacheDatabase {
    /// Opens or creates an Attune SQLite cache database at the specified filesystem path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CacheError> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(p)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: Some(p.to_path_buf()),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Opens an in-memory SQLite cache database (useful for testing or ephemeral sessions).
    pub fn open_in_memory() -> Result<Self, CacheError> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: None,
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Returns the standard default cache path (~/.attune/cache.db or ./attune_cache.db).
    pub fn default_path() -> PathBuf {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join(".attune").join("cache.db")
        } else {
            PathBuf::from("attune_cache.db")
        }
    }

    fn init_schema(&self) -> Result<(), CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;

        // Enable WAL mode for high concurrent read performance
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;");

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tracks (
                audio_hash TEXT PRIMARY KEY,
                canonical_slug TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                analysis_tier TEXT NOT NULL,
                nominal_bpm REAL NOT NULL,
                primary_key TEXT NOT NULL,
                macro_vibe TEXT NOT NULL,
                aggression REAL NOT NULL,
                tap_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tracks_slug ON tracks(canonical_slug);
            CREATE INDEX IF NOT EXISTS idx_tracks_bpm ON tracks(nominal_bpm);

            CREATE TABLE IF NOT EXISTS transitions (
                track_a_hash TEXT NOT NULL,
                track_b_hash TEXT NOT NULL,
                archetype TEXT NOT NULL,
                confidence_score REAL NOT NULL,
                ptr_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (track_a_hash, track_b_hash)
            );
            CREATE INDEX IF NOT EXISTS idx_trans_a ON transitions(track_a_hash);
            CREATE INDEX IF NOT EXISTS idx_trans_b ON transitions(track_b_hash);

            CREATE TABLE IF NOT EXISTS stem_bridges (
                track_a_hash TEXT NOT NULL,
                track_b_hash TEXT NOT NULL,
                stem_type TEXT NOT NULL,
                uri TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (track_a_hash, track_b_hash, stem_type)
            );
            "#,
        )?;

        Ok(())
    }

    /// Stores or updates a Track Analysis Profile (.tap) in SQLite.
    pub fn store_tap(&self, profile: &TrackAnalysisProfile) -> Result<(), CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let tap_json = serde_json::to_string(profile)?;
        let tier_str = serde_json::to_string(&profile.analysis_tier)?.replace('"', "");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            r#"
            INSERT INTO tracks (
                audio_hash, canonical_slug, duration_ms, analysis_tier,
                nominal_bpm, primary_key, macro_vibe, aggression, tap_json, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(audio_hash) DO UPDATE SET
                canonical_slug = excluded.canonical_slug,
                duration_ms = excluded.duration_ms,
                analysis_tier = excluded.analysis_tier,
                nominal_bpm = excluded.nominal_bpm,
                primary_key = excluded.primary_key,
                macro_vibe = excluded.macro_vibe,
                aggression = excluded.aggression,
                tap_json = excluded.tap_json,
                updated_at = excluded.updated_at;
            "#,
            params![
                profile.track_id.audio_hash,
                profile.track_id.canonical_slug,
                profile.track_id.duration_ms as i64,
                tier_str,
                profile.global_summary.nominal_bpm,
                profile.global_summary.primary_key,
                profile.global_summary.macro_vibe_family,
                profile.global_summary.mean_aggression_index,
                tap_json,
                now,
            ],
        )?;

        Ok(())
    }

    /// Retrieves a .tap profile by audio_hash.
    pub fn get_tap(&self, audio_hash: &str) -> Result<Option<TrackAnalysisProfile>, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT tap_json FROM tracks WHERE audio_hash = ?1")?;
        let mut rows = stmt.query(params![audio_hash])?;

        if let Some(row) = rows.next()? {
            let json_str: String = row.get(0)?;
            let profile: TrackAnalysisProfile = serde_json::from_str(&json_str)?;
            Ok(Some(profile))
        } else {
            Ok(None)
        }
    }

    /// Checks if a track profile exists in cache without deserializing full JSON.
    pub fn has_tap(&self, audio_hash: &str) -> Result<bool, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tracks WHERE audio_hash = ?1",
            params![audio_hash],
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    /// Stores or updates a Pairwise Transition Recipe (.ptr) in SQLite.
    pub fn store_ptr(&self, recipe: &PairwiseTransitionRecipe) -> Result<(), CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let ptr_json = serde_json::to_string(recipe)?;
        let archetype_str = serde_json::to_string(&recipe.archetype)?.replace('"', "");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            r#"
            INSERT INTO transitions (
                track_a_hash, track_b_hash, archetype, confidence_score, ptr_json, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(track_a_hash, track_b_hash) DO UPDATE SET
                archetype = excluded.archetype,
                confidence_score = excluded.confidence_score,
                ptr_json = excluded.ptr_json,
                updated_at = excluded.updated_at;
            "#,
            params![
                recipe.track_a_hash,
                recipe.track_b_hash,
                archetype_str,
                recipe.confidence_score,
                ptr_json,
                now,
            ],
        )?;

        Ok(())
    }

    /// Retrieves a .ptr recipe by track pair hashes.
    pub fn get_ptr(
        &self,
        track_a_hash: &str,
        track_b_hash: &str,
    ) -> Result<Option<PairwiseTransitionRecipe>, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT ptr_json FROM transitions WHERE track_a_hash = ?1 AND track_b_hash = ?2",
        )?;
        let mut rows = stmt.query(params![track_a_hash, track_b_hash])?;

        if let Some(row) = rows.next()? {
            let json_str: String = row.get(0)?;
            let recipe: PairwiseTransitionRecipe = serde_json::from_str(&json_str)?;
            Ok(Some(recipe))
        } else {
            Ok(None)
        }
    }

    /// Checks if a transition recipe exists in cache without deserializing full JSON.
    pub fn has_ptr(&self, track_a_hash: &str, track_b_hash: &str) -> Result<bool, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM transitions WHERE track_a_hash = ?1 AND track_b_hash = ?2",
            params![track_a_hash, track_b_hash],
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    /// Stores a stem bridge record (e.g. isolated vocal or riff Demucs export).
    pub fn store_stem_bridge(&self, record: &StemBridgeRecord) -> Result<(), CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            r#"
            INSERT INTO stem_bridges (
                track_a_hash, track_b_hash, stem_type, uri, duration_ms, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(track_a_hash, track_b_hash, stem_type) DO UPDATE SET
                uri = excluded.uri,
                duration_ms = excluded.duration_ms,
                created_at = excluded.created_at;
            "#,
            params![
                record.track_a_hash,
                record.track_b_hash,
                record.stem_type,
                record.uri,
                record.duration_ms as i64,
                now,
            ],
        )?;

        Ok(())
    }

    /// Retrieves an optional stem bridge record.
    pub fn get_stem_bridge(
        &self,
        track_a_hash: &str,
        track_b_hash: &str,
        stem_type: &str,
    ) -> Result<Option<StemBridgeRecord>, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT uri, duration_ms FROM stem_bridges WHERE track_a_hash = ?1 AND track_b_hash = ?2 AND stem_type = ?3",
        )?;
        let mut rows = stmt.query(params![track_a_hash, track_b_hash, stem_type])?;

        if let Some(row) = rows.next()? {
            let uri: String = row.get(0)?;
            let duration_ms: i64 = row.get(1)?;
            Ok(Some(StemBridgeRecord {
                track_a_hash: track_a_hash.to_string(),
                track_b_hash: track_b_hash.to_string(),
                stem_type: stem_type.to_string(),
                uri,
                duration_ms: duration_ms as u64,
            }))
        } else {
            Ok(None)
        }
    }

    /// Lists all tracked IDs with basic metadata.
    pub fn list_track_ids(&self) -> Result<Vec<TrackId>, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT audio_hash, canonical_slug, duration_ms FROM tracks ORDER BY canonical_slug ASC")?;
        let rows = stmt.query_map([], |row| {
            let hash: String = row.get(0)?;
            let slug: String = row.get(1)?;
            let dur: i64 = row.get(2)?;
            Ok(TrackId {
                audio_hash: hash,
                canonical_slug: slug,
                duration_ms: dur as u64,
            })
        })?;

        let mut tracks = Vec::new();
        for r in rows {
            tracks.push(r?);
        }
        Ok(tracks)
    }

    /// Lists all Track Analysis Profiles in cache.
    pub fn list_all_taps(&self) -> Result<Vec<TrackAnalysisProfile>, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT tap_json FROM tracks ORDER BY canonical_slug ASC")?;
        let rows = stmt.query_map([], |row| {
            let json_str: String = row.get(0)?;
            Ok(json_str)
        })?;

        let mut profiles = Vec::new();
        for r in rows {
            let s = r?;
            let p: TrackAnalysisProfile = serde_json::from_str(&s)?;
            profiles.push(p);
        }
        Ok(profiles)
    }

    /// Lists all Pairwise Transition Recipes in cache.
    pub fn list_all_ptrs(&self) -> Result<Vec<PairwiseTransitionRecipe>, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT ptr_json FROM transitions ORDER BY confidence_score DESC")?;
        let rows = stmt.query_map([], |row| {
            let json_str: String = row.get(0)?;
            Ok(json_str)
        })?;

        let mut recipes = Vec::new();
        for r in rows {
            let s = r?;
            let p: PairwiseTransitionRecipe = serde_json::from_str(&s)?;
            recipes.push(p);
        }
        Ok(recipes)
    }

    /// Lists all Stem Bridge records in cache.
    pub fn list_all_stem_bridges(&self) -> Result<Vec<StemBridgeRecord>, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT track_a_hash, track_b_hash, stem_type, uri, duration_ms FROM stem_bridges",
        )?;
        let rows = stmt.query_map([], |row| {
            let a: String = row.get(0)?;
            let b: String = row.get(1)?;
            let st: String = row.get(2)?;
            let uri: String = row.get(3)?;
            let dur: i64 = row.get(4)?;
            Ok(StemBridgeRecord {
                track_a_hash: a,
                track_b_hash: b,
                stem_type: st,
                uri,
                duration_ms: dur as u64,
            })
        })?;

        let mut bridges = Vec::new();
        for r in rows {
            bridges.push(r?);
        }
        Ok(bridges)
    }

    /// Returns top-K transitions ordered by confidence for a given outgoing track.
    pub fn top_transitions_for_track(
        &self,
        track_a_hash: &str,
        limit: usize,
    ) -> Result<Vec<PairwiseTransitionRecipe>, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT ptr_json FROM transitions WHERE track_a_hash = ?1 ORDER BY confidence_score DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![track_a_hash, limit as i64], |row| {
            let json_str: String = row.get(0)?;
            Ok(json_str)
        })?;

        let mut recipes = Vec::new();
        for r in rows {
            let s = r?;
            let p: PairwiseTransitionRecipe = serde_json::from_str(&s)?;
            recipes.push(p);
        }
        Ok(recipes)
    }

    /// Returns high-level statistics for the cache.
    pub fn get_stats(&self) -> Result<CacheStats, CacheError> {
        let conn = self.conn.lock().map_err(|e| CacheError::LockPoisoned(e.to_string()))?;
        let tracks_count: i64 = conn.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))?;
        let transitions_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM transitions", [], |r| r.get(0))?;
        let stem_bridges_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM stem_bridges", [], |r| r.get(0))?;

        Ok(CacheStats {
            tracks_count: tracks_count as usize,
            transitions_count: transitions_count as usize,
            stem_bridges_count: stem_bridges_count as usize,
            database_path: self.db_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        })
    }
}
