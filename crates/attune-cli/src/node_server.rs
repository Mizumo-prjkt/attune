use attune_core::cache::sync::{NodeStatus, SyncPullRequest, SyncPullResponse, SyncPushRequest, SyncPushResponse};
use attune_core::cache::{CacheDatabase, CachePack, LibraryScanner};
use attune_core::schema::{PairwiseTransitionRecipe, TrackAnalysisProfile};
use std::path::PathBuf;
use std::sync::Arc;
use tiny_http::{Header, Response, Server, StatusCode};

pub fn run_node_server(
    port: u16,
    db: CacheDatabase,
    scan_dir: Option<PathBuf>,
    precompute_pairs: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&addr).map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    println!("🚀 Attune External Compute Node running at http://{}", addr);
    println!("   API Endpoints:");
    println!("     GET  /api/v1/info");
    println!("     GET  /api/v1/tracks");
    println!("     GET  /api/v1/tap/:hash");
    println!("     POST /api/v1/tap");
    println!("     GET  /api/v1/ptr/:hash_a/:hash_b");
    println!("     POST /api/v1/ptr");
    println!("     GET  /api/v1/export");
    println!("     POST /api/v1/sync/push");
    println!("     POST /api/v1/sync/pull");

    // If an initial music directory is specified, scan in background thread
    if let Some(dir) = scan_dir {
        let scanner_db = db.clone();
        std::thread::spawn(move || {
            println!("🔍 Background Worker: Scanning audio directory: {}...", dir.display());
            let scanner = LibraryScanner::new(scanner_db);
            match scanner.scan_library(&dir, precompute_pairs) {
                Ok(report) => {
                    println!(
                        "✅ Background Worker: Scan complete! Found {} files, profiled {} new tracks, precomputed {} transitions.",
                        report.total_audio_files,
                        report.newly_profiled_tracks,
                        report.precomputed_transitions
                    );
                }
                Err(e) => {
                    eprintln!("❌ Background Worker error: {}", e);
                }
            }
        });
    }

    let shared_server = Arc::new(server);

    for mut request in shared_server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().as_str().to_uppercase();

        let json_header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();

        match (method.as_str(), url.as_str()) {
            ("GET", "/api/v1/info") => {
                let stats = db.get_stats().unwrap_or(attune_core::cache::CacheStats {
                    tracks_count: 0,
                    transitions_count: 0,
                    stem_bridges_count: 0,
                    database_path: None,
                });
                let info = NodeStatus {
                    status: "ok".to_string(),
                    version: "0.3.0".to_string(),
                    node_type: "EXTERNAL_COMPUTE_NODE".to_string(),
                    tracks_count: stats.tracks_count,
                    transitions_count: stats.transitions_count,
                    stem_bridges_count: stats.stem_bridges_count,
                };
                let body = serde_json::to_string_pretty(&info).unwrap();
                let res = Response::from_string(body).with_header(json_header);
                let _ = request.respond(res);
            }

            ("GET", "/api/v1/tracks") => {
                let tracks = db.list_track_ids().unwrap_or_default();
                let body = serde_json::to_string_pretty(&tracks).unwrap();
                let res = Response::from_string(body).with_header(json_header);
                let _ = request.respond(res);
            }

            ("GET", "/api/v1/export") => {
                match CachePack::from_db(&db) {
                    Ok(pack) => {
                        let body = serde_json::to_string(&pack).unwrap();
                        let res = Response::from_string(body).with_header(json_header);
                        let _ = request.respond(res);
                    }
                    Err(e) => {
                        let res = Response::from_string(format!("{{\"error\": \"{}\"}}", e))
                            .with_status_code(StatusCode(500))
                            .with_header(json_header);
                        let _ = request.respond(res);
                    }
                }
            }

            ("POST", "/api/v1/tap") => {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<TrackAnalysisProfile>(&body) {
                    Ok(profile) => {
                        let _ = db.store_tap(&profile);
                        let res = Response::from_string("{\"status\": \"stored\"}").with_header(json_header);
                        let _ = request.respond(res);
                    }
                    Err(e) => {
                        let res = Response::from_string(format!("{{\"error\": \"{}\"}}", e))
                            .with_status_code(StatusCode(400))
                            .with_header(json_header);
                        let _ = request.respond(res);
                    }
                }
            }

            ("POST", "/api/v1/ptr") => {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<PairwiseTransitionRecipe>(&body) {
                    Ok(recipe) => {
                        let _ = db.store_ptr(&recipe);
                        let res = Response::from_string("{\"status\": \"stored\"}").with_header(json_header);
                        let _ = request.respond(res);
                    }
                    Err(e) => {
                        let res = Response::from_string(format!("{{\"error\": \"{}\"}}", e))
                            .with_status_code(StatusCode(400))
                            .with_header(json_header);
                        let _ = request.respond(res);
                    }
                }
            }

            ("POST", "/api/v1/sync/push") => {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<SyncPushRequest>(&body) {
                    Ok(push) => {
                        let mut stored_taps = 0;
                        let mut stored_ptrs = 0;
                        for tap in push.taps {
                            if db.store_tap(&tap).is_ok() {
                                stored_taps += 1;
                            }
                        }
                        for ptr in push.ptrs {
                            if db.store_ptr(&ptr).is_ok() {
                                stored_ptrs += 1;
                            }
                        }
                        let resp = SyncPushResponse {
                            status: "ok".to_string(),
                            taps_stored: stored_taps,
                            ptrs_stored: stored_ptrs,
                        };
                        let body = serde_json::to_string(&resp).unwrap();
                        let res = Response::from_string(body).with_header(json_header);
                        let _ = request.respond(res);
                    }
                    Err(e) => {
                        let res = Response::from_string(format!("{{\"error\": \"{}\"}}", e))
                            .with_status_code(StatusCode(400))
                            .with_header(json_header);
                        let _ = request.respond(res);
                    }
                }
            }

            ("POST", "/api/v1/sync/pull") => {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<SyncPullRequest>(&body) {
                    Ok(pull) => {
                        let mut missing_taps = Vec::new();
                        let mut relevant_ptrs = Vec::new();

                        // Supply profiles and transitions for requested hashes
                        for hash in &pull.known_track_hashes {
                            if let Ok(Some(tap)) = db.get_tap(hash) {
                                missing_taps.push(tap);
                            }
                            if let Ok(ptrs) = db.top_transitions_for_track(hash, 5) {
                                relevant_ptrs.extend(ptrs);
                            }
                        }

                        let resp = SyncPullResponse {
                            status: "ok".to_string(),
                            missing_taps,
                            relevant_ptrs,
                        };
                        let body = serde_json::to_string(&resp).unwrap();
                        let res = Response::from_string(body).with_header(json_header);
                        let _ = request.respond(res);
                    }
                    Err(e) => {
                        let res = Response::from_string(format!("{{\"error\": \"{}\"}}", e))
                            .with_status_code(StatusCode(400))
                            .with_header(json_header);
                        let _ = request.respond(res);
                    }
                }
            }

            _ => {
                // Dynamic path handlers
                if method == "GET" && url.starts_with("/api/v1/tap/") {
                    let hash = &url["/api/v1/tap/".len()..];
                    match db.get_tap(hash) {
                        Ok(Some(tap)) => {
                            let body = serde_json::to_string_pretty(&tap).unwrap();
                            let res = Response::from_string(body).with_header(json_header);
                            let _ = request.respond(res);
                        }
                        Ok(None) => {
                            let res = Response::from_string("{\"error\": \"Not found\"}")
                                .with_status_code(StatusCode(404))
                                .with_header(json_header);
                            let _ = request.respond(res);
                        }
                        Err(e) => {
                            let res = Response::from_string(format!("{{\"error\": \"{}\"}}", e))
                                .with_status_code(StatusCode(500))
                                .with_header(json_header);
                            let _ = request.respond(res);
                        }
                    }
                } else if method == "GET" && url.starts_with("/api/v1/ptr/") {
                    let rest = &url["/api/v1/ptr/".len()..];
                    let parts: Vec<&str> = rest.split('/').collect();
                    if parts.len() == 2 {
                        match db.get_ptr(parts[0], parts[1]) {
                            Ok(Some(ptr)) => {
                                let body = serde_json::to_string_pretty(&ptr).unwrap();
                                let res = Response::from_string(body).with_header(json_header);
                                let _ = request.respond(res);
                            }
                            Ok(None) => {
                                let res = Response::from_string("{\"error\": \"Not found\"}")
                                    .with_status_code(StatusCode(404))
                                    .with_header(json_header);
                                let _ = request.respond(res);
                            }
                            Err(e) => {
                                let res = Response::from_string(format!("{{\"error\": \"{}\"}}", e))
                                    .with_status_code(StatusCode(500))
                                    .with_header(json_header);
                                let _ = request.respond(res);
                            }
                        }
                    } else {
                        let res = Response::from_string("{\"error\": \"Bad format, expected /ptr/:hash_a/:hash_b\"}")
                            .with_status_code(StatusCode(400))
                            .with_header(json_header);
                        let _ = request.respond(res);
                    }
                } else {
                    let res = Response::from_string("{\"error\": \"Endpoint not found\"}")
                        .with_status_code(StatusCode(404))
                        .with_header(json_header);
                    let _ = request.respond(res);
                }
            }
        }
    }

    Ok(())
}
