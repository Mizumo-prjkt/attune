use attune_core::cache::{CacheDatabase, CachePack, StemBridgeRecord};
use attune_core::schema::{
    AnalysisTier, CueZone, DeckAutomation, DspAutomationKeyframe, Frames20Hz, GlobalSummary,
    TrackAnalysisProfile, TrackId, TransitionArchetype, TransitionRecipe, TransitionTiming,
    ZoneType,
};

fn create_mock_tap(slug: &str, bpm: f32, key: &str) -> TrackAnalysisProfile {
    TrackAnalysisProfile {
        schema_version: 1,
        track_id: TrackId {
            audio_hash: format!("hash_{}", slug),
            duration_ms: 180000,
            canonical_slug: slug.to_string(),
        },
        analysis_tier: AnalysisTier::FullLocal,
        global_summary: GlobalSummary {
            nominal_bpm: bpm,
            bpm_stability: 0.9,
            primary_key: key.to_string(),
            mean_aggression_index: 0.65,
            macro_vibe_family: "AGGRESSIVE_ROCK_PUNK_JROCK".to_string(),
            dominant_instruments: vec!["inst_synth_electronic".to_string()],
        },
        cue_zones: vec![CueZone {
            zone_type: ZoneType::ENTRY,
            start_ms: 1000,
            end_ms: 45000,
            hop_ms: 50,
            beats_ms: vec![1000, 1350],
            downbeats_ms: vec![1000],
            phrase_boundaries_ms: vec![1000],
            sustained_pitch_regions: vec![],
            frames_20hz: Frames20Hz {
                f0_midi_cents: vec![0.0; 10],
                chroma_12: vec![[0.0; 12]; 10],
                energy_4band_db: vec![[0.0; 4]; 10],
                aggression_curve: vec![0.5; 10],
                instrument_activations: vec![vec![0.0; 6]; 10],
            },
        }],
    }
}

fn create_mock_ptr(hash_a: &str, hash_b: &str, score: f32) -> TransitionRecipe {
    TransitionRecipe {
        schema_version: 1,
        track_a_hash: hash_a.to_string(),
        track_b_hash: hash_b.to_string(),
        archetype: TransitionArchetype::RiffHandoff,
        confidence_score: score,
        timing: TransitionTiming {
            track_a_transition_start_ms: 156000,
            track_a_exit_end_ms: 158000,
            track_b_entry_start_ms: 1000,
            track_b_native_lock_ms: 3000,
        },
        optional_stem_bridge_uri: None,
        dsp_automation_keyframes: vec![
            DspAutomationKeyframe {
                offset_ms: 0,
                deck_a: DeckAutomation::default(),
                deck_b: DeckAutomation {
                    gain_db: -18.0,
                    ..Default::default()
                },
            },
            DspAutomationKeyframe {
                offset_ms: 1400,
                deck_a: DeckAutomation {
                    gain_db: -60.0,
                    reverb_freeze_send: 0.45,
                    ..Default::default()
                },
                deck_b: DeckAutomation::default(),
            },
        ],
    }
}

#[test]
fn test_cache_database_crud_operations() {
    let db = CacheDatabase::open_in_memory().expect("Should open in-memory SQLite");

    let tap_a = create_mock_tap("rabbit_hole", 171.4, "C Major");
    let tap_b = create_mock_tap("not_a_devil", 149.3, "D Minor");

    // Store profiles
    db.store_tap(&tap_a).expect("Store tap A");
    db.store_tap(&tap_b).expect("Store tap B");

    assert!(db.has_tap("hash_rabbit_hole").unwrap());
    assert!(db.has_tap("hash_not_a_devil").unwrap());
    assert!(!db.has_tap("non_existent").unwrap());

    // Retrieve profile
    let retrieved_a = db.get_tap("hash_rabbit_hole").unwrap().expect("Should find tap A");
    assert_eq!(retrieved_a.track_id.canonical_slug, "rabbit_hole");
    assert_eq!(retrieved_a.global_summary.nominal_bpm, 171.4);

    // Store and retrieve transition recipe
    let ptr_ab = create_mock_ptr("hash_rabbit_hole", "hash_not_a_devil", 0.92);
    db.store_ptr(&ptr_ab).expect("Store ptr");

    assert!(db.has_ptr("hash_rabbit_hole", "hash_not_a_devil").unwrap());
    let retrieved_ptr = db
        .get_ptr("hash_rabbit_hole", "hash_not_a_devil")
        .unwrap()
        .expect("Should find ptr");
    assert_eq!(retrieved_ptr.confidence_score, 0.92);
    assert_eq!(retrieved_ptr.archetype, TransitionArchetype::RiffHandoff);

    // Store stem bridge
    let bridge = StemBridgeRecord {
        track_a_hash: "hash_rabbit_hole".to_string(),
        track_b_hash: "hash_not_a_devil".to_string(),
        stem_type: "vocal".to_string(),
        uri: "stems/vocal_bridge_ab.wav".to_string(),
        duration_ms: 2000,
    };
    db.store_stem_bridge(&bridge).expect("Store stem bridge");

    let retrieved_bridge = db
        .get_stem_bridge("hash_rabbit_hole", "hash_not_a_devil", "vocal")
        .unwrap()
        .expect("Should find stem bridge");
    assert_eq!(retrieved_bridge.uri, "stems/vocal_bridge_ab.wav");

    // Check stats
    let stats = db.get_stats().expect("Get stats");
    assert_eq!(stats.tracks_count, 2);
    assert_eq!(stats.transitions_count, 1);
    assert_eq!(stats.stem_bridges_count, 1);
}

#[test]
fn test_cache_pack_export_and_import() {
    let source_db = CacheDatabase::open_in_memory().unwrap();

    let tap1 = create_mock_tap("track_1", 128.0, "F Major");
    let tap2 = create_mock_tap("track_2", 130.0, "G Major");
    let tap3 = create_mock_tap("track_3", 175.0, "A Minor");

    source_db.store_tap(&tap1).unwrap();
    source_db.store_tap(&tap2).unwrap();
    source_db.store_tap(&tap3).unwrap();

    let ptr12 = create_mock_ptr("hash_track_1", "hash_track_2", 0.88);
    let ptr23 = create_mock_ptr("hash_track_2", "hash_track_3", 0.75);

    source_db.store_ptr(&ptr12).unwrap();
    source_db.store_ptr(&ptr23).unwrap();

    // Export pack
    let pack = CachePack::from_db(&source_db).expect("Export pack");
    assert_eq!(pack.taps.len(), 3);
    assert_eq!(pack.ptrs.len(), 2);

    // Roundtrip to temp file
    let tmp_path = std::env::temp_dir().join(format!("attune_test_pack_{}.json", std::process::id()));
    pack.save_to_file(&tmp_path).expect("Save pack to file");

    let loaded_pack = CachePack::load_from_file(&tmp_path).expect("Load pack from file");
    assert_eq!(loaded_pack.taps.len(), 3);
    assert_eq!(loaded_pack.ptrs.len(), 2);

    // Import into fresh database
    let dest_db = CacheDatabase::open_in_memory().unwrap();
    let stats = loaded_pack.import_into_db(&dest_db).expect("Import pack");
    assert_eq!(stats.taps_imported, 3);
    assert_eq!(stats.ptrs_imported, 2);

    assert!(dest_db.has_tap("hash_track_1").unwrap());
    assert!(dest_db.has_ptr("hash_track_1", "hash_track_2").unwrap());

    // Clean up
    let _ = std::fs::remove_file(tmp_path);
}

#[test]
fn test_top_transitions_ranking() {
    let db = CacheDatabase::open_in_memory().unwrap();

    let ptr1 = create_mock_ptr("source", "dest_c", 0.95);
    let ptr2 = create_mock_ptr("source", "dest_a", 0.82);
    let ptr3 = create_mock_ptr("source", "dest_b", 0.91);

    db.store_ptr(&ptr1).unwrap();
    db.store_ptr(&ptr2).unwrap();
    db.store_ptr(&ptr3).unwrap();

    let top2 = db.top_transitions_for_track("source", 2).unwrap();
    assert_eq!(top2.len(), 2);
    assert_eq!(top2[0].track_b_hash, "dest_c"); // 0.95 highest
    assert_eq!(top2[1].track_b_hash, "dest_b"); // 0.91 second highest
}
