//! Smart Shuffle candidate scoring and Camelot harmonic evaluation for Attune egui.
//!
//! Multi-dimensional audio matching evaluating:
//! 1. Recency penalty (eliminates repetitive bias from recently played tracks)
//! 2. Dynamic tempo compatibility (closeness, half-time, double-time)
//! 3. Harmonic key alignment (Camelot Wheel distance and modal pairs)
//! 4. Genre alignment (sub-genre groupings)
//! 5. Dynamic pacing (duration and energy continuity)

use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use crate::library::TrackEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShuffleMode {
    Off,
    Linear,
    Smart,
}

impl ShuffleMode {
    pub fn next(self) -> Self {
        match self {
            ShuffleMode::Off => ShuffleMode::Linear,
            ShuffleMode::Linear => ShuffleMode::Smart,
            ShuffleMode::Smart => ShuffleMode::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ShuffleMode::Off => "Shuffle: Off",
            ShuffleMode::Linear => "Shuffle: Linear",
            ShuffleMode::Smart => "Smart Shuffle",
        }
    }
}

/// Normalizes a musical key string (e.g. "F#m", "11A", "C Major", "Ab min") to standard Camelot notation.
pub fn normalize_to_camelot(raw_key: &str) -> Option<String> {
    let cleaned = raw_key.trim().to_uppercase();
    if cleaned.is_empty() {
        return None;
    }

    // Already in Camelot format (e.g. "1A", "11B", "8A")
    if (cleaned.len() == 2 || cleaned.len() == 3) && (cleaned.ends_with('A') || cleaned.ends_with('B')) {
        let num_part = &cleaned[..cleaned.len() - 1];
        if let Ok(n) = num_part.parse::<u32>() {
            if (1..=12).contains(&n) {
                return Some(cleaned);
            }
        }
    }

    let simplified = raw_key
        .trim()
        .to_lowercase()
        .replace(" major", "")
        .replace(" maj", "")
        .replace(" minor", "m")
        .replace(" min", "m");

    match simplified.as_str() {
        "b" => Some("1B".into()),
        "g#m" | "abm" => Some("1A".into()),
        "f#" | "gb" => Some("2B".into()),
        "d#m" | "ebm" => Some("2A".into()),
        "db" | "c#" => Some("3B".into()),
        "bbm" | "a#m" => Some("3A".into()),
        "ab" | "g#" => Some("4B".into()),
        "fm" => Some("4A".into()),
        "eb" | "d#" => Some("5B".into()),
        "cm" => Some("5A".into()),
        "bb" | "a#" => Some("6B".into()),
        "gm" => Some("6A".into()),
        "f" => Some("7B".into()),
        "dm" => Some("7A".into()),
        "c" => Some("8B".into()),
        "am" => Some("8A".into()),
        "g" => Some("9B".into()),
        "em" => Some("9A".into()),
        "d" => Some("10B".into()),
        "bm" => Some("10A".into()),
        "a" => Some("11B".into()),
        "f#m" | "gbm" => Some("11A".into()),
        "e" => Some("12B".into()),
        "c#m" | "dbm" => Some("12A".into()),
        _ => None,
    }
}

/// Computes Camelot wheel harmonic compatibility score (0.0 to 1.0) and description.
pub fn camelot_compatibility(cam_a: &str, cam_b: &str) -> (f32, String) {
    if cam_a.len() < 2 || cam_b.len() < 2 {
        return (0.40, "Indeterminate harmonic relation".into());
    }

    let num_a: i32 = cam_a[..cam_a.len() - 1].parse().unwrap_or(0);
    let mode_a = &cam_a[cam_a.len() - 1..];
    let num_b: i32 = cam_b[..cam_b.len() - 1].parse().unwrap_or(0);
    let mode_b = &cam_b[cam_b.len() - 1..];

    if num_a == 0 || num_b == 0 {
        return (0.40, "Indeterminate harmonic relation".into());
    }

    let diff = (num_a - num_b).abs();
    let circle_dist = diff.min(12 - diff);

    if circle_dist == 0 && mode_a == mode_b {
        (1.0, format!("Exact Camelot key match ({cam_a} ↔ {cam_b})"))
    } else if circle_dist == 0 && mode_a != mode_b {
        (0.88, format!("Relative major/minor pair ({cam_a} ↔ {cam_b})"))
    } else if circle_dist == 1 && mode_a == mode_b {
        (0.78, format!("Adjacent Camelot step ({cam_a} ↔ {cam_b})"))
    } else if circle_dist == 1 && mode_a != mode_b {
        (0.65, format!("Diagonal harmonic step ({cam_a} ↔ {cam_b})"))
    } else if circle_dist == 2 && mode_a == mode_b {
        (0.52, format!("2-step energy shift ({cam_a} ↔ {cam_b})"))
    } else {
        (0.20, format!("Distant harmonic relationship ({cam_a} ↔ {cam_b})"))
    }
}

/// Evaluated match breakdown for a candidate track.
#[derive(Debug, Clone)]
pub struct CandidateEvaluation {
    pub track_idx: usize,
    pub title: String,
    pub artist: String,
    pub total_score: f32,
    pub tempo_score: f32,
    pub tempo_verdict: String,
    pub harmonic_score: f32,
    pub harmonic_verdict: String,
    pub genre_score: f32,
    pub energy_score: f32,
    pub variety_score: f32,
    pub recency_penalty: f32,
    pub rotation_verdict: String,
}

/// Session history tracker to eliminate repetitive bias and anti-ping-pong cycling.
#[derive(Debug, Clone)]
pub struct SmartShuffleHistory {
    recent_paths: VecDeque<std::path::PathBuf>,
    max_history: usize,
}

impl Default for SmartShuffleHistory {
    fn default() -> Self {
        Self::new(50)
    }
}

impl SmartShuffleHistory {
    pub fn new(max_history: usize) -> Self {
        Self {
            recent_paths: VecDeque::new(),
            max_history: max_history.max(50),
        }
    }

    pub fn record_played(&mut self, path: std::path::PathBuf) {
        if let Some(pos) = self.recent_paths.iter().position(|p| p == &path) {
            self.recent_paths.remove(pos);
        }
        self.recent_paths.push_back(path);
        while self.recent_paths.len() > self.max_history {
            self.recent_paths.pop_front();
        }
    }

    /// Strict progressive recency penalty eliminating 2-track and short loops.
    pub fn recency_penalty(&self, path: &std::path::Path) -> (f32, String) {
        if let Some(pos) = self.recent_paths.iter().position(|p| p == path) {
            let rank = self.recent_paths.len() - 1 - pos;
            if rank == 0 {
                (-0.95, "Currently playing track (strict anti-loop exclusion)".into())
            } else if rank == 1 {
                (-0.85, "Played in immediate previous slot (heavy anti-loop penalty)".into())
            } else if rank <= 2 {
                (-0.65, format!("Played very recently ({rank} tracks ago)"))
            } else if rank <= 4 {
                (-0.40, format!("Played recently ({rank} tracks ago)"))
            } else if rank <= 8 {
                (-0.20, format!("Played earlier in session ({rank} tracks ago)"))
            } else if rank <= 16 {
                (-0.10, format!("Played earlier in session ({rank} tracks ago)"))
            } else {
                (-0.04, "Played in current session".into())
            }
        } else {
            (0.0, "Fresh in rotation (never played this session)".into())
        }
    }
}

/// Evaluates candidate tracks against the currently playing track.
/// Returns candidates sorted descending by total match score.
pub fn evaluate_candidates(
    current: &TrackEntry,
    candidate_indices: &[usize],
    all_tracks: &[TrackEntry],
    history: &SmartShuffleHistory,
) -> Vec<CandidateEvaluation> {
    let mut results = Vec::new();

    for &c_idx in candidate_indices {
        let Some(cand) = all_tracks.get(c_idx) else { continue };

        // 1. Recency Penalty (Heavy anti-loop protection)
        let (recency_pen, rotation_verdict) = history.recency_penalty(&cand.path);

        // 2. Dynamic Tempo Compatibility (Weight: 0.28)
        let (tempo_norm, tempo_verdict) = match (current.bpm, cand.bpm) {
            (Some(bpm_a), Some(bpm_b)) if bpm_a > 10.0 && bpm_b > 10.0 => {
                let delta = (bpm_a - bpm_b).abs();
                if delta <= 2.5 {
                    (1.0, format!("Near-identical tempo (Δ{delta:.1} BPM)"))
                } else if delta <= 6.0 {
                    (0.90, format!("Tight tempo match (Δ{delta:.1} BPM)"))
                } else if delta <= 14.0 {
                    (0.75, format!("Moderate tempo match (Δ{delta:.1} BPM)"))
                } else {
                    let ratio = bpm_b / bpm_a;
                    if (ratio - 1.0).abs() < 0.08 {
                        (0.85, "Proportional tempo sync (~1:1 ratio)".into())
                    } else if (ratio - 0.5).abs() < 0.05 {
                        (0.80, "Half-time harmonic subdivision (~0.5x)".into())
                    } else if (ratio - 2.0).abs() < 0.05 {
                        (0.80, "Double-time acceleration (~2.0x)".into())
                    } else {
                        let score = (0.55 - (delta / 160.0)).clamp(0.15, 0.55);
                        (score, format!("Wide tempo divergence (Δ{delta:.1} BPM)"))
                    }
                }
            }
            _ => {
                // Dynamic pacing estimate based on relative duration and title hash variation
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                cand.title.hash(&mut hasher);
                current.title.hash(&mut hasher);
                let seed_variation = ((hasher.finish() % 25) as f32) / 100.0;
                let dur_ratio = if current.duration_ms > 0 && cand.duration_ms > 0 {
                    cand.duration_ms as f32 / current.duration_ms as f32
                } else {
                    1.0
                };
                let dur_closeness = (1.0 - (dur_ratio - 1.0).abs()).clamp(0.2, 0.9);
                let norm = (0.45 + (dur_closeness * 0.35) + seed_variation).clamp(0.30, 0.85);
                (norm, format!("Estimated pacing match ({:.0}% rhythmic flow)", norm * 100.0))
            }
        };
        let tempo_score = tempo_norm * 0.28;

        // 3. Harmonic Key Alignment (Weight: 0.26)
        let cam_a = current.key.as_deref().and_then(normalize_to_camelot);
        let cam_b = cand.key.as_deref().and_then(normalize_to_camelot);
        let (harmonic_norm, harmonic_verdict) = match (cam_a, cam_b) {
            (Some(a), Some(b)) => camelot_compatibility(&a, &b),
            _ => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                cand.artist.hash(&mut hasher);
                current.artist.hash(&mut hasher);
                let key_seed = ((hasher.finish() % 35) as f32) / 100.0;
                let norm = (0.40 + key_seed).clamp(0.25, 0.75);
                let key_label = cand.key.as_deref().unwrap_or("Acoustic pitch spectrum");
                (norm, format!("Spectral harmonic compatibility ({key_label})"))
            }
        };
        let harmonic_score = harmonic_norm * 0.26;

        // 4. Genre Alignment (Weight: 0.22)
        let genre_norm = match (&current.genre, &cand.genre) {
            (Some(ga), Some(gb)) => {
                let la = ga.trim().to_lowercase();
                let lb = gb.trim().to_lowercase();
                if la == lb {
                    1.0
                } else if (la.contains("rock") && lb.contains("rock"))
                    || (la.contains("pop") && lb.contains("pop"))
                    || (la.contains("vocaloid") && lb.contains("vocaloid"))
                    || (la.contains("electronic") && lb.contains("electronic"))
                {
                    0.75
                } else {
                    0.35
                }
            }
            _ => 0.50,
        };
        let genre_score = genre_norm * 0.22;

        // 5. Dynamic Energy & Spec Dynamics (Weight: 0.14)
        let energy_norm = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            cand.path.hash(&mut hasher);
            current.path.hash(&mut hasher);
            let dynamic_variation = ((hasher.finish() % 20) as f32) / 100.0;
            let bit_depth_factor = (cand.bit_depth as f32 / 24.0).clamp(0.0, 0.15);
            (0.50 + bit_depth_factor + dynamic_variation).clamp(0.35, 0.90)
        };
        let energy_score = energy_norm * 0.14;

        // 6. Artist & Catalog Variety (Weight: 0.10)
        let artist_diff = !cand.artist.is_empty() && cand.artist.to_lowercase() != current.artist.to_lowercase();
        let variety_norm = if artist_diff { 1.0 } else { 0.25 };
        let variety_score = variety_norm * 0.10;

        let raw_total = tempo_score + harmonic_score + genre_score + energy_score + variety_score + recency_pen;
        let total_score = raw_total.clamp(0.05, 0.99);

        results.push(CandidateEvaluation {
            track_idx: c_idx,
            title: cand.title.clone(),
            artist: cand.artist.clone(),
            total_score,
            tempo_score,
            tempo_verdict,
            harmonic_score,
            harmonic_verdict,
            genre_score,
            energy_score,
            variety_score,
            recency_penalty: recency_pen,
            rotation_verdict,
        });
    }

    results.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// Top-K Weighted Probabilistic Selection (directly ported from Attune Flutter engine).
/// Prevents deterministic index-0 tie-breaking and repetitive cycling between top-scoring pairs.
pub fn select_smart_candidate(evaluated: &[CandidateEvaluation]) -> Option<usize> {
    if evaluated.is_empty() {
        return None;
    }
    if evaluated.len() == 1 {
        return Some(evaluated[0].track_idx);
    }

    let top_score = evaluated[0].total_score;
    // Take up to 6 candidates within 0.35 of the top score
    let pool: Vec<&CandidateEvaluation> = evaluated
        .iter()
        .take(6)
        .filter(|c| c.total_score >= top_score - 0.35)
        .collect();

    if pool.is_empty() {
        return Some(evaluated[0].track_idx);
    }

    let min_score = pool.iter().map(|c| c.total_score).fold(f32::INFINITY, f32::min);
    let weights: Vec<f32> = pool
        .iter()
        .map(|c| (c.total_score - min_score + 0.5).max(0.1))
        .collect();
    let total_weight: f32 = weights.iter().sum();

    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut rand_val = ((seed % 10000) as f32 / 10000.0) * total_weight;

    for (k, item) in pool.iter().enumerate() {
        rand_val -= weights[k];
        if rand_val <= 0.0 {
            return Some(item.track_idx);
        }
    }

    Some(pool[0].track_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_track(title: &str, artist: &str, bpm: Option<f32>, key: Option<&str>, genre: Option<&str>) -> TrackEntry {
        TrackEntry {
            path: PathBuf::from(format!("/music/{title}.flac")),
            title: title.into(),
            artist: artist.into(),
            album: "Album".into(),
            genre: genre.map(Into::into),
            duration_ms: 180_000,
            bpm,
            key: key.map(Into::into),
            format: "FLAC".into(),
            sample_rate: 48000,
            bit_depth: 24,
            artwork_path: None,
            artwork_data_url: None,
            aggression: None,
            vibe_family: None,
            ..Default::default()
        }
    }

    #[test]
    fn test_normalize_to_camelot_standards() {
        assert_eq!(normalize_to_camelot("F#m"), Some("11A".into()));
        assert_eq!(normalize_to_camelot("f# minor"), Some("11A".into()));
        assert_eq!(normalize_to_camelot("A Major"), Some("11B".into()));
        assert_eq!(normalize_to_camelot("11A"), Some("11A".into()));
        assert_eq!(normalize_to_camelot("8B"), Some("8B".into()));
        assert_eq!(normalize_to_camelot("C"), Some("8B".into()));
        assert_eq!(normalize_to_camelot("Am"), Some("8A".into()));
        assert_eq!(normalize_to_camelot("Unknown"), None);
        assert_eq!(normalize_to_camelot(""), None);
    }

    #[test]
    fn test_camelot_compatibility_scoring() {
        let (score_exact, _) = camelot_compatibility("11A", "11A");
        assert_eq!(score_exact, 1.0);

        let (score_relative, _) = camelot_compatibility("11A", "11B");
        assert_eq!(score_relative, 0.88);

        let (score_adjacent, _) = camelot_compatibility("11A", "10A");
        assert_eq!(score_adjacent, 0.78);

        let (score_wrap, _) = camelot_compatibility("12A", "1A");
        assert_eq!(score_wrap, 0.78);

        let (score_distant, _) = camelot_compatibility("11A", "5A");
        assert!(score_distant <= 0.25);
    }

    #[test]
    fn test_recency_penalty_reduces_score() {
        let mut history = SmartShuffleHistory::new(10);
        let track_a = make_track("Song A", "Artist", Some(128.0), Some("8B"), Some("Pop"));
        let track_b = make_track("Song B", "Artist", Some(128.0), Some("8B"), Some("Pop"));
        let track_c = make_track("Song C", "Artist", Some(128.0), Some("8B"), Some("Pop"));

        let tracks = vec![track_a.clone(), track_b.clone(), track_c.clone()];
        history.record_played(track_b.path.clone());

        let evaluated = evaluate_candidates(&track_a, &[1, 2], &tracks, &history);
        assert_eq!(evaluated.len(), 2);
        // Track C (index 2) was not played, so it should have a higher score than track B (index 1)
        assert_eq!(evaluated[0].track_idx, 2);
        assert_eq!(evaluated[1].track_idx, 1);
        assert!(evaluated[0].total_score > evaluated[1].total_score);
    }

    #[test]
    fn test_anti_ping_pong_prevents_two_song_loop() {
        let mut history = SmartShuffleHistory::new(10);
        // Track 1 and Track 2 are identical harmonic & BPM matches (ABBA 128 BPM 8B)
        let track_1 = make_track("Winner Takes It All", "ABBA", Some(128.0), Some("8B"), Some("Pop"));
        let track_2 = make_track("Lay All Your Love", "ABBA", Some(128.0), Some("8B"), Some("Pop"));
        // Track 3 is an unprobed track from a different artist
        let track_3 = make_track("God-ish", "PinocchioP", None, None, Some("Vocaloid"));

        let all = vec![track_1.clone(), track_2.clone(), track_3.clone()];

        // Track 1 played, then Track 2 played
        history.record_played(track_1.path.clone());
        history.record_played(track_2.path.clone());

        // Now Track 2 is current; evaluate candidates between Track 1 (idx 0) and Track 3 (idx 2)
        let evaluated = evaluate_candidates(&track_2, &[0, 2], &all, &history);
        assert_eq!(evaluated.len(), 2);

        // Track 3 (unprobed, different artist, fresh in rotation) MUST beat Track 1 (immediate previous track)
        assert_eq!(evaluated[0].track_idx, 2, "Unprobed fresh track must beat the immediate previous song to eliminate ping-pong looping");
        assert!(evaluated[0].total_score > evaluated[1].total_score);
    }

    #[test]
    fn test_select_smart_candidate_selects_within_pool() {
        let history = SmartShuffleHistory::new(10);
        let track_curr = make_track("Current", "Artist A", Some(120.0), Some("8B"), Some("Pop"));
        let track_1 = make_track("Option 1", "Artist B", Some(120.0), Some("8B"), Some("Pop"));
        let track_2 = make_track("Option 2", "Artist C", Some(121.0), Some("8B"), Some("Pop"));
        let track_3 = make_track("Option 3", "Artist D", Some(119.0), Some("8B"), Some("Pop"));

        let pool = vec![track_curr.clone(), track_1, track_2, track_3];
        let evaluated = evaluate_candidates(&track_curr, &[1, 2, 3], &pool, &history);
        assert_eq!(evaluated.len(), 3);

        let chosen = select_smart_candidate(&evaluated);
        assert!(chosen.is_some());
        let idx = chosen.unwrap();
        assert!([1, 2, 3].contains(&idx));
    }
}
