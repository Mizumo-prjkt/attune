use crate::schema::{PairwiseTransitionRecipe, TrackAnalysisProfile};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeStatus {
    pub status: String,
    pub version: String,
    pub node_type: String,
    pub tracks_count: usize,
    pub transitions_count: usize,
    pub stem_bridges_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncPushRequest {
    pub taps: Vec<TrackAnalysisProfile>,
    pub ptrs: Vec<PairwiseTransitionRecipe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncPushResponse {
    pub status: String,
    pub taps_stored: usize,
    pub ptrs_stored: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncPullRequest {
    pub known_track_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncPullResponse {
    pub status: String,
    pub missing_taps: Vec<TrackAnalysisProfile>,
    pub relevant_ptrs: Vec<PairwiseTransitionRecipe>,
}
