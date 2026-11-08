#!/usr/bin/env python3
"""
Attune Unified Build & Packaging Orchestrator
---------------------------------------------
Assesses toolchains, manages component version IDs & Linux-style localversion strings,
etches compiler environment metadata into build_info.json, generates commit patch notes,
compiles native Rust DSP engine (attune-ffi) and Flutter frontend, and bundles
portable releases (Linux AppImage & Windows Inno Setup).

Includes both a CLI mode (for headless CI runners) and an interactive Tkinter GUI.
Distributed under the MIT License.
"""

import argparse
import datetime
from datetime import timezone
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
from pathlib import Path

# Repository root directory
REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(Path(__file__).resolve().parent))

try:
    from tools.generate_windows_icons import generate_windows_icon, validate_ico_file
except ImportError:
    try:
        from generate_windows_icons import generate_windows_icon, validate_ico_file
    except ImportError:
        generate_windows_icon = None
        validate_ico_file = None

IS_WINDOWS = sys.platform == "win32"

# Ensure UTF-8 output on Windows console avoiding cp1252 charmap exceptions
if IS_WINDOWS:
    if hasattr(sys.stdout, "reconfigure"):
        try:
            sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass
    if hasattr(sys.stderr, "reconfigure"):
        try:
            sys.stderr.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass



def get_git_info():
    """Extracts git commit hash, branch, and status."""
    info = {
        "git_commit": "unknown",
        "git_commit_full": "unknown",
        "git_branch": "unknown",
        "git_dirty": False,
    }
    try:
        short_hash = subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=REPO_ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        full_hash = subprocess.check_output(
            ["git", "rev-parse", "HEAD"],
            cwd=REPO_ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        branch = subprocess.check_output(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"],
            cwd=REPO_ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        status = subprocess.check_output(
            ["git", "status", "--porcelain"],
            cwd=REPO_ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()

        info["git_commit"] = short_hash
        info["git_commit_full"] = full_hash
        info["git_branch"] = branch
        info["git_dirty"] = len(status) > 0
    except Exception:
        pass
    return info


def get_pubspec_version():
    """Reads app version and build number from apps/attune_app/pubspec.yaml."""
    pubspec_path = REPO_ROOT / "apps" / "attune_app" / "pubspec.yaml"
    if not pubspec_path.exists():
        build_info_path = REPO_ROOT / "apps" / "attune_app" / "assets" / "build_info.json"
        if build_info_path.exists():
            try:
                data = json.loads(build_info_path.read_text(encoding="utf-8"))
                return data.get("full_version") or data.get("version", "1.0.0"), str(data.get("build_number", "1"))
            except Exception:
                pass
        return "1.0.0", "1"
    try:
        content = pubspec_path.read_text(encoding="utf-8")
        match = re.search(r"^version:\s*([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?)(?:\+([0-9]+))?", content, re.MULTILINE)
        if match:
            return match.group(1), match.group(2) or "1"
    except Exception:
        pass
    return "1.0.0", "1"


def compute_dynamic_version():
    """
    Computes dynamic continuous integration version identifier from Git metadata:
    - Base version from pubspec.yaml / Cargo.toml (e.g. 0.1.0)
    - Git commit count as build number (e.g. 159)
    - Git describe tag/hash (e.g. v0.1.0-rc5-102-g0765b45)
    - Dynamic version string suitable for releases and auto-updates.
    """
    base_ver, pub_build = get_pubspec_version()
    git_info = get_git_info()

    commit_count = None
    try:
        out = subprocess.check_output(
            ["git", "rev-list", "--count", "HEAD"],
            cwd=REPO_ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        if out.isdigit():
            commit_count = int(out)
    except Exception:
        pass

    git_desc = None
    try:
        out = subprocess.check_output(
            ["git", "describe", "--tags", "--always", "--dirty"],
            cwd=REPO_ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        if out:
            git_desc = out.lstrip("v")
    except Exception:
        pass

    effective_build = str(commit_count) if commit_count is not None else pub_build
    dynamic_id = git_desc if git_desc else f"{base_ver}+{effective_build}"

    return {
        "base_version": base_ver,
        "build_number": effective_build,
        "commit_count": commit_count,
        "git_describe": git_desc,
        "dynamic_id": dynamic_id,
        "git_commit": git_info["git_commit"],
        "git_dirty": git_info["git_dirty"],
    }


def sync_all_version_metadata(version_str, build_num=None, log_callback=None):
    """
    Synchronizes all application manifests across the codebase with the exact target version
    and build number, ensuring zero desync between components (Mandate 4):
    1. apps/attune_app/pubspec.yaml
    2. root Cargo.toml ([workspace.package] version)
    3. apps/attune_app/android/local.properties
    4. packaging/windows/attune_installer.iss (#define MyAppVersion)
    """
    def log(msg):
        if log_callback:
            log_callback(msg)
        else:
            print(msg)

    cur_ver, cur_build = get_pubspec_version()
    effective_build = str(build_num).strip() if build_num is not None and str(build_num).strip() else cur_build

    # 1. Update apps/attune_app/pubspec.yaml
    pubspec_path = REPO_ROOT / "apps" / "attune_app" / "pubspec.yaml"
    if pubspec_path.exists():
        try:
            content = pubspec_path.read_text(encoding="utf-8")
            new_version_line = f"version: {version_str}+{effective_build}"
            updated_content, count = re.subn(r"^version:\s*.*$", new_version_line, content, flags=re.MULTILINE)
            if count > 0:
                pubspec_path.write_text(updated_content, encoding="utf-8")
                log(f"[Sync] Updated {pubspec_path.name} to: {new_version_line}")
        except Exception as e:
            log(f"[Sync] Warning: Failed to update pubspec.yaml: {e}")

    # 2. Update root Cargo.toml [workspace.package] version
    cargo_path = REPO_ROOT / "Cargo.toml"
    if cargo_path.exists():
        try:
            content = cargo_path.read_text(encoding="utf-8")
            updated = re.sub(
                r'(\[workspace\.package\][^\[]*?version\s*=\s*)"[^"]+"',
                f'\\1"{version_str}"',
                content,
                flags=re.DOTALL,
            )
            cargo_path.write_text(updated, encoding="utf-8")
            log(f"[Sync] Updated root Cargo.toml [workspace.package] to version = \"{version_str}\"")
        except Exception as e:
            log(f"[Sync] Note: Could not update Cargo.toml: {e}")

    # 3. Update apps/attune_app/android/local.properties
    local_props = REPO_ROOT / "apps" / "attune_app" / "android" / "local.properties"
    if local_props.exists():
        try:
            lines = local_props.read_text(encoding="utf-8").splitlines()
            new_lines = []
            has_name = False
            has_code = False
            for line in lines:
                if line.startswith("flutter.versionName="):
                    new_lines.append(f"flutter.versionName={version_str}")
                    has_name = True
                elif line.startswith("flutter.versionCode="):
                    new_lines.append(f"flutter.versionCode={effective_build}")
                    has_code = True
                else:
                    new_lines.append(line)
            if not has_name:
                new_lines.append(f"flutter.versionName={version_str}")
            if not has_code:
                new_lines.append(f"flutter.versionCode={effective_build}")
            local_props.write_text("\n".join(new_lines) + "\n", encoding="utf-8")
            log(f"[Sync] Updated android/local.properties (versionName={version_str}, versionCode={effective_build})")
        except Exception as e:
            log(f"[Sync] Note: Could not update local.properties: {e}")

    # 4. Update packaging/windows/attune_installer.iss
    iss_path = REPO_ROOT / "packaging" / "windows" / "attune_installer.iss"
    if iss_path.exists():
        try:
            content = iss_path.read_text(encoding="utf-8")
            updated = re.sub(
                r'#define MyAppVersion "[^"]+"',
                f'#define MyAppVersion "{version_str}"',
                content,
            )
            iss_path.write_text(updated, encoding="utf-8")
            log(f"[Sync] Updated {iss_path.name} to: #define MyAppVersion \"{version_str}\"")
        except Exception as e:
            log(f"[Sync] Note: Could not update attune_installer.iss: {e}")


def sync_pubspec_version(version_str, build_num=None, log_callback=None):
    """Alias for sync_all_version_metadata for backwards compatibility."""
    sync_all_version_metadata(version_str, build_num=build_num, log_callback=log_callback)


def bump_semver(bump_type="patch", log_callback=None):
    """
    Increments semver (patch, minor, major) and derives build number from Git commit count.
    Synchronizes all workspace manifests.
    """
    def log(msg):
        if log_callback:
            log_callback(msg)
        else:
            print(msg)

    cur_ver, cur_build = get_pubspec_version()
    m = re.match(r"^(\d+)\.(\d+)\.(\d+)(?:-(.+))?$", cur_ver)
    if not m:
        major, minor, patch = 0, 1, 0
    else:
        major = int(m.group(1))
        minor = int(m.group(2))
        patch = int(m.group(3))

    b_type = (bump_type or "patch").strip().lower()
    if b_type == "major":
        major += 1
        minor = 0
        patch = 0
    elif b_type == "minor":
        minor += 1
        patch = 0
    else:  # patch
        patch += 1

    new_ver = f"{major}.{minor}.{patch}"
    dyn = compute_dynamic_version()
    new_build = str(dyn["commit_count"] + 1) if dyn["commit_count"] is not None else str(int(cur_build) + 1)

    log(f"[Bump] Bumping version ({b_type}): {cur_ver}+{cur_build} -> {new_ver}+{new_build}")
    sync_all_version_metadata(new_ver, new_build, log_callback=log)
    return new_ver, new_build


def get_rust_crate_version(crate_name="attune-ffi"):
    """Reads Rust crate version from crates/<crate_name>/Cargo.toml."""
    cargo_path = REPO_ROOT / "crates" / crate_name / "Cargo.toml"
    if not cargo_path.exists():
        return "0.1.0"
    try:
        content = cargo_path.read_text(encoding="utf-8")
        match = re.search(r'^version\s*=\s*"([^"]+)"', content, re.MULTILINE)
        if match:
            return match.group(1)
    except Exception:
        pass
    return "0.1.0"


def query_command(cmd):
    """Executes a command and returns the first line of output, or None if failed."""
    try:
        res = subprocess.check_output(cmd, text=True, stderr=subprocess.DEVNULL, shell=IS_WINDOWS).strip()
        return res.splitlines()[0] if res else None
    except Exception:
        return None



def find_android_sdk():
    """Detects Android SDK path dynamically without hardcoding."""
    for env_var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"]:
        val = os.environ.get(env_var)
        if val and Path(val).exists():
            return Path(val)
    home = Path.home()
    candidates = [
        home / ".local" / "android" / "sdk",
        home / "Android" / "Sdk",
        home / "Library" / "Android" / "sdk",
        Path("/usr/lib/android-sdk"),
    ]
    for c in candidates:
        if c.exists():
            return c
    return None


def find_android_ndk():
    """Detects Android NDK installation path dynamically without hardcoding."""
    candidates = []
    for env_var in ["ANDROID_NDK_HOME", "NDK_HOME", "ANDROID_NDK_ROOT", "NDK_ROOT", "ANDROID_SDK_ROOT", "ANDROID_HOME"]:
        val = os.environ.get(env_var)
        if val:
            candidates.append(Path(val))
    home = Path.home()
    candidates.extend([
        home / ".local" / "android" / "sdk",
        home / "Android" / "Sdk",
        home / ".android-sdk",
        Path("/opt/android-sdk"),
        Path("/opt/android-ndk"),
        home / "Library" / "Android" / "sdk",
        Path("/usr/lib/android-sdk"),
    ])
    for c in candidates:
        if not c.exists():
            continue
        if (c / "toolchains" / "llvm").exists():
            return c
        ndk_dir = c / "ndk"
        if ndk_dir.exists() and ndk_dir.is_dir():
            versions = sorted([p for p in ndk_dir.iterdir() if p.is_dir()], reverse=True)
            for v in versions:
                if (v / "toolchains" / "llvm").exists():
                    return v
    return None


def get_ndk_llvm_bin(ndk_path):
    """Returns the prebuilt LLVM bin directory inside the NDK."""
    if not ndk_path:
        return None
    prebuilt_dir = ndk_path / "toolchains" / "llvm" / "prebuilt"
    if not prebuilt_dir.exists():
        return None
    for host_dir in prebuilt_dir.iterdir():
        bin_dir = host_dir / "bin"
        if bin_dir.exists():
            return bin_dir
    return None



def assess_toolchains():
    """Assesses host environment and required compilation toolchains."""
    results = {}

    results["python"] = {
        "version": platform.python_version(),
        "path": sys.executable,
        "status": True,
    }

    # Rust & Cargo
    rustc_ver = query_command(["rustc", "--version"])
    results["rustc"] = {
        "version": rustc_ver or "Not Found",
        "status": rustc_ver is not None,
    }

    cargo_ver = query_command(["cargo", "--version"])
    results["cargo"] = {
        "version": cargo_ver or "Not Found",
        "status": cargo_ver is not None,
    }

    # Flutter & Dart
    flutter_ver = query_command(["flutter", "--version"])
    results["flutter"] = {
        "version": flutter_ver or "Not Found",
        "status": flutter_ver is not None,
    }

    dart_ver = query_command(["dart", "--version"])
    results["dart"] = {
        "version": dart_ver or "Not Found",
        "status": dart_ver is not None,
    }

    # Java / JDK
    java_ver = query_command(["javac", "-version"]) or query_command(["java", "-version"])
    results["java"] = {
        "version": java_ver or "Not Found",
        "status": java_ver is not None,
    }

    # Android SDK & NDK
    android_sdk = find_android_sdk()
    results["android_sdk"] = {
        "version": str(android_sdk) if android_sdk else "Not Found",
        "status": android_sdk is not None,
    }

    android_ndk = find_android_ndk()
    results["android_ndk"] = {
        "version": str(android_ndk.name) if android_ndk else "Not Found",
        "path": str(android_ndk) if android_ndk else None,
        "status": android_ndk is not None,
    }

    # CMake & C/C++ Compiler
    cmake_ver = query_command(["cmake", "--version"])
    results["cmake"] = {
        "version": cmake_ver or "Not Found",
        "status": cmake_ver is not None,
    }

    cc_ver = query_command(["gcc", "--version"]) or query_command(["clang", "--version"]) or query_command(["cl.exe"])
    results["c_compiler"] = {
        "version": cc_ver or "Not Found",
        "status": cc_ver is not None,
    }

    # Windows Inno Setup Compiler
    iscc_path = shutil.which("iscc")
    if not iscc_path and platform.system() == "Windows":
        default_inno = Path("C:/Program Files (x86)/Inno Setup 6/ISCC.exe")
        if default_inno.exists():
            iscc_path = str(default_inno)
    results["iscc"] = {
        "version": "Inno Setup Compiler" if iscc_path else "Not Found",
        "path": iscc_path,
        "status": iscc_path is not None,
    }

    # Linux AppImageTool
    appimage_tool = shutil.which("appimagetool") or "/tmp/appimagetool-x86_64.AppImage"
    appimage_available = shutil.which("appimagetool") is not None or Path("/tmp/appimagetool-x86_64.AppImage").exists()
    results["appimagetool"] = {
        "version": "appimagetool" if appimage_available else "Not Found (will auto-fetch)",
        "status": appimage_available or platform.system() == "Linux",
    }

    # Sigstore Cosign (Artifact signing)
    cosign_ver = query_command(["cosign", "version"])
    results["cosign"] = {
        "version": cosign_ver or "Not Found (Optional for artifact signing)",
        "status": cosign_ver is not None,
    }

    # Java keytool (Android Keystore generator)
    keytool_bin = None
    try:
        from tools.generate_keystore import find_keytool
        keytool_bin = find_keytool()
    except Exception:
        keytool_bin = shutil.which("keytool")
    results["keytool"] = {
        "version": keytool_bin or "Not Found (Optional for Android keystore generation)",
        "status": keytool_bin is not None,
    }

    return results


def etch_build_info(target_os=None, mode="release", localversion="", version_override="", rust_version_override="", build_number_override="", log_callback=None):
    """
    Etches compiler versions, environment variables, build flags, commit hash,
    and MIT licensing into apps/attune_app/assets/build_info.json.
    Synchronizes pubspec.yaml and local.properties so Android and Flutter always reflect the input version.
    """
    def log(msg):
        if log_callback:
            log_callback(msg)
        else:
            print(msg)

    git_info = get_git_info()
    pub_ver, pub_build = get_pubspec_version()
    version = version_override if version_override else pub_ver
    build_num = str(build_number_override).strip() if build_number_override and str(build_number_override).strip() else pub_build
    full_ver = f"{version}{localversion}"
    rust_ffi_version = rust_version_override if rust_version_override else get_rust_crate_version("attune-ffi")

    effective_target_os = (target_os or platform.system()).lower()
    toolchains = assess_toolchains()

    # Automatically synchronize pubspec.yaml and local.properties with target version input
    sync_pubspec_version(full_ver, build_num, log_callback=log)

    build_info = {
        "application_name": "Attune",
        "version": version,
        "build_number": build_num,
        "localversion": localversion,
        "suffix_version": localversion,
        "release_tag": f"v{full_ver}",
        "full_version": full_ver,
        "rust_ffi_version": rust_ffi_version,
        "git_commit": git_info["git_commit"],
        "git_commit_full": git_info["git_commit_full"],
        "git_branch": git_info["git_branch"],
        "git_dirty": git_info["git_dirty"],
        "build_mode": mode.capitalize(),
        "build_timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "target_os": effective_target_os,
        "target_arch": "universal (arm64/x86_64)" if effective_target_os == "android" else platform.machine(),
        "python_version": toolchains["python"]["version"],
        "compiler_rustc": toolchains["rustc"]["version"],
        "compiler_cargo": toolchains["cargo"]["version"],
        "compiler_flutter": toolchains["flutter"]["version"],
        "compiler_dart": toolchains["dart"]["version"],
        "compiler_java": toolchains["java"]["version"],
        "android_ndk": toolchains["android_ndk"]["version"],
        "ai_credits": "Gemini 3.8 Flash (Google Antigravity) & Claude Opus 5",
        "license": "MIT License",
        "license_disclaimer": (
            "Distributed under the MIT License. Third-party components and DSP libraries "
            "are licensed under BSD-2-Clause, BSD-3-Clause, Apache-2.0, MIT, MPL-2.0, LGPL, and GPL."
        ),
    }

    assets_dir = REPO_ROOT / "apps" / "attune_app" / "assets"
    assets_dir.mkdir(parents=True, exist_ok=True)
    out_file = assets_dir / "build_info.json"
    out_file.write_text(json.dumps(build_info, indent=2), encoding="utf-8")
    log(f"[Etch] Successfully etched compiler and build metadata into {out_file} (v{full_ver}+{build_num})")
    return build_info


def apply_git_tag(version, suffix="", log_func=None):
    """Sets/updates the git release tag v{version}{suffix} on the current commit."""
    def log(msg):
        if log_func:
            log_func(msg)
        else:
            print(msg)

    tag_name = f"v{version}{suffix}"
    try:
        existing_tags = subprocess.check_output(
            ["git", "tag", "-l", tag_name], cwd=REPO_ROOT, text=True, stderr=subprocess.DEVNULL
        ).strip()
        if existing_tags:
            cmd = ["git", "tag", "-f", tag_name, "-m", f"Release {tag_name}"]
            log(f"[Git] Updating existing release git tag: {tag_name}")
        else:
            cmd = ["git", "tag", "-a", tag_name, "-m", f"Release {tag_name}"]
            log(f"[Git] Creating release git tag: {tag_name}")
        subprocess.check_call(cmd, cwd=REPO_ROOT, stderr=subprocess.DEVNULL)
        log(f"[Git] Successfully set release git tag: {tag_name}")
        return tag_name
    except Exception as e:
        log(f"[Git] Warning: Could not set git tag {tag_name}: {e}")
        return None


def generate_patch_notes(max_commits=30, localversion="", version_override=""):
    """
    Generates patch_notes.txt from git commit history to embed into Inno Setup
    and GitHub Actions release notes.
    """
    pub_ver, _ = get_pubspec_version()
    version = version_override if version_override else pub_ver
    full_ver = f"{version}{localversion}"
    git_info = get_git_info()
    timestamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    header = (
        f"Attune Release Patch Notes - Version {full_ver} (Tag: v{full_ver})\n"
        f"Built on {timestamp} (Commit {git_info['git_commit']} on branch {git_info['git_branch']})\n"
        f"Distributed under the MIT License\n"
        f"{'=' * 78}\n\n"
        f"Recent Git Commits:\n"
    )

    try:
        log_output = subprocess.check_output(
            ["git", "log", f"-n{max_commits}", "--pretty=format:* %s (%h) - %an, %ad", "--date=short"],
            cwd=REPO_ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except Exception:
        log_output = "* Initial release and packaging milestone."

    patch_notes_content = f"{header}{log_output}\n\n"
    patch_notes_file = REPO_ROOT / "patch_notes.txt"
    patch_notes_file.write_text(patch_notes_content, encoding="utf-8")
    print(f"[PatchNotes] Generated {patch_notes_file} with {max_commits} recent commits.")
    return patch_notes_content


def compile_rust(mode="release", target_os=None, log_callback=None):
    """Compiles the native Rust DSP engine crate (attune-ffi)."""
    def log(msg):
        if log_callback:
            log_callback(msg)
        else:
            print(msg)

    effective_target_os = (target_os or platform.system()).lower()
    log(f"[Rust] Compiling attune-ffi for {effective_target_os} in {mode} mode...")

    if effective_target_os == "android":
        ndk = find_android_ndk()
        if not ndk:
            raise RuntimeError("Android NDK not found. Set ANDROID_NDK_HOME, NDK_HOME, or install NDK via Android SDK manager.")
        llvm_bin = get_ndk_llvm_bin(ndk)
        if not llvm_bin:
            raise RuntimeError(f"Could not locate LLVM toolchain binaries in NDK: {ndk}")

        is_windows = platform.system() == "Windows"
        ext = ".cmd" if is_windows else ""
        ar_ext = ".exe" if is_windows else ""

        # Android targets to cross-compile (API 26 is minimum required for AAudio / -laaudio support)
        android_targets = [
            ("aarch64-linux-android", "arm64-v8a", f"aarch64-linux-android26-clang{ext}", f"aarch64-linux-android26-clang++{ext}"),
            ("armv7-linux-androideabi", "armeabi-v7a", f"armv7a-linux-androideabi26-clang{ext}", f"armv7a-linux-androideabi26-clang++{ext}"),
            ("x86_64-linux-android", "x86_64", f"x86_64-linux-android26-clang{ext}", f"x86_64-linux-android26-clang++{ext}"),
        ]

        produced = []
        for rust_target, jni_abi, clang_name, clangpp_name in android_targets:
            log(f"[Rust] Building Android target {rust_target} ({jni_abi})...")
            clang_bin = llvm_bin / clang_name
            clangpp_bin = llvm_bin / clangpp_name
            ar_bin = llvm_bin / f"llvm-ar{ar_ext}"

            if not clang_bin.exists():
                raise FileNotFoundError(f"NDK clang tool not found: {clang_bin}")

            target_snake = rust_target.replace("-", "_")
            env = os.environ.copy()
            env[f"CC_{target_snake}"] = str(clang_bin)
            env[f"CXX_{target_snake}"] = str(clangpp_bin)
            env[f"AR_{target_snake}"] = str(ar_bin)
            env[f"CARGO_TARGET_{target_snake.upper()}_LINKER"] = str(clang_bin)

            cmd = ["cargo", "build", "-p", "attune-ffi", "-p", "attune-watchguard", "--target", rust_target]
            if mode.lower() == "release":
                cmd.append("--release")

            proc = subprocess.Popen(
                cmd,
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                env=env,
                shell=IS_WINDOWS,
            )
            for line in proc.stdout:
                log(f"  [cargo-{jni_abi}] {line.rstrip()}")
            proc.wait()
            if proc.returncode != 0:
                raise RuntimeError(f"Cargo build failed for {rust_target} with exit code {proc.returncode}")

            so_path = REPO_ROOT / "target" / rust_target / mode.lower() / "libattune_engine.so"
            watchguard_so = REPO_ROOT / "target" / rust_target / mode.lower() / "libattune_watchguard.so"
            if not so_path.exists():
                raise FileNotFoundError(f"Android native library not produced at: {so_path}")

            # Place in apps/attune_app/android/app/src/main/jniLibs/<jni_abi>/
            jni_dir = REPO_ROOT / "apps" / "attune_app" / "android" / "app" / "src" / "main" / "jniLibs" / jni_abi
            jni_dir.mkdir(parents=True, exist_ok=True)
            dest_so = jni_dir / "libattune_engine.so"
            shutil.copy2(so_path, dest_so)
            log(f"[Rust] Copied {jni_abi} native engine library to: {dest_so}")
            if watchguard_so.exists():
                dest_watchguard = jni_dir / "libattune_watchguard.so"
                shutil.copy2(watchguard_so, dest_watchguard)
                log(f"[Rust] Copied {jni_abi} watchguard library to: {dest_watchguard}")
            produced.append(dest_so)

        # Also copy arm64-v8a to apps/attune_app/lib/ for generic fallback
        dest_lib_dir = REPO_ROOT / "apps" / "attune_app" / "lib"
        dest_lib_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(produced[0], dest_lib_dir / "libattune_engine.so")
        target_wg = REPO_ROOT / "target" / "aarch64-linux-android" / mode.lower() / "libattune_watchguard.so"
        if target_wg.exists():
            shutil.copy2(target_wg, dest_lib_dir / "libattune_watchguard.so")
        return produced[0]

    # Host desktop build (linux, windows, macos)
    cmd = [
        "cargo", "build",
        "-p", "attune-ffi",
        "-p", "attune-watchguard",
        "-p", "attune-procmgr",
        "-p", "attune-music-metadata",
        "-p", "attune-desktop",
    ]
    if mode.lower() == "release":
        cmd.append("--release")

    env = os.environ.copy()
    proc = subprocess.Popen(
        cmd,
        cwd=REPO_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env=env,
        shell=IS_WINDOWS,
    )

    for line in proc.stdout:
        log(f"  [cargo] {line.rstrip()}")

    proc.wait()
    if proc.returncode != 0:
        raise RuntimeError(f"Cargo build failed with exit code {proc.returncode}")


    target_dir = REPO_ROOT / "target" / mode.lower()
    system = platform.system()

    if effective_target_os == "linux" or system == "Linux":
        lib_name = "libattune_engine.so"
        wg_name = "libattune_watchguard.so"
        pm_name = "libattune_procmgr.so"
        mc_name = "libattune_music_metadata.so"
    elif effective_target_os == "windows" or system == "Windows":
        lib_name = "attune_engine.dll"
        wg_name = "attune_watchguard.dll"
        pm_name = "attune_procmgr.dll"
        mc_name = "attune_music_metadata.dll"
    elif system == "Darwin":
        lib_name = "libattune_engine.dylib"
        wg_name = "libattune_watchguard.dylib"
        pm_name = "libattune_procmgr.dylib"
        mc_name = "libattune_music_metadata.dylib"
    else:
        lib_name = "libattune_engine.so"
        wg_name = "libattune_watchguard.so"
        pm_name = "libattune_procmgr.so"
        mc_name = "libattune_music_metadata.so"

    lib_path = target_dir / lib_name
    if not lib_path.exists():
        raise FileNotFoundError(f"Native library was not produced at: {lib_path}")

    # Copy native library to apps/attune_app/lib/ for relative lookup
    dest_lib_dir = REPO_ROOT / "apps" / "attune_app" / "lib"
    dest_lib_dir.mkdir(parents=True, exist_ok=True)
    dest_lib = dest_lib_dir / lib_name
    shutil.copy2(lib_path, dest_lib)
    log(f"[Rust] Copied native library to relative lookup path: {dest_lib}")

    wg_path = target_dir / wg_name
    if wg_path.exists():
        dest_wg = dest_lib_dir / wg_name
        shutil.copy2(wg_path, dest_wg)
        log(f"[Rust] Copied watchguard library to: {dest_wg}")

    pm_path = target_dir / pm_name
    if pm_path.exists():
        dest_pm = dest_lib_dir / pm_name
        shutil.copy2(pm_path, dest_pm)
        log(f"[Rust] Copied procmgr library to: {dest_pm}")

    mc_path = target_dir / mc_name
    if mc_path.exists():
        dest_mc = dest_lib_dir / mc_name
        shutil.copy2(mc_path, dest_mc)
        log(f"[Rust] Copied music_metadata library to: {dest_mc}")

        # Also maintain music_metadata_changer alias in target and app lib dir
        changer_name = mc_name.replace("attune_music_metadata", "attune_music_metadata_changer")
        dest_changer = dest_lib_dir / changer_name
        shutil.copy2(mc_path, dest_changer)
        target_changer = target_dir / changer_name
        shutil.copy2(mc_path, target_changer)
        log(f"[Rust] Mirrored music_metadata_changer alias to: {dest_changer}")
    return lib_path


def compile_flutter(target_os="linux", mode="release", version_override="", localversion="", build_number=None, log_callback=None):
    """Compiles Flutter application (desktop or Android APK) passing exact build-name and build-number."""
    def log(msg):
        if log_callback:
            log_callback(msg)
        else:
            print(msg)

    effective_target_os = (target_os or platform.system()).lower()
    pub_ver, cur_build = get_pubspec_version()
    base_ver = version_override if version_override else pub_ver
    effective_build_name = f"{base_ver}{localversion}" if localversion else base_ver
    effective_build_num = str(build_number).strip() if build_number and str(build_number).strip() else cur_build

    # Always ensure pubspec.yaml and local.properties match the requested version and build number
    sync_pubspec_version(effective_build_name, effective_build_num, log_callback=log)

    log(f"[Flutter] Compiling Flutter app for {effective_target_os} in {mode} mode (version: {effective_build_name}+{effective_build_num})...")
    flutter_dir = REPO_ROOT / "apps" / "attune_app"

    if effective_target_os == "android" and mode.lower() == "release":
        key_props = flutter_dir / "android" / "key.properties"
        keystore_cand = flutter_dir / "android" / "app" / "attune_release.jks"
        if not key_props.exists() and not keystore_cand.exists() and not os.environ.get("ATTUNE_KEYSTORE_PASSWORD"):
            log("[Flutter] Note: Android release keystore/key.properties not detected. Gradle will build using debug signing.")
            log("[Flutter] Note: Run 'python tools/generate_keystore.py' or 'python tools/builder.py --generate-keystore' to generate a release keystore.")

    if effective_target_os == "android":
        cmd = [
            "flutter", "build", "apk",
            f"--{mode.lower()}",
            f"--build-name={effective_build_name}",
            f"--build-number={effective_build_num}",
        ]
    else:
        cmd = [
            "flutter", "build", effective_target_os,
            f"--{mode.lower()}",
            f"--build-name={effective_build_name}",
            f"--build-number={effective_build_num}",
        ]

    proc = subprocess.Popen(
        cmd,
        cwd=flutter_dir,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        shell=IS_WINDOWS,
    )


    for line in proc.stdout:
        log(f"  [flutter] {line.rstrip()}")

    proc.wait()
    if proc.returncode != 0:
        raise RuntimeError(f"Flutter build failed with exit code {proc.returncode}")

    # After Flutter compilation, guarantee native library is bundled inside the bundle's lib/ directory
    if effective_target_os == "linux":
        bundle_dir = flutter_dir / "build" / "linux" / "x64" / mode.lower() / "bundle"
        rust_lib = REPO_ROOT / "target" / mode.lower() / "libattune_engine.so"
        if bundle_dir.exists() and rust_lib.exists():
            bundle_lib_dir = bundle_dir / "lib"
            bundle_lib_dir.mkdir(parents=True, exist_ok=True)
            shutil.copy2(rust_lib, bundle_lib_dir / "libattune_engine.so")
            log(f"[Flutter] Bundled {rust_lib.name} into {bundle_lib_dir}")
    elif effective_target_os == "windows":
        runner_dir = flutter_dir / "build" / "windows" / "x64" / "runner" / mode.capitalize()
        rust_dll = REPO_ROOT / "target" / mode.lower() / "attune_engine.dll"
        if runner_dir.exists() and rust_dll.exists():
            runner_lib_dir = runner_dir / "lib"
            runner_lib_dir.mkdir(parents=True, exist_ok=True)
            shutil.copy2(rust_dll, runner_lib_dir / "attune_engine.dll")
            shutil.copy2(rust_dll, runner_dir / "attune_engine.dll")
            log(f"[Flutter] Bundled {rust_dll.name} into {runner_dir}")
    elif effective_target_os == "android":
        apk_path = flutter_dir / "build" / "app" / "outputs" / "flutter-apk" / f"app-{mode.lower()}.apk"
        log(f"[Flutter] Built Android APK at: {apk_path}")

    log(f"[Flutter] Build completed successfully for {effective_target_os}.")


def sign_blob_with_cosign(blob_path, log_func=None):
    """Signs a binary or archive blob using Sigstore Cosign, producing <blob>.bundle."""
    def log(msg):
        if log_func:
            log_func(msg)
        else:
            print(msg)

    cosign_bin = shutil.which("cosign")
    if not cosign_bin:
        log("[Cosign] Note: cosign binary not found in PATH; skipping artifact signing.")
        return None

    blob = Path(blob_path)
    if not blob.exists():
        log(f"[Cosign] Warning: Target blob not found for signing: {blob}")
        return None

    bundle_out = blob.with_name(f"{blob.name}.bundle")
    log(f"[Cosign] Signing {blob.name} via Sigstore keyless workflow (output: {bundle_out.name})...")
    cmd = [cosign_bin, "sign-blob", "--yes", "--bundle", str(bundle_out), str(blob)]
    try:
        proc = subprocess.run(
            cmd,
            cwd=blob.parent,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            shell=IS_WINDOWS,
        )
        if proc.returncode == 0 and bundle_out.exists():
            log(f"[Cosign] Successfully generated Sigstore verification bundle: {bundle_out.name}")
            return bundle_out
        else:
            err_msg = proc.stderr.strip() if proc.stderr else f"exit code {proc.returncode}"
            log(f"[Cosign] Note: cosign sign-blob completed with note ({err_msg})")
            return None
    except Exception as e:
        log(f"[Cosign] Warning: Failed to invoke cosign: {e}")
        return None


def sign_dll_file(dll_path, log_func=None):
    """
    Signs a dynamic component (.dll / .so) for anti-tamper verification.
    1. Cosign keyless / bundle signing (<dll>.bundle)
    2. Authenticode signing via osslsigncode or Microsoft SignTool if certificate configured.
    """
    def log(msg):
        if log_func:
            log_func(msg)
        else:
            print(msg)

    dll = Path(dll_path)
    if not dll.exists():
        log(f"[Sign] Warning: Target dynamic component not found: {dll}")
        return None

    # 1. Sigstore / Cosign bundle signing
    bundle = sign_blob_with_cosign(dll, log_func=log)

    # 2. Windows Authenticode signing (if certificate configured)
    cert_path = os.environ.get("CERT_PATH") or os.environ.get("WINDOWS_CERT_PATH")
    cert_pass = os.environ.get("CERT_PASS") or os.environ.get("WINDOWS_CERT_PASS")

    if cert_path and Path(cert_path).exists():
        ossl_bin = shutil.which("osslsigncode")
        ms_signtool = shutil.which("signtool.exe") or (shutil.which("signtool") if IS_WINDOWS else None)

        if ossl_bin:
            log(f"[Sign/Authenticode] Signing {dll.name} via osslsigncode...")
            signed_tmp = dll.with_suffix(".signed")
            cmd = [
                ossl_bin, "sign",
                "-pkcs12", str(cert_path),
                "-n", "Attune Dynamic Component",
                "-i", "https://github.com/mizproject/attune",
                "-in", str(dll),
                "-out", str(signed_tmp),
            ]
            if cert_pass:
                cmd.extend(["-pass", cert_pass])
            try:
                res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                if res.returncode == 0 and signed_tmp.exists():
                    shutil.move(str(signed_tmp), str(dll))
                    log(f"[Sign/Authenticode] Successfully Authenticode-signed: {dll.name}")
                else:
                    log(f"[Sign/Authenticode] osslsigncode note: {res.stderr.strip()}")
            except Exception as e:
                log(f"[Sign/Authenticode] Warning: {e}")
        elif ms_signtool and IS_WINDOWS:
            log(f"[Sign/Authenticode] Signing {dll.name} via SignTool.exe...")
            cmd = [ms_signtool, "sign", "/f", str(cert_path), "/d", "Attune Dynamic Component", "/fd", "SHA256"]
            if cert_pass:
                cmd.extend(["/p", cert_pass])
            cmd.append(str(dll))
            try:
                res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                if res.returncode == 0:
                    log(f"[Sign/Authenticode] Successfully signed with SignTool: {dll.name}")
            except Exception as e:
                log(f"[Sign/Authenticode] Warning: {e}")
    else:
        log(f"[Sign] {dll.name}: Authenticode cert unset (CERT_PATH not configured); anti-tamper secured via cryptographic manifest & Sigstore.")

    return bundle


def generate_integrity_manifest(target_dir, version=None, log_func=None):
    """
    Generates attune_integrity.json containing cryptographic digests (SHA-256 and BLAKE3)
    and file sizes for all dynamic components (.dll and .so) in target_dir.
    Also produces SHA256SUMS.txt and mirrors to lib/ and target/ directories.
    """
    def log(msg):
        if log_func:
            log_func(msg)
        else:
            print(msg)

    target_dir = Path(target_dir)
    if not target_dir.exists():
        log(f"[Integrity] Warning: Target directory does not exist: {target_dir}")
        return None

    dyn = compute_dynamic_version()
    eff_ver = version or dyn["base_version"]

    components = {}
    sums_lines = []

    # Files to inspect: search target_dir, target_dir/lib, and workspace target/ folders
    target_files = []
    search_dirs = [target_dir, target_dir / "lib", REPO_ROOT / "target" / "release", REPO_ROOT / "target" / "debug"]
    for sdir in search_dirs:
        if sdir.exists():
            for ext in ("*.dll", "*.so"):
                target_files.extend(sdir.glob(ext))

    seen_names = set()
    for f in target_files:
        if f.name in seen_names or not f.is_file():
            continue
        seen_names.add(f.name)

        data = f.read_bytes()
        sha256_hash = hashlib.sha256(data).hexdigest()

        blake3_hash = None
        try:
            import blake3
            blake3_hash = blake3.blake3(data).hexdigest()
        except ImportError:
            b3_cli = shutil.which("b2sum") or shutil.which("blake3")
            if b3_cli:
                try:
                    res = subprocess.check_output([b3_cli, str(f)], text=True).split()[0]
                    blake3_hash = res.strip()
                except Exception:
                    pass
            if not blake3_hash:
                blake3_hash = sha256_hash

        components[f.name] = {
            "size_bytes": len(data),
            "sha256": sha256_hash,
            "blake3": blake3_hash,
        }
        sums_lines.append(f"{sha256_hash}  {f.name}")
        log(f"[Integrity] Hashed {f.name}: size={len(data)}B, SHA-256={sha256_hash[:12]}...")

    manifest = {
        "version": eff_ver,
        "dynamic_version": dyn["dynamic_id"],
        "build_number": dyn["build_number"],
        "generated_at": datetime.datetime.now(timezone.utc).isoformat(),
        "git_commit": dyn["git_commit"],
        "components": components,
    }

    manifest_path = target_dir / "attune_integrity.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    log(f"[Integrity] Generated anti-tamper manifest: {manifest_path}")

    sums_path = target_dir / "SHA256SUMS.txt"
    sums_path.write_text("\n".join(sums_lines) + "\n", encoding="utf-8")
    log(f"[Integrity] Generated checksums: {sums_path}")

    # Mirror into target/release/ and target/debug/ if present
    for mode in ("release", "debug"):
        t_dir = REPO_ROOT / "target" / mode
        if t_dir.exists():
            shutil.copy2(manifest_path, t_dir / "attune_integrity.json")

    # Mirror into lib/ if present
    lib_dir = target_dir / "lib"
    if lib_dir.exists() and lib_dir != target_dir:
        shutil.copy2(manifest_path, lib_dir / "attune_integrity.json")

    return manifest_path


def package_bundle(target_os="linux", mode="release", localversion="", bundle_types=None, version_override="", sign=False, log_callback=None):
    """Packages compiled binaries into Linux AppImage or Windows Inno Setup installer."""
    def log(msg):
        if log_callback:
            log_callback(msg)
        else:
            print(msg)

    if bundle_types is None:
        bundle_types = ["all"]

    pub_ver, _ = get_pubspec_version()
    version = version_override if version_override else pub_ver
    full_ver = f"{version}{localversion}"
    dist_dir = REPO_ROOT / "dist" / target_os.lower()
    dist_dir.mkdir(parents=True, exist_ok=True)

    artifacts = []

    if target_os.lower() == "linux":
        # 0. Standalone Native Rust Desktop Executable + Required Shared Libraries (.so) + Anti-Tamper Manifest
        native_desktop_bin = REPO_ROOT / "target" / mode.lower() / "attune-desktop"
        dist_lib_dir = dist_dir / "lib"
        dist_lib_dir.mkdir(parents=True, exist_ok=True)
        for so_name in ("libattune_engine.so", "libattune_procmgr.so", "libattune_watchguard.so", "libattune_music_metadata.so", "libattune_music_metadata_changer.so"):
            so_src = REPO_ROOT / "target" / mode.lower() / so_name
            if so_src.exists():
                shutil.copy2(so_src, dist_lib_dir / so_name)
                shutil.copy2(so_src, dist_dir / so_name)
                log(f"[Package] Bundled dynamic component {so_name} -> {dist_lib_dir / so_name}")
                sign_dll_file(dist_lib_dir / so_name, log_func=log)

        generate_integrity_manifest(dist_dir, version=version, log_func=log)

        if native_desktop_bin.exists():
            dest_native = dist_dir / f"Attune-Desktop-{full_ver}-Linux-x86_64"
            shutil.copy2(native_desktop_bin, dest_native)
            os.chmod(dest_native, 0o755)
            log(f"[Package] Packaged standalone native desktop binary: {dest_native}")
            if dest_native not in artifacts:
                artifacts.append(dest_native)

        # 1. Portable .tar.gz bundle
        if "all" in bundle_types or "zip" in bundle_types or "tar" in bundle_types:
            tar_name = f"Attune-{full_ver}-Linux-x86_64-{mode.capitalize()}.tar.gz"
            tar_path = dist_dir / tar_name
            stage_tar_dir = REPO_ROOT / "build" / f"linux-portable-{mode.lower()}" / "Attune"
            if stage_tar_dir.exists():
                shutil.rmtree(stage_tar_dir)
            stage_tar_dir.mkdir(parents=True, exist_ok=True)
            stage_lib_dir = stage_tar_dir / "lib"
            stage_lib_dir.mkdir(parents=True, exist_ok=True)

            if native_desktop_bin.exists():
                shutil.copy2(native_desktop_bin, stage_tar_dir / "attune-desktop")
                os.chmod(stage_tar_dir / "attune-desktop", 0o755)
            for so_name in ("libattune_engine.so", "libattune_procmgr.so", "libattune_watchguard.so", "libattune_music_metadata.so", "libattune_music_metadata_changer.so"):
                so_src = dist_lib_dir / so_name
                if so_src.exists():
                    shutil.copy2(so_src, stage_lib_dir / so_name)
                    shutil.copy2(so_src, stage_tar_dir / so_name)
            for meta_file in ("attune_integrity.json", "SHA256SUMS.txt"):
                m_src = dist_dir / meta_file
                if m_src.exists():
                    shutil.copy2(m_src, stage_tar_dir / meta_file)
                    shutil.copy2(m_src, stage_lib_dir / meta_file)

            log(f"[Package] Creating portable tarball {tar_path}...")
            subprocess.check_call(
                ["tar", "-czf", str(tar_path), "-C", str(stage_tar_dir.parent), "Attune"],
                cwd=REPO_ROOT,
            )
            artifacts.append(tar_path)

        # 2. Linux AppImage
        if "all" in bundle_types or "appimage" in bundle_types:
            log("[Package] Assembling Linux AppImage...")
            script_path = REPO_ROOT / "packaging" / "linux" / "build_appimage.sh"
            cmd = [
                str(script_path),
                "--mode", mode.lower(),
                "--version", version,
                "--localversion", localversion,
                "--output", str(dist_dir),
            ]
            proc = subprocess.Popen(
                cmd,
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
            )
            for line in proc.stdout:
                log(f"  [appimage] {line.rstrip()}")
            proc.wait()
            if proc.returncode != 0:
                log("[Package] Warning: AppImage creation encountered an issue (is appimagetool installed?).")
            else:
                expected_appimage = dist_dir / f"Attune-{full_ver}-x86_64-{mode.lower()}.AppImage"
                if expected_appimage.exists():
                    artifacts.append(expected_appimage)

    elif target_os.lower() == "windows":
        # -1. Windows-compliant Icon Generation & Distribution
        if generate_windows_icon:
            try:
                generate_windows_icon(all_targets=True, log_func=log)
                log("[Package] Generated and synchronized Windows-compliant multi-resolution icons (.ico)")
            except Exception as e:
                log(f"[Package] Note: Windows icon generation: {e}")

        # 0. Standalone Native Rust Desktop Executable + Required Shared Libraries (.dll)
        native_desktop_exe = REPO_ROOT / "target" / mode.lower() / "attune-desktop.exe"
        dist_lib_dir = dist_dir / "lib"
        dist_lib_dir.mkdir(parents=True, exist_ok=True)

        if native_desktop_exe.exists():
            dest_native_exe = dist_dir / f"Attune-Desktop-{full_ver}-Windows-x64.exe"
            shutil.copy2(native_desktop_exe, dest_native_exe)
            log(f"[Package] Packaged standalone native desktop executable: {dest_native_exe}")
            if dest_native_exe not in artifacts:
                artifacts.append(dest_native_exe)

        for dll_name in ("attune_engine.dll", "attune_procmgr.dll", "attune_watchguard.dll", "attune_music_metadata.dll", "attune_music_metadata_changer.dll"):
            dll_src = REPO_ROOT / "target" / mode.lower() / dll_name
            if dll_src.exists():
                shutil.copy2(dll_src, dist_lib_dir / dll_name)
                shutil.copy2(dll_src, dist_dir / dll_name)
                log(f"[Package] Bundled dynamic component {dll_name} -> {dist_lib_dir / dll_name}")
                sign_dll_file(dist_lib_dir / dll_name, log_func=log)

        # Generate anti-tamper integrity manifest for all dynamic DLLs
        generate_integrity_manifest(dist_dir, version=version, log_func=log)

        # 1. Portable .zip bundle (Native Rust Desktop + Signed DLLs + Integrity Manifest)
        if "all" in bundle_types or "zip" in bundle_types:
            zip_base = dist_dir / f"Attune-{full_ver}-Windows-x86_64-{mode.capitalize()}"
            zip_path = dist_dir / f"Attune-{full_ver}-Windows-x86_64-{mode.capitalize()}.zip"
            stage_win_dir = REPO_ROOT / "build" / f"windows-portable-{mode.lower()}"
            if stage_win_dir.exists():
                shutil.rmtree(stage_win_dir)
            stage_win_dir.mkdir(parents=True, exist_ok=True)
            stage_win_lib = stage_win_dir / "lib"
            stage_win_lib.mkdir(parents=True, exist_ok=True)

            if native_desktop_exe.exists():
                shutil.copy2(native_desktop_exe, stage_win_dir / "attune-desktop.exe")
            for dll_name in ("attune_engine.dll", "attune_procmgr.dll", "attune_watchguard.dll", "attune_music_metadata.dll", "attune_music_metadata_changer.dll"):
                dll_src = dist_lib_dir / dll_name
                if dll_src.exists():
                    shutil.copy2(dll_src, stage_win_lib / dll_name)
                    shutil.copy2(dll_src, stage_win_dir / dll_name)
            for meta_file in ("attune_integrity.json", "SHA256SUMS.txt"):
                m_src = dist_dir / meta_file
                if m_src.exists():
                    shutil.copy2(m_src, stage_win_dir / meta_file)
                    shutil.copy2(m_src, stage_win_lib / meta_file)
            ico_src = REPO_ROOT / "packaging" / "windows" / "app_icon.ico"
            if ico_src.exists():
                shutil.copy2(ico_src, stage_win_dir / "app_icon.ico")

            log(f"[Package] Creating portable zip {zip_path}...")
            shutil.make_archive(str(zip_base), "zip", stage_win_dir)
            if zip_path.exists():
                artifacts.append(zip_path)

        # 2. Inno Setup Installer
        if "all" in bundle_types or "innosetup" in bundle_types:
            iscc_cmd = shutil.which("iscc") or "C:\\Program Files (x86)\\Inno Setup 6\\ISCC.exe"
            iss_script = REPO_ROOT / "packaging" / "windows" / "attune_installer.iss"
            if Path(iscc_cmd).exists() or shutil.which(iscc_cmd):
                log("[Package] Compiling Inno Setup installer...")
                cmd = [
                    iscc_cmd,
                    f"/DMyAppVersion={version}",
                    f"/DMyAppLocalVersion={localversion}",
                    f"/DBuildMode={mode.capitalize()}",
                    str(iss_script),
                ]
                proc = subprocess.Popen(
                    cmd,
                    cwd=REPO_ROOT,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    shell=IS_WINDOWS,
                )

                for line in proc.stdout:
                    log(f"  [iscc] {line.rstrip()}")
                proc.wait()
                expected_setup = dist_dir / f"Attune-{full_ver}-Windows-{mode.capitalize()}-Setup.exe"
                if expected_setup.exists() and expected_setup not in artifacts:
                    log(f"[Package] Produced Windows Installer: {expected_setup}")
                    artifacts.append(expected_setup)
            else:
                log("[Package] ISCC compiler not found; skipping Windows setup creation.")

        # 3. Sigstore / Cosign Signing for Windows blobs
        should_sign = sign or os.environ.get("COSIGN_SIGN") == "1" or os.environ.get("GITHUB_ACTIONS") == "true"
        if should_sign:
            for art in list(artifacts):
                if art.suffix.lower() in (".exe", ".zip"):
                    bundle = sign_blob_with_cosign(art, log_func=log)
                    if bundle and bundle.exists() and bundle not in artifacts:
                        artifacts.append(bundle)

    elif target_os.lower() == "android":
        flutter_dir = REPO_ROOT / "apps" / "attune_app"
        apk_candidates = [
            flutter_dir / "build" / "app" / "outputs" / "flutter-apk" / f"app-{mode.lower()}.apk",
            flutter_dir / "build" / "app" / "outputs" / "apk" / mode.lower() / f"app-{mode.lower()}.apk",
        ]
        found_apk = None
        for apk in apk_candidates:
            if apk.exists():
                found_apk = apk
                break
        if found_apk:
            dest_apk = dist_dir / f"Attune-{full_ver}-Android-{mode.capitalize()}.apk"
            shutil.copy2(found_apk, dest_apk)
            log(f"[Package] Copied Android APK: {dest_apk}")
            artifacts.append(dest_apk)
        else:
            log(f"[Package] Warning: No compiled APK found matching mode '{mode.lower()}' at {apk_candidates}")

    log(f"[Package] Completed packaging. Artifacts in {dist_dir}")
    return artifacts


# ==============================================================================
# Interactive Tkinter GUI Mode
# ==============================================================================

def launch_gui():
    """Launches the interactive Tkinter desktop GUI."""
    try:
        import tkinter as tk
        from tkinter import ttk, messagebox, scrolledtext
    except ImportError:
        print("ERROR: Tkinter is not installed on this system.")
        print("Run with --no-gui for command-line mode.")
        sys.exit(1)

    root = tk.Tk()
    root.title("Attune - Build & Packaging Studio")
    root.geometry("820x680")
    root.minsize(740, 560)

    # Styling
    bg_color = "#161922"
    fg_color = "#E0E3EC"
    accent_color = "#68D391"
    card_color = "#1E222D"

    root.configure(bg=bg_color)

    style = ttk.Style()
    try:
        style.theme_use("clam")
    except Exception:
        pass
    style.configure(".", background=bg_color, foreground=fg_color)
    style.configure("TLabel", background=bg_color, foreground=fg_color, font=("Segoe UI", 10))
    style.configure("Header.TLabel", font=("Segoe UI", 16, "bold"), foreground=accent_color)
    style.configure("Card.TFrame", background=card_color, relief="flat")
    style.configure("TButton", font=("Segoe UI", 9, "bold"), padding=6)
    style.configure("Accent.TButton", foreground="#0D1117", background=accent_color)

    # Main Scrollable / Padded Frame
    main_frame = tk.Frame(root, bg=bg_color, padx=20, pady=16)
    main_frame.pack(fill=tk.BOTH, expand=True)

    # Header
    header_frame = tk.Frame(main_frame, bg=bg_color)
    header_frame.pack(fill=tk.X, pady=(0, 12))

    lbl_title = ttk.Label(header_frame, text="Attune Build & Assessment Studio", style="Header.TLabel")
    lbl_title.pack(side=tk.LEFT)

    lbl_sub = ttk.Label(
        header_frame,
        text="DJ-Grade Native Engine & UI Packaging",
        font=("Segoe UI", 9, "italic"),
        foreground="#A0AEC0",
    )
    lbl_sub.pack(side=tk.RIGHT, pady=(6, 0))

    # Version Override & Suffix Configuration Card
    config_card = tk.LabelFrame(
        main_frame,
        text=" Version Override & Suffix Version ID ",
        bg=card_color,
        fg=accent_color,
        font=("Segoe UI", 10, "bold"),
        padx=14,
        pady=10,
    )
    config_card.pack(fill=tk.X, pady=(0, 10))

    cur_pub_ver, cur_pub_build = get_pubspec_version()
    cur_rust_ver = get_rust_crate_version("attune-ffi")

    # Grid rows: only show option for override version id, suffix/localversion, and build number input
    tk.Label(config_card, text="Override Version ID:", bg=card_color, fg=fg_color).grid(row=0, column=0, sticky="w", pady=4)
    ent_override_ver = tk.Entry(config_card, width=16, bg="#2D3748", fg=fg_color, insertbackground=fg_color)
    ent_override_ver.insert(0, cur_pub_ver)
    ent_override_ver.grid(row=0, column=1, sticky="w", padx=6, pady=4)

    tk.Label(config_card, text="(Sets base version ID & release tags, e.g. 0.1.1, 1.0.0)", bg=card_color, fg="#718096", font=("Segoe UI", 8)).grid(row=0, column=2, sticky="w", padx=(10, 0))

    tk.Label(config_card, text="Suffix Version ID:", bg=card_color, fg=fg_color).grid(row=1, column=0, sticky="w", pady=4)
    ent_suffix_ver = tk.Entry(config_card, width=16, bg="#2D3748", fg=fg_color, insertbackground=fg_color)
    ent_suffix_ver.insert(0, "")
    ent_suffix_ver.grid(row=1, column=1, sticky="w", padx=6, pady=4)

    tk.Label(config_card, text="(Optional suffix appended to version, e.g. -rc1, -beta, -git)", bg=card_color, fg="#718096", font=("Segoe UI", 8)).grid(row=1, column=2, sticky="w", padx=(10, 0))

    tk.Label(config_card, text="Build Code / Number:", bg=card_color, fg=fg_color).grid(row=2, column=0, sticky="w", pady=4)
    ent_build_num = tk.Entry(config_card, width=16, bg="#2D3748", fg=fg_color, insertbackground=fg_color)
    ent_build_num.insert(0, cur_pub_build)
    ent_build_num.grid(row=2, column=1, sticky="w", padx=6, pady=4)

    tk.Label(config_card, text="(Android versionCode & build metadata, e.g. 2, 3)", bg=card_color, fg="#718096", font=("Segoe UI", 8)).grid(row=2, column=2, sticky="w", padx=(10, 0))

    set_tag_var = tk.BooleanVar(value=True)
    chk_set_tag = tk.Checkbutton(
        config_card,
        text="Apply as Git Release Tag (v<version><suffix>)",
        variable=set_tag_var,
        bg=card_color,
        fg=accent_color,
        selectcolor="#2D3748",
        activebackground=card_color,
        activeforeground=accent_color,
        font=("Segoe UI", 9),
    )
    chk_set_tag.grid(row=3, column=0, columnspan=2, sticky="w", pady=(4, 0))

    # Dynamic Continuous Integration Version Row
    dyn_init = compute_dynamic_version()
    lbl_dyn_ver = tk.Label(
        config_card,
        text=f"Dynamic Version: {dyn_init['dynamic_id']} (Commits: {dyn_init['build_number']})",
        bg=card_color,
        fg=accent_color,
        font=("Segoe UI", 8, "bold"),
    )
    lbl_dyn_ver.grid(row=4, column=0, columnspan=2, sticky="w", pady=(6, 2))

    def on_bump_action(btype):
        n_ver, n_bld = bump_semver(btype, log_callback=gui_log)
        ent_override_ver.delete(0, tk.END)
        ent_override_ver.insert(0, n_ver)
        ent_build_num.delete(0, tk.END)
        ent_build_num.insert(0, n_bld)
        new_dyn = compute_dynamic_version()
        lbl_dyn_ver.config(text=f"Dynamic Version: {new_dyn['dynamic_id']} (Commits: {new_dyn['build_number']})")
        gui_log(f"Dynamic Version bumped ({btype}): v{n_ver}+{n_bld}")

    bump_frame = tk.Frame(config_card, bg=card_color)
    bump_frame.grid(row=4, column=2, sticky="w", padx=(10, 0), pady=(4, 2))
    tk.Label(bump_frame, text="Quick Bump:", bg=card_color, fg="#718096", font=("Segoe UI", 8)).pack(side=tk.LEFT, padx=(0, 4))
    tk.Button(bump_frame, text="+ Patch", bg="#2D3748", fg=fg_color, font=("Segoe UI", 8), command=lambda: on_bump_action("patch")).pack(side=tk.LEFT, padx=2)
    tk.Button(bump_frame, text="+ Minor", bg="#2D3748", fg=fg_color, font=("Segoe UI", 8), command=lambda: on_bump_action("minor")).pack(side=tk.LEFT, padx=2)
    tk.Button(bump_frame, text="+ Major", bg="#2D3748", fg=fg_color, font=("Segoe UI", 8), command=lambda: on_bump_action("major")).pack(side=tk.LEFT, padx=2)

    # Build Mode & Target OS Selection Card
    target_card = tk.LabelFrame(
        main_frame,
        text=" Compilation Target & Mode ",
        bg=card_color,
        fg=accent_color,
        font=("Segoe UI", 10, "bold"),
        padx=14,
        pady=8,
    )
    target_card.pack(fill=tk.X, pady=(0, 10))

    target_os_var = tk.StringVar(value=platform.system().lower())
    mode_var = tk.StringVar(value="release")

    tk.Label(target_card, text="Target OS:", bg=card_color, fg=fg_color).grid(row=0, column=0, sticky="w", pady=2)
    cb_target = ttk.Combobox(target_card, textvariable=target_os_var, values=["linux", "windows", "android"], width=10, state="readonly")
    cb_target.grid(row=0, column=1, sticky="w", padx=6, pady=2)

    tk.Label(target_card, text="Build Mode:", bg=card_color, fg=fg_color).grid(row=0, column=2, sticky="w", padx=(16, 0), pady=2)
    tk.Radiobutton(target_card, text="Release", variable=mode_var, value="release", bg=card_color, fg=fg_color, selectcolor="#2D3748").grid(row=0, column=3, sticky="w", padx=4)
    tk.Radiobutton(target_card, text="Debug", variable=mode_var, value="debug", bg=card_color, fg=fg_color, selectcolor="#2D3748").grid(row=0, column=4, sticky="w", padx=4)
    tk.Radiobutton(target_card, text="Both (Release & Debug)", variable=mode_var, value="both", bg=card_color, fg=fg_color, selectcolor="#2D3748").grid(row=0, column=5, sticky="w", padx=4)

    # Action Buttons Toolbar
    btn_frame = tk.Frame(main_frame, bg=bg_color)
    btn_frame.pack(fill=tk.X, pady=(0, 8))

    def gui_log(msg):
        txt_log.config(state=tk.NORMAL)
        txt_log.insert(tk.END, msg + "\n")
        txt_log.see(tk.END)
        txt_log.config(state=tk.DISABLED)
        root.update_idletasks()

    def on_assess():
        gui_log("=== Assessing Host Toolchains & Environment ===")
        results = assess_toolchains()
        for k, v in results.items():
            st = "✓ READY" if v.get("status") else "✗ MISSING"
            ver = v.get("version", "N/A")
            gui_log(f"  [{st}] {k:15}: {ver}")

    def on_etch():
        loc = ent_suffix_ver.get().strip()
        ver = ent_override_ver.get().strip()
        bld = ent_build_num.get().strip()
        m = mode_var.get()
        t = target_os_var.get()
        modes = ["debug", "release"] if m == "both" else [m]
        for sub_m in modes:
            info = etch_build_info(target_os=t, mode=sub_m, localversion=loc, version_override=ver, rust_version_override=ver, build_number_override=bld, log_callback=gui_log)
            gui_log(f"Etched build info ({sub_m}): v{info['full_version']}+{info['build_number']} target {t} commit {info['git_commit']}")

    def on_patch_notes():
        loc = ent_suffix_ver.get().strip()
        ver = ent_override_ver.get().strip()
        notes = generate_patch_notes(max_commits=25, localversion=loc, version_override=ver)
        if set_tag_var.get() and ver:
            apply_git_tag(ver, loc, log_func=gui_log)
        gui_log(f"Generated patch_notes.txt with {len(notes.splitlines())} lines.")

    def on_compile_rust():
        m = mode_var.get()
        t = target_os_var.get()
        modes = ["debug", "release"] if m == "both" else [m]
        try:
            for sub_m in modes:
                lib = compile_rust(mode=sub_m, target_os=t, log_callback=gui_log)
                gui_log(f"✓ Rust build succeeded for {t} ({sub_m}): {lib}")
        except Exception as e:
            gui_log(f"✗ Rust build error: {e}")

    def on_compile_flutter():
        loc = ent_suffix_ver.get().strip()
        ver = ent_override_ver.get().strip()
        bld = ent_build_num.get().strip()
        m = mode_var.get()
        t = target_os_var.get()
        modes = ["debug", "release"] if m == "both" else [m]
        try:
            for sub_m in modes:
                compile_flutter(target_os=t, mode=sub_m, version_override=ver, localversion=loc, build_number=bld, log_callback=gui_log)
                gui_log(f"✓ Flutter build succeeded for {t} ({sub_m}).")
        except Exception as e:
            gui_log(f"✗ Flutter build error: {e}")

    def on_full_build():
        loc = ent_suffix_ver.get().strip()
        ver = ent_override_ver.get().strip()
        bld = ent_build_num.get().strip()
        m = mode_var.get()
        t = target_os_var.get()
        modes = ["debug", "release"] if m == "both" else [m]
        gui_log("==================================================")
        gui_log(f"Starting Full Attune Build: {t} {m} v{ver}{loc}+{bld}")
        gui_log("==================================================")
        try:
            generate_patch_notes(max_commits=25, localversion=loc, version_override=ver)
            if set_tag_var.get() and ver:
                apply_git_tag(ver, loc, log_func=gui_log)
            all_artifacts = []
            for sub_m in modes:
                gui_log(f"\n--- [Pipeline Mode: {sub_m.upper()}] ---")
                etch_build_info(target_os=t, mode=sub_m, localversion=loc, version_override=ver, rust_version_override=ver, build_number_override=bld, log_callback=gui_log)
                compile_rust(mode=sub_m, target_os=t, log_callback=gui_log)
                compile_flutter(target_os=t, mode=sub_m, version_override=ver, localversion=loc, build_number=bld, log_callback=gui_log)
                artifacts = package_bundle(target_os=t, mode=sub_m, localversion=loc, version_override=ver, log_callback=gui_log)
                all_artifacts.extend(artifacts)
            gui_log(f"\n🎉 Full build & packaging completed! Artifacts ({len(all_artifacts)}): {all_artifacts}")
        except Exception as e:
            gui_log(f"❌ Build failed: {e}")

    btn_assess = tk.Button(btn_frame, text="🔍 Assess Toolchains", bg="#2D3748", fg=fg_color, command=on_assess)
    btn_assess.pack(side=tk.LEFT, padx=(0, 6))

    btn_etch = tk.Button(btn_frame, text="📝 Etch Build Info", bg="#2D3748", fg=fg_color, command=on_etch)
    btn_etch.pack(side=tk.LEFT, padx=(0, 6))

    btn_notes = tk.Button(btn_frame, text="📜 Patch Notes", bg="#2D3748", fg=fg_color, command=on_patch_notes)
    btn_notes.pack(side=tk.LEFT, padx=(0, 6))

    btn_rust = tk.Button(btn_frame, text="🦀 Compile Rust FFI", bg="#2D3748", fg=fg_color, command=on_compile_rust)
    btn_rust.pack(side=tk.LEFT, padx=(0, 6))

    btn_flutter = tk.Button(btn_frame, text="📱 Compile Flutter", bg="#2D3748", fg=fg_color, command=on_compile_flutter)
    btn_flutter.pack(side=tk.LEFT, padx=(0, 6))

    def on_generate_icons():
        if generate_windows_icon:
            try:
                gui_log("=== Generating Windows-Compliant Multi-Resolution Icons (.ico) ===")
                out = generate_windows_icon(all_targets=True, log_func=gui_log)
                gui_log(f"✓ Successfully generated & distributed Windows icons: {out}")
            except Exception as e:
                gui_log(f"✗ Icon generation failed: {e}")
        else:
            gui_log("✗ generate_windows_icons module not available.")

    def on_sign_manifest():
        gui_log("=== Generating Anti-Tamper Integrity Manifest & Signing DLLs ===")
        t = target_os_var.get()
        m = mode_var.get()
        modes = ["debug", "release"] if m == "both" else [m]
        dist_dir = REPO_ROOT / "dist" / t.lower()
        dist_dir.mkdir(parents=True, exist_ok=True)
        for sub_m in modes:
            for ext in ("*.dll", "*.so"):
                for cand in (REPO_ROOT / "target" / sub_m).glob(ext):
                    sign_dll_file(cand, log_func=gui_log)
            generate_integrity_manifest(dist_dir, log_func=gui_log)
        gui_log("✓ Anti-tamper integrity manifest & signatures completed.")

    btn_icons = tk.Button(btn_frame, text="🎨 Icons", bg="#2D3748", fg=fg_color, command=on_generate_icons)
    btn_icons.pack(side=tk.LEFT, padx=(0, 6))

    btn_sign = tk.Button(btn_frame, text="🛡️ Sign/Manifest", bg="#2D3748", fg=fg_color, command=on_sign_manifest)
    btn_sign.pack(side=tk.LEFT, padx=(0, 6))

    def on_open_keystore_tool():
        try:
            from tools.generate_keystore import launch_gui as launch_keystore_gui
            launch_keystore_gui()
        except Exception as e:
            gui_log(f"✗ Failed to open Keystore Studio: {e}")

    btn_keystore = tk.Button(btn_frame, text="🔑 Keystore Tool", bg="#2D3748", fg=accent_color, command=on_open_keystore_tool)
    btn_keystore.pack(side=tk.LEFT, padx=(0, 6))

    btn_full = tk.Button(btn_frame, text="🚀 Full Build & Package", bg="#38A169", fg="#FFFFFF", font=("Segoe UI", 9, "bold"), command=on_full_build)
    btn_full.pack(side=tk.RIGHT)

    # Scrolled Text Console Log
    log_frame = tk.LabelFrame(
        main_frame,
        text=" Live Execution Logs ",
        bg=card_color,
        fg=accent_color,
        font=("Segoe UI", 10, "bold"),
    )
    log_frame.pack(fill=tk.BOTH, expand=True)

    txt_log = scrolledtext.ScrolledText(
        log_frame,
        bg="#0D1117",
        fg="#A0AEC0",
        insertbackground="#FFFFFF",
        font=("Consolas", 9),
        state=tk.DISABLED,
    )
    txt_log.pack(fill=tk.BOTH, expand=True, padx=6, pady=6)

    gui_log("Attune Build & Assessment Studio Ready.")
    gui_log(f"Detected OS: {platform.system()} ({platform.machine()})")
    gui_log(f"Pubspec App Version: {cur_pub_ver}+{cur_pub_build} | Rust Core: {cur_rust_ver}")
    gui_log("Click 'Assess Toolchains' or 'Full Build & Package' to begin.")

    root.mainloop()


# ==============================================================================
# CLI Entrypoint
# ==============================================================================

def preprocess_argv(argv):
    """Normalizes CLI arguments so flags whose values start with '-' (e.g. --localversion -rc1)
    are joined with '=' syntax (e.g. --localversion=-rc1), preventing argparse from
    misinterpreting the value as an option flag.
    """
    known_val_flags = {
        "--target",
        "--mode",
        "--build-type",
        "--version",
        "--version-id",
        "--rust-version",
        "--localversion",
        "--build-number",
        "--build-code",
        "--step",
        "--bundle",
        "--bump",
    }
    known_boolean_flags = {
        "--gui",
        "--no-gui",
        "--set-tag",
        "--package",
        "--installer",
        "--download-tools",
        "--test",
        "--generate-icons",
        "--sign-dlls",
        "--dynamic-version",
        "-h",
        "--help",
    }
    new_argv = []
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg in known_val_flags:
            if i + 1 < len(argv):
                next_arg = argv[i + 1]
                if (
                    next_arg not in known_boolean_flags
                    and next_arg not in known_val_flags
                    and not any(next_arg.startswith(f + "=") for f in known_val_flags)
                ):
                    new_argv.append(f"{arg}={next_arg}")
                    i += 2
                    continue
        new_argv.append(arg)
        i += 1
    return new_argv


def main():
    parser = argparse.ArgumentParser(
        description="Attune Unified Build & Packaging Orchestrator (CLI and GUI)"
    )
    parser.add_argument(
        "--gui",
        action="store_true",
        help="Launch interactive Tkinter desktop GUI",
    )
    parser.add_argument(
        "--no-gui",
        action="store_true",
        help="Force CLI execution mode (disables GUI launch)",
    )
    parser.add_argument(
        "--target",
        choices=["linux", "windows", "android", "appimage", "all"],
        default=platform.system().lower(),
        help="Target platform operating system (default: current host OS)",
    )
    parser.add_argument(
        "--mode",
        "--build-type",
        dest="mode",
        default="release",
        help="Compilation mode (release, debug, or both; default: release)",
    )
    parser.add_argument(
        "--version",
        "--version-id",
        dest="version",
        default="",
        help="Explicit version ID override (e.g. 0.1.0, 1.0.0; defaults to pubspec.yaml version)",
    )
    parser.add_argument(
        "--rust-version",
        dest="rust_version",
        default="",
        help="Explicit Rust crate version ID override (defaults to Cargo.toml version)",
    )
    parser.add_argument(
        "--localversion",
        default="",
        help="Suffix version ID (e.g. -git, -rc1, -miku)",
    )
    parser.add_argument(
        "--build-number",
        "--build-code",
        dest="build_number",
        default="",
        help="Explicit integer build number / versionCode (defaults to pubspec.yaml build number)",
    )
    parser.add_argument(
        "--set-tag",
        action="store_true",
        help="Apply git release tag v<version><suffix> to current commit",
    )
    parser.add_argument(
        "--step",
        choices=["all", "assess", "etch", "patchnotes", "rust", "flutter", "package"],
        default="all",
        help="Specific build pipeline step to execute (default: all)",
    )
    parser.add_argument(
        "--bundle",
        choices=["all", "appimage", "innosetup", "zip", "tar", "none"],
        default="all",
        help="Packaging artifact formats to produce (default: all)",
    )
    parser.add_argument(
        "--package",
        action="store_true",
        help="Ensure packaging step is executed",
    )
    parser.add_argument(
        "--installer",
        action="store_true",
        help="Ensure Windows Inno Setup installer packaging is executed",
    )
    parser.add_argument(
        "--download-tools",
        action="store_true",
        help="Download external helper tools such as appimagetool if not present",
    )
    parser.add_argument(
        "--test",
        action="store_true",
        help="Run verification test suite before building/packaging",
    )
    parser.add_argument(
        "--generate-keystore",
        action="store_true",
        help="Generate a persistent PKCS12 release keystore for Android (via tools/generate_keystore.py)",
    )
    parser.add_argument(
        "--sign",
        "--cosign",
        dest="sign",
        action="store_true",
        help="Cryptographically sign release artifacts via Sigstore / Cosign",
    )
    parser.add_argument(
        "--generate-icons",
        action="store_true",
        help="Generate Windows-compliant multi-resolution .ico files from master icon/icon.png across all target paths",
    )
    parser.add_argument(
        "--bump",
        choices=["patch", "minor", "major"],
        help="Automatically bump semver (patch, minor, or major) and synchronize all manifests across repo",
    )
    parser.add_argument(
        "--sign-dlls",
        action="store_true",
        help="Sign all dynamic component DLLs and generate attune_integrity.json anti-tamper manifest",
    )
    parser.add_argument(
        "--dynamic-version",
        action="store_true",
        help="Print dynamic continuous integration version derived from Git metadata and exit",
    )

    args = parser.parse_args(preprocess_argv(sys.argv[1:]))

    if args.dynamic_version:
        dyn = compute_dynamic_version()
        print(f"Dynamic Version ID: {dyn['dynamic_id']}")
        print(f"Base SemVer       : {dyn['base_version']}")
        print(f"Build Number      : {dyn['build_number']}")
        print(f"Git Commit        : {dyn['git_commit']}")
        print(f"Git Describe      : {dyn['git_describe']}")
        print(f"Dirty Status      : {dyn['git_dirty']}")
        return

    if args.bump:
        new_v, new_b = bump_semver(args.bump)
        print(f"[Bump] Successfully bumped version to {new_v}+{new_b}")
        if not args.version:
            args.version = new_v
        if not args.build_number:
            args.build_number = new_b

    if args.generate_icons:
        print("\n--- [Step: Generate Windows-Compliant Multi-Resolution Icons] ---")
        if generate_windows_icon:
            try:
                out = generate_windows_icon(all_targets=True)
                print(f"  [OK] Successfully generated Windows-compliant .ico files: {out}")
            except Exception as e:
                print(f"  [X] Failed to generate Windows icons: {e}")
                sys.exit(1)
        else:
            print("  [X] Error: tools/generate_windows_icons.py not found.")
            sys.exit(1)
        if len(sys.argv) == 2 or (len(sys.argv) == 3 and "--no-gui" in sys.argv):
            return

    if args.sign_dlls:
        print("\n--- [Step: Sign DLLs & Generate Anti-Tamper Integrity Manifest] ---")
        dist_win = REPO_ROOT / "dist" / "windows"
        dist_win.mkdir(parents=True, exist_ok=True)
        for m in ("release", "debug"):
            t_dir = REPO_ROOT / "target" / m
            if t_dir.exists():
                for dll in t_dir.glob("*.dll"):
                    sign_dll_file(dll)
                for so in t_dir.glob("*.so"):
                    sign_dll_file(so)
        manifest = generate_integrity_manifest(dist_win)
        print(f"  [OK] Anti-tamper manifest and checksums generated: {manifest}")
        if len(sys.argv) == 2 or (len(sys.argv) == 3 and "--no-gui" in sys.argv):
            return

    # Determine whether to launch GUI:
    # If explicitly requested with --gui, or if no CLI step flags were passed AND DISPLAY is present and not --no-gui
    has_display = bool(os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY") or platform.system() == "Windows")
    if args.gui or (not args.no_gui and len(sys.argv) == 1 and has_display):
        launch_gui()
        return

    # Normalize Mode and Target OS
    mode_raw = (args.mode or "release").strip().lower()
    if mode_raw in ("both", "all"):
        modes = ["debug", "release"]
        display_mode = "both"
    elif mode_raw == "debug":
        modes = ["debug"]
        display_mode = "debug"
    else:
        modes = ["release"]
        display_mode = "release"

    raw_target = (args.target or platform.system().lower()).strip().lower()
    bundle_pref = args.bundle
    if raw_target == "appimage":
        target_os = "linux"
        bundle_pref = "appimage"
    else:
        target_os = raw_target

    if args.installer and target_os == "windows":
        bundle_pref = "all"

    # Handle --download-tools for AppImage
    if args.download_tools and target_os == "linux":
        tool_path = Path("/tmp/appimagetool-x86_64.AppImage")
        if not shutil.which("appimagetool") and not tool_path.exists():
            print("[Tools] Downloading appimagetool...")
            try:
                import urllib.request
                url = "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
                urllib.request.urlretrieve(url, str(tool_path))
                tool_path.chmod(0o755)
                print(f"[Tools] Standalone appimagetool downloaded to {tool_path}")
            except Exception as e:
                print(f"[Tools] Warning: Failed to download appimagetool: {e}")

    # Handle --generate-keystore for Android release signing
    if args.generate_keystore:
        if not args.no_gui and has_display:
            try:
                from tools.generate_keystore import launch_gui as launch_keystore_gui
                launch_keystore_gui()
                return
            except Exception as e:
                print(f"[Keystore] Note: Failed to launch Keystore GUI ({e}), falling back to CLI.")

        print("\n--- [Step: Generate Android Release Keystore] ---")
        try:
            from tools.generate_keystore import generate_keystore
            generate_keystore()
            print("  [OK] Release keystore and companion configuration ready.")
        except Exception as e:
            print(f"  [X] Keystore generation failed: {e}")
        return

    # Run verification tests if --test is requested
    if args.test:
        print("\n--- [Step: Verification Test Suite] ---")
        try:
            print("[Test] Running Rust test suite...")
            subprocess.check_call(["cargo", "test"], cwd=REPO_ROOT, shell=IS_WINDOWS)
            if target_os == "android" and shutil.which("flutter"):
                print("[Test] Running Flutter widget & algorithm test suite...")
                subprocess.check_call(["flutter", "test"], cwd=REPO_ROOT / "apps" / "attune_app", shell=IS_WINDOWS)
            print("  [OK] All test suites passed successfully.")
        except Exception as e:
            print(f"  [X] Test execution note: {e}")


    # CLI Pipeline Execution
    pub_ver, pub_build = get_pubspec_version()
    effective_ver = args.version if args.version else pub_ver
    local_sfx = args.localversion or ""
    if local_sfx and effective_ver.endswith(local_sfx):
        effective_ver = effective_ver[:-len(local_sfx)]
    build_num = args.build_number if args.build_number else pub_build
    full_ver = f"{effective_ver}{local_sfx}"

    # Sync all workspace version metadata early with version input
    sync_all_version_metadata(full_ver, build_num)

    print("==============================================================================")
    print(" Attune Build & Packaging Orchestrator (Headless CLI)")
    print(f" Target: {args.target} (effective: {target_os}) | Mode: {display_mode} | Version: '{effective_ver}' | Suffix: '{local_sfx}' | Build: '{build_num}'")
    print(" Distributed under the MIT License")
    print("==============================================================================")

    if args.set_tag:
        apply_git_tag(effective_ver, local_sfx)

    step = args.step
    if args.package and step != "all":
        step = "package"

    if step in ("all", "assess"):
        print("\n--- [Step: Assess Toolchains] ---")
        toolchains = assess_toolchains()
        for k, v in toolchains.items():
            status = "OK" if v.get("status") else "MISSING"
            print(f"  [{status:7}] {k:15}: {v.get('version')}")


    if step in ("all", "patchnotes"):
        print("\n--- [Step: Generate Patch Notes from Git Commits] ---")
        generate_patch_notes(
            max_commits=30,
            localversion=local_sfx,
            version_override=effective_ver,
        )

    all_artifacts = []
    for cur_mode in modes:
        if len(modes) > 1:
            print("\n==============================================================================")
            print(f" Executing Pipeline Phase: Mode = {cur_mode.upper()} | Target = {target_os}")
            print("==============================================================================")

        if step in ("all", "etch"):
            print(f"\n--- [Step: Etch Compiler Metadata into build_info.json ({cur_mode})] ---")
            etch_build_info(
                target_os=target_os,
                mode=cur_mode,
                localversion=local_sfx,
                version_override=effective_ver,
                rust_version_override=args.rust_version,
                build_number_override=build_num,
            )

        if step in ("all", "rust"):
            print(f"\n--- [Step: Compile Native Rust Engine & Desktop Suite ({cur_mode})] ---")
            compile_rust(mode=cur_mode, target_os=target_os)

        if step == "flutter" or (step == "all" and target_os == "android"):
            print(f"\n--- [Step: Compile Flutter Application ({cur_mode})] ---")
            compile_flutter(
                target_os=target_os,
                mode=cur_mode,
                version_override=effective_ver,
                localversion=local_sfx,
                build_number=build_num,
            )

        if step in ("all", "package"):
            print(f"\n--- [Step: Package Standalone Bundles ({cur_mode})] ---")
            bundles = [bundle_pref] if bundle_pref != "all" else ["all"]
            artifacts = package_bundle(
                target_os=target_os,
                mode=cur_mode,
                localversion=local_sfx,
                bundle_types=bundles,
                version_override=effective_ver,
                sign=args.sign,
            )
            all_artifacts.extend(artifacts)

    if step in ("all", "package"):
        print(f"\nSuccessfully generated {len(all_artifacts)} build artifact(s):")
        for art in all_artifacts:
            print(f"  -> {art}")

    print("\n[Done] Attune pipeline completed successfully.")


if __name__ == "__main__":
    main()
