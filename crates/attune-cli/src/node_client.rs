use attune_core::cache::sync::{NodeStatus, SyncPullRequest, SyncPullResponse};
use attune_core::cache::CacheDatabase;

pub fn sync_with_remote_node(
    remote_url: &str,
    db: &CacheDatabase,
) -> Result<(), Box<dyn std::error::Error>> {
    let base = remote_url.trim_end_matches('/');

    println!("📡 Connecting to External Compute Node at {}...", base);

    // 1. Check Node Status
    let info_url = format!("{}/api/v1/info", base);
    let info_resp = ureq::get(&info_url).call()?;
    let info_str = info_resp.into_string()?;
    let status: NodeStatus = serde_json::from_str(&info_str)?;

    println!("✨ Connected to Node: {} (v{})", status.node_type, status.version);
    println!("   Remote Node Catalog: {} tracks, {} precomputed transitions", status.tracks_count, status.transitions_count);

    // 2. Collect local track hashes
    let local_tracks = db.list_track_ids()?;
    let local_hashes: Vec<String> = local_tracks.into_iter().map(|t| t.audio_hash).collect();

    println!("📦 Requesting transitions and profiles for {} local tracks...", local_hashes.len());

    let pull_url = format!("{}/api/v1/sync/pull", base);
    let pull_req = SyncPullRequest {
        known_track_hashes: local_hashes,
    };
    let pull_json = serde_json::to_string(&pull_req)?;

    let pull_resp = ureq::post(&pull_url)
        .set("Content-Type", "application/json")
        .send_string(&pull_json)?;

    let pull_body = pull_resp.into_string()?;
    let sync_data: SyncPullResponse = serde_json::from_str(&pull_body)?;

    let mut imported_taps = 0;
    for tap in &sync_data.missing_taps {
        if db.store_tap(tap).is_ok() {
            imported_taps += 1;
        }
    }

    let mut imported_ptrs = 0;
    for ptr in &sync_data.relevant_ptrs {
        if db.store_ptr(ptr).is_ok() {
            imported_ptrs += 1;
        }
    }

    println!("🎉 Sync Completed Successfully!");
    println!("   📥 Imported {} .tap profiles", imported_taps);
    println!("   📥 Imported {} .ptr transition recipes", imported_ptrs);

    Ok(())
}
