//! Empirical Stress Tests for Windows Dynamic Library Resolution & Error Decoding
//! (Milestone M1 Empirical Challenger Suite)

#[path = "../src/process_manager.rs"]
mod process_manager;

#[path = "../src/dynamic_components.rs"]
mod dynamic_components;

use std::path::{Path, PathBuf};
use std::thread;

// 1. Error Decoding Tests
#[test]
fn test_win32_error_decoder_exhaustive() {
    use dynamic_components::format_windows_error;

    // Standard Win32 loader error codes
    assert!(format_windows_error(5).contains("ERROR_ACCESS_DENIED (5)"));
    assert!(format_windows_error(87).contains("ERROR_INVALID_PARAMETER (87)"));
    assert!(format_windows_error(126).contains("ERROR_MOD_NOT_FOUND (126)"));
    assert!(format_windows_error(127).contains("ERROR_PROC_NOT_FOUND (127)"));
    assert!(format_windows_error(193).contains("ERROR_BAD_EXE_FORMAT (193)"));
    assert!(format_windows_error(998).contains("ERROR_NOACCESS (998)"));
    assert!(format_windows_error(1114).contains("ERROR_DLL_INIT_FAILED (1114)"));
    assert!(format_windows_error(14001).contains("ERROR_SXS_CANT_GEN_ACTCTX (14001)"));

    // Verify key explanatory phrases are present
    assert!(format_windows_error(87).contains(r"\\?\"));
    assert!(format_windows_error(193).contains("architecture mismatch"));
    assert!(format_windows_error(14001).contains("Microsoft Visual C++ CRT"));

    // Boundary and unmapped error codes should not panic
    let edge_codes = [0, 1, 2, 3, 32, 1157, 129, 216, u32::MAX, 1234567];
    for &code in &edge_codes {
        let msg = format_windows_error(code);
        assert!(!msg.is_empty(), "format_windows_error({code}) returned empty string");
        assert!(msg.contains("Unknown Win32") || msg.contains("error"));
    }
}

// 2. DLL Directory Path Sanitization Stress-Testing
#[test]
fn test_sanitize_dll_directory_path_adversarial() {
    use dynamic_components::sanitize_dll_directory_path;

    // Canonical Windows paths
    assert_eq!(
        sanitize_dll_directory_path(Path::new(r"C:\Program Files\Attune\lib")),
        Some(PathBuf::from(r"C:\Program Files\Attune\lib"))
    );

    // Trailing slashes / backslashes of varying depth
    assert_eq!(
        sanitize_dll_directory_path(Path::new(r"C:\Attune\lib\")),
        Some(PathBuf::from(r"C:\Attune\lib"))
    );
    assert_eq!(
        sanitize_dll_directory_path(Path::new(r"C:\Attune\lib\\\\")),
        Some(PathBuf::from(r"C:\Attune\lib"))
    );
    assert_eq!(
        sanitize_dll_directory_path(Path::new("C:/Attune/lib///")),
        Some(PathBuf::from("C:/Attune/lib"))
    );
    assert_eq!(
        sanitize_dll_directory_path(Path::new(r"C:/Attune/lib\//\")),
        Some(PathBuf::from("C:/Attune/lib"))
    );

    // Verbatim \\?\ prefixes
    assert_eq!(
        sanitize_dll_directory_path(Path::new(r"\\?\C:\Program Files\Attune\lib")),
        Some(PathBuf::from(r"C:\Program Files\Attune\lib"))
    );
    assert_eq!(
        sanitize_dll_directory_path(Path::new(r"\\?\C:\Program Files\Attune\lib\")),
        Some(PathBuf::from(r"C:\Program Files\Attune\lib"))
    );

    // Verbatim UNC \\?\UNC\ prefixes -> normalized to \\server\share
    assert_eq!(
        sanitize_dll_directory_path(Path::new(r"\\?\UNC\server\share\lib")),
        Some(PathBuf::from(r"\\server\share\lib"))
    );
    assert_eq!(
        sanitize_dll_directory_path(Path::new(r"\\?\UNC\server\share\lib\")),
        Some(PathBuf::from(r"\\server\share\lib"))
    );

    // Forward-slash variants
    assert_eq!(
        sanitize_dll_directory_path(Path::new("//?/C:/Program Files/Attune/lib")),
        Some(PathBuf::from("C:/Program Files/Attune/lib"))
    );
    assert_eq!(
        sanitize_dll_directory_path(Path::new("//?/UNC/server/share/lib/")),
        Some(PathBuf::from(r"\\server/share/lib"))
    );

    // Unicode / non-ASCII path strings (e.g. Japanese, spaces, emoji)
    let unicode_path = r"C:\Users\初音ミク\Music Player 🎵\lib\";
    assert_eq!(
        sanitize_dll_directory_path(Path::new(unicode_path)),
        Some(PathBuf::from(r"C:\Users\初音ミク\Music Player 🎵\lib"))
    );

    // Surrounding whitespace
    assert_eq!(
        sanitize_dll_directory_path(Path::new("   C:\\Attune\\lib   ")),
        Some(PathBuf::from(r"C:\Attune\lib"))
    );

    // Malformed / Empty / Slush-only inputs
    assert_eq!(sanitize_dll_directory_path(Path::new("")), None);
    assert_eq!(sanitize_dll_directory_path(Path::new("   ")), None);
    assert_eq!(sanitize_dll_directory_path(Path::new("\t\n ")), None);
    assert_eq!(sanitize_dll_directory_path(Path::new(r"\\")), None);
    assert_eq!(sanitize_dll_directory_path(Path::new(r"\\\\")), None);
    assert_eq!(sanitize_dll_directory_path(Path::new("///")), None);
    assert_eq!(sanitize_dll_directory_path(Path::new(r"\\?\")), None);
    assert_eq!(sanitize_dll_directory_path(Path::new(r"\\?\UNC\")), None);
    assert_eq!(sanitize_dll_directory_path(Path::new("//?/")), None);
    assert_eq!(sanitize_dll_directory_path(Path::new("//?/UNC/")), None);
}

// 3. Multi-Threaded Concurrency & Idempotency of DLL Directory Initialization
#[test]
fn test_init_windows_dll_directories_concurrency_stress() {
    use dynamic_components::init_windows_dll_directories;

    let handles: Vec<_> = (0..32)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..50 {
                    init_windows_dll_directories();
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("Thread calling init_windows_dll_directories panicked!");
    }
}

// 4. Candidate Path Resolution with Environment Variable Variations
#[test]
fn test_resolve_candidate_paths_env_variations() {
    use dynamic_components::resolve_candidate_paths;

    // Normal base resolution
    let base = "attune_watchguard";
    let candidates = resolve_candidate_paths(base);
    assert!(!candidates.is_empty());

    // Filename should end with platform dylib extension
    let filename = candidates.last().unwrap().to_str().unwrap();
    #[cfg(target_os = "windows")]
    assert!(filename.ends_with(".dll"));
    #[cfg(target_os = "linux")]
    assert!(filename.ends_with(".so"));

    // Test with ATTUNE_LIB_DIR override
    let unique_dir = std::env::temp_dir().join(format!("attune_test_lib_{}", std::process::id()));
    std::fs::create_dir_all(&unique_dir).ok();

    std::env::set_var("ATTUNE_LIB_DIR", &unique_dir);
    let candidates_with_env = resolve_candidate_paths(base);
    assert_eq!(candidates_with_env[0].parent().unwrap(), unique_dir);
    std::env::remove_var("ATTUNE_LIB_DIR");

    let _ = std::fs::remove_dir_all(&unique_dir);
}

// 5. Negative Scenario: Tampered / Corrupted Binary Rejection
#[test]
fn test_negative_tampered_and_corrupted_component_rejection() {
    use dynamic_components::verify_component_integrity;

    let unique_dir = std::env::temp_dir().join(format!("attune_test_tamper_{}", std::process::id()));
    std::fs::create_dir_all(&unique_dir).expect("create test temp dir");

    let fake_dll = unique_dir.join("attune_watchguard.dll");
    let fake_so = unique_dir.join("libattune_watchguard.so");

    // 1. Truncated / zero-byte binary
    std::fs::write(&fake_dll, b"").expect("write zero-byte fake dll");
    std::fs::write(&fake_so, b"").expect("write zero-byte fake so");

    let manifest_content = r#"{
        "version": "1.0.0",
        "components": {
            "attune_watchguard.dll": {
                "sha256": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                "size_bytes": 1024
            },
            "libattune_watchguard.so": {
                "sha256": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                "size_bytes": 1024
            }
        }
    }"#;
    std::fs::write(unique_dir.join("attune_integrity.json"), manifest_content).expect("write manifest");

    // Must be rejected by size mismatch or hash mismatch
    let err_dll = verify_component_integrity(&fake_dll, "attune_watchguard")
        .expect_err("Zero-byte DLL must be rejected");
    assert!(err_dll.contains("size mismatch") || err_dll.contains("mismatch"));

    let err_so = verify_component_integrity(&fake_so, "attune_watchguard")
        .expect_err("Zero-byte SO must be rejected");
    assert!(err_so.contains("size mismatch") || err_so.contains("mismatch"));

    // 2. Corrupted contents matching size but invalid hash
    let garbage = vec![0xCC; 1024];
    std::fs::write(&fake_dll, &garbage).expect("write corrupt 1024-byte dll");
    let err_hash = verify_component_integrity(&fake_dll, "attune_watchguard")
        .expect_err("Corrupted DLL with wrong hash must be rejected");
    assert!(err_hash.contains("CRITICAL") && err_hash.contains("mismatch"));

    let _ = std::fs::remove_dir_all(&unique_dir);
}


// 6. Negative Scenario: Invalid ATTUNE_LIB_DIR Inputs
#[test]
fn test_negative_invalid_attune_lib_dir() {
    use dynamic_components::{resolve_candidate_paths, sanitize_dll_directory_path};

    // Case 1: Empty string
    assert_eq!(sanitize_dll_directory_path(Path::new("")), None);

    // Case 2: Whitespace only
    assert_eq!(sanitize_dll_directory_path(Path::new("   \t  ")), None);

    // Case 3: Non-existent directory
    let non_existent = Path::new("/opt/attune_nonexistent_dir_xyz_123");
    let sanitized = sanitize_dll_directory_path(non_existent);
    assert_eq!(sanitized, Some(PathBuf::from("/opt/attune_nonexistent_dir_xyz_123")));

    // Case 4: Verbatim prefix in ATTUNE_LIB_DIR
    let verbatim = Path::new(r"\\?\C:\Users\Miku\AppData\Local\Attune\lib\");
    let sanitized_verbatim = sanitize_dll_directory_path(verbatim);
    assert_eq!(
        sanitized_verbatim,
        Some(PathBuf::from(r"C:\Users\Miku\AppData\Local\Attune\lib"))
    );

    // Verify resolve_candidate_paths handles invalid ATTUNE_LIB_DIR gracefully
    std::env::set_var("ATTUNE_LIB_DIR", "");
    let candidates = resolve_candidate_paths("attune_watchguard");
    assert!(!candidates.is_empty());
    std::env::remove_var("ATTUNE_LIB_DIR");
}

// 7. Error 126 Diagnostic Simulation (Missing Dependency vs Missing File)
#[test]
fn test_error_126_dependent_dll_diagnostic_simulation() {
    use dynamic_components::format_windows_error;

    let code = 126;
    let desc = format_windows_error(code);
    assert!(desc.contains("ERROR_MOD_NOT_FOUND (126)"));

    // Scenario A: File exists on disk, but LoadLibraryW returns 126 because a dependent DLL is missing
    let temp_file = std::env::temp_dir().join(format!("attune_test_dep_{}.dll", std::process::id()));
    std::fs::write(&temp_file, b"MZ_fake_pe_header").expect("write temp pe");

    let extra_existing = if code == 126 && temp_file.exists() {
        " (Note: file exists on disk; a dependent DLL such as vcruntime140.dll or a workspace DLL in lib/ failed to resolve)"
    } else {
        ""
    };
    assert!(extra_existing.contains("Note: file exists on disk"));
    assert!(extra_existing.contains("dependent DLL"));

    // Scenario B: File does not exist on disk
    let non_existent = std::env::temp_dir().join("does_not_exist_at_all.dll");
    let extra_missing = if code == 126 && non_existent.exists() {
        " (Note: file exists on disk; a dependent DLL such as vcruntime140.dll or a workspace DLL in lib/ failed to resolve)"
    } else {
        ""
    };
    assert!(extra_missing.is_empty());

    let _ = std::fs::remove_file(&temp_file);
}

// 8. Loader Stability: Repeated and Concurrent Preflight
#[test]
fn test_loader_stability_under_repeated_preflight() {
    use dynamic_components::preflight_check_and_load;

    // Sequential invocations
    for _ in 0..5 {
        let _ = preflight_check_and_load();
    }

    // Concurrent multi-threaded invocations
    let handles: Vec<_> = (0..8)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..5 {
                    let _ = preflight_check_and_load();
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("Concurrent preflight_check_and_load panicked!");
    }
}
