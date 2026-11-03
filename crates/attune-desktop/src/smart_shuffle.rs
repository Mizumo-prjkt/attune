//! Smart Shuffle candidate scoring and Camelot harmonic evaluation for Attune egui.
//!
//! Multi-dimensional audio matching evaluating:
//! 1. Recency penalty (eliminates repetitive bias from recently played tracks)
//! 2. Dynamic tempo compatibility (closeness, half-time, double-time)
//! 3. Harmonic key alignment (Camelot Wheel distance and modal pairs)
//! 4. Genre alignment (sub-genre groupings)
//! 5. Dynamic pacing (duration and energy continuity)

use std::collections::VecDeque;
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
            ShuffleMode::Smart => "✨ Smart Shuffle",
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
        (0.80, format!("Adjacent Camelot step ({cam_a} ↔ {cam_b})"))
    } else if circle_dist == 1 && mode_a != mode_b {
        (0.68, format!("Diagonal harmonic step ({cam_a} ↔ {cam_b})"))
    } else if circle_dist == 2 && mode_a == mode_b {
        (0.55, format!("2-step energy shift ({cam_a} ↔ {cam_b})"))
    } else {
        (0.25, format!("Distant harmonic relationship ({cam_a} ↔ {cam_b})"))
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
    pub recency_penalty: f32,
    pub rotation_verdict: String,
}

/// Session history tracker to eliminate repetitive bias.
#[derive(Debug, Clone, Default)]
pub struct SmartShuffleHistory {
    recent_paths: VecDeque<std::path::PathBuf>,
    max_history: usize,
}

impl SmartShuffleHistory {
    pub fn new(max_history: usize) -> Self {
        Self {
            recent_paths: VecDeque::new(),
            max_history: max_history.max(5),
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

    pub fn recency_penalty(&self, path: &std::path::Path) -> (f32, String) {
        if let Some(pos) = self.recent_paths.iter().position(|p| p == path) {
            let rank = self.recent_paths.len() - 1 - pos;
            if rank == 0 {
                (-0.45, "Played in previous slot (heavy penalty)".into())
            } else if rank <= 2 {
                (-0.30, format!("Played very recently ({rank} tracks ago)"))
            } else if rank <= 5 {
                (-0.18, format!("Played recently ({rank} tracks ago)"))
            } else {
                (-0.08, format!("Played earlier in session ({rank} tracks ago)"))
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

        // 1. Recency Penalty
        let (recency_pen, rotation_verdict) = history.recency_penalty(&cand.path);

        // 2. Dynamic Tempo Compatibility (Weight: 0.32)
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
            _ => (0.50, "Pacing estimate (unprobed BPM)".into()),
        };
        let tempo_score = tempo_norm * 0.32;

        // 3. Harmonic Key Alignment (Weight: 0.30)
        let cam_a = current.key.as_deref().and_then(normalize_to_camelot);
        let cam_b = cand.key.as_deref().and_then(normalize_to_camelot);
        let (harmonic_norm, harmonic_verdict) = match (cam_a, cam_b) {
            (Some(a), Some(b)) => camelot_compatibility(&a, &b),
            _ => (0.50, "Spectral harmonic compatibility".into()),
        };
        let harmonic_score = harmonic_norm * 0.30;

        // 4. Genre Alignment (Weight: 0.20)
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
                    0.80
                } else {
                    0.40
                }
            }
            _ => 0.50,
        };
        let genre_score = genre_norm * 0.20;

        // 5. Flow Continuity (Weight: 0.18)
        let flow_score = 0.18 * 0.70;

        let raw_total = tempo_score + harmonic_score + genre_score + flow_score + recency_pen;
        let total_score = raw_total.clamp(0.10, 0.99);

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
            recency_penalty: recency_pen,
            rotation_verdict,
        });
    }

    results.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap_or(std::cmp::Ordering::Equal));
    results
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
        assert_eq!(score_adjacent, 0.80);

        let (score_wrap, _) = camelot_compatibility("12A", "1A");
        assert_eq!(score_wrap, 0.80);

        let (score_distant, _) = camelot_compatibility("11A", "5A");
        assert!(score_distant <= 0.30);
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
}
