//! Build script: resolve the user-facing version without hardcoding (Mandate 4).
//!
//! Priority: `ATTUNE_VERSION` env (set by tools/builder.py / CI) →
//! `apps/attune_app/pubspec.yaml` `version:` (single source of truth shared with
//! the Android build, ADR-063) → crate version (`CARGO_PKG_VERSION`).

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=ATTUNE_VERSION");

    if std::env::var("ATTUNE_VERSION").map(|v| !v.trim().is_empty()).unwrap_or(false) {
        return; // already provided; option_env!() picks it up directly
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let pubspec = manifest_dir.join("../../apps/attune_app/pubspec.yaml");
    println!("cargo:rerun-if-changed={}", pubspec.display());

    let version = std::fs::read_to_string(&pubspec).ok().and_then(|text| {
        text.lines()
            .find_map(|l| l.strip_prefix("version:"))
            .map(|v| v.trim().split('+').next().unwrap_or("").trim().to_string())
            .filter(|v| !v.is_empty())
    });

    match version {
        Some(v) => println!("cargo:rustc-env=ATTUNE_VERSION={v}"),
        None => println!(
            "cargo:warning=Could not read version from {}; falling back to CARGO_PKG_VERSION",
            pubspec.display()
        ),
    }
}
