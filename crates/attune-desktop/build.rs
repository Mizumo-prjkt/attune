//! Build script: resolve the user-facing version dynamically without hardcoding (Mandate 4).
//!
//! Resolution Priority:
//! 1. `ATTUNE_VERSION` environment variable (set by tools/builder.py / CI)
//! 2. `git describe --tags --always --dirty` (runtime dynamic version derived from repo state)
//! 3. `apps/attune_app/pubspec.yaml` `version:` (single source of truth shared with Android)
//! 4. Fallback to `CARGO_PKG_VERSION`

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=ATTUNE_VERSION");

    // On Linux, emit --as-needed and relative $ORIGIN RUNPATH so the binary resolves bundled
    // shared libraries in ./lib and ../usr/lib and never hard-depends on distro-specific
    // SONAMEs like libxdo.so.3 vs libxdo.so.4.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg=-Wl,--as-needed");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN:$ORIGIN/lib:$ORIGIN/../lib:$ORIGIN/../usr/lib");
    }

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=kernel32");
    }

    if let Ok(v) = std::env::var("ATTUNE_VERSION") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            println!("cargo:rustc-env=ATTUNE_VERSION={trimmed}");
            return;
        }
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let pubspec = manifest_dir.join("../../apps/attune_app/pubspec.yaml");
    println!("cargo:rerun-if-changed={}", pubspec.display());

    // 1. Try Git describe for dynamic continuous integration versioning
    let git_desc = Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .current_dir(&manifest_dir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    let clean = s.strip_prefix('v').unwrap_or(&s).to_string();
                    return Some(clean);
                }
            }
            None
        });

    if let Some(ref gver) = git_desc {
        println!("cargo:rustc-env=ATTUNE_VERSION={gver}");
        return;
    }

    // 2. Try pubspec.yaml version
    let version = std::fs::read_to_string(&pubspec).ok().and_then(|text| {
        text.lines()
            .find_map(|l| l.strip_prefix("version:"))
            .map(|v| v.trim().split('+').next().unwrap_or("").trim().to_string())
            .filter(|v| !v.is_empty())
    });

    match version {
        Some(v) => println!("cargo:rustc-env=ATTUNE_VERSION={v}"),
        None => println!(
            "cargo:warning=Could not read version from git or {}; falling back to CARGO_PKG_VERSION",
            pubspec.display()
        ),
    }
}
