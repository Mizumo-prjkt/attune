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
    """Reads app version from apps/attune_app/pubspec.yaml."""
    pubspec_path = REPO_ROOT / "apps" / "attune_app" / "pubspec.yaml"
    if not pubspec_path.exists():
        return "0.1.0", "1"
    try:
        content = pubspec_path.read_text(encoding="utf-8")
        match = re.search(r"^version:\s*([0-9]+\.[0-9]+\.[0-9]+)(?:\+([0-9]+))?", content, re.MULTILINE)
        if match:
            return match.group(1), match.group(2) or "1"
    except Exception:
        pass
    return "0.1.0", "1"


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
        res = subprocess.check_output(cmd, text=True, stderr=subprocess.DEVNULL).strip()
        return res.splitlines()[0] if res else None
    except Exception:
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

    return results


def etch_build_info(mode="release", localversion="", version_override="", rust_version_override=""):
    """
    Etches compiler versions, environment variables, build flags, commit hash,
    and MIT licensing into apps/attune_app/assets/build_info.json.
    """
    git_info = get_git_info()
    pub_ver, build_num = get_pubspec_version()
    version = version_override if version_override else pub_ver
    rust_ffi_version = rust_version_override if rust_version_override else get_rust_crate_version("attune-ffi")

    toolchains = assess_toolchains()

    build_info = {
        "application_name": "Attune",
        "version": version,
        "build_number": build_num,
        "localversion": localversion,
        "full_version": f"{version}{localversion}",
        "rust_ffi_version": rust_ffi_version,
        "git_commit": git_info["git_commit"],
        "git_commit_full": git_info["git_commit_full"],
        "git_branch": git_info["git_branch"],
        "git_dirty": git_info["git_dirty"],
        "build_mode": mode.capitalize(),
        "build_timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "target_os": platform.system().lower(),
        "target_arch": platform.machine(),
        "python_version": toolchains["python"]["version"],
        "compiler_rustc": toolchains["rustc"]["version"],
        "compiler_cargo": toolchains["cargo"]["version"],
        "compiler_flutter": toolchains["flutter"]["version"],
        "compiler_dart": toolchains["dart"]["version"],
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
    print(f"[Etch] Successfully etched compiler and build metadata into {out_file} (v{version}{localversion})")
    return build_info


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
        f"Attune Release Patch Notes - Version {full_ver}\n"
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


def compile_rust(mode="release", log_callback=None):
    """Compiles the native Rust DSP engine crate (attune-ffi)."""
    def log(msg):
        if log_callback:
            log_callback(msg)
        else:
            print(msg)

    log(f"[Rust] Compiling attune-ffi in {mode} mode...")
    cmd = ["cargo", "build", "-p", "attune-ffi"]
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
    )

    for line in proc.stdout:
        log(f"  [cargo] {line.rstrip()}")

    proc.wait()
    if proc.returncode != 0:
        raise RuntimeError(f"Cargo build failed with exit code {proc.returncode}")

    target_dir = REPO_ROOT / "target" / mode.lower()
    system = platform.system()

    if system == "Linux":
        lib_name = "libattune_engine.so"
    elif system == "Windows":
        lib_name = "attune_engine.dll"
    elif system == "Darwin":
        lib_name = "libattune_engine.dylib"
    else:
        lib_name = "libattune_engine.so"

    lib_path = target_dir / lib_name
    if not lib_path.exists():
        raise FileNotFoundError(f"Native library was not produced at: {lib_path}")

    # Copy native library to apps/attune_app/lib/ for relative lookup
    dest_lib_dir = REPO_ROOT / "apps" / "attune_app" / "lib"
    dest_lib_dir.mkdir(parents=True, exist_ok=True)
    dest_lib = dest_lib_dir / lib_name
    shutil.copy2(lib_path, dest_lib)
    log(f"[Rust] Copied native library to relative lookup path: {dest_lib}")
    return lib_path


def compile_flutter(target_os="linux", mode="release", log_callback=None):
    """Compiles Flutter desktop application."""
    def log(msg):
        if log_callback:
            log_callback(msg)
        else:
            print(msg)

    log(f"[Flutter] Compiling Flutter app for {target_os} in {mode} mode...")
    flutter_dir = REPO_ROOT / "apps" / "attune_app"

    cmd = ["flutter", "build", target_os.lower(), f"--{mode.lower()}"]
    proc = subprocess.Popen(
        cmd,
        cwd=flutter_dir,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )

    for line in proc.stdout:
        log(f"  [flutter] {line.rstrip()}")

    proc.wait()
    if proc.returncode != 0:
        raise RuntimeError(f"Flutter build failed with exit code {proc.returncode}")

    # After Flutter compilation, guarantee native library is bundled inside the bundle's lib/ directory
    if target_os.lower() == "linux":
        bundle_dir = flutter_dir / "build" / "linux" / "x64" / mode.lower() / "bundle"
        rust_lib = REPO_ROOT / "target" / mode.lower() / "libattune_engine.so"
        if bundle_dir.exists() and rust_lib.exists():
            bundle_lib_dir = bundle_dir / "lib"
            bundle_lib_dir.mkdir(parents=True, exist_ok=True)
            shutil.copy2(rust_lib, bundle_lib_dir / "libattune_engine.so")
            log(f"[Flutter] Bundled {rust_lib.name} into {bundle_lib_dir}")
    elif target_os.lower() == "windows":
        runner_dir = flutter_dir / "build" / "windows" / "x64" / "runner" / mode.capitalize()
        rust_dll = REPO_ROOT / "target" / mode.lower() / "attune_engine.dll"
        if runner_dir.exists() and rust_dll.exists():
            runner_lib_dir = runner_dir / "lib"
            runner_lib_dir.mkdir(parents=True, exist_ok=True)
            shutil.copy2(rust_dll, runner_lib_dir / "attune_engine.dll")
            shutil.copy2(rust_dll, runner_dir / "attune_engine.dll")
            log(f"[Flutter] Bundled {rust_dll.name} into {runner_dir}")

    log(f"[Flutter] Build completed successfully for {target_os}.")


def package_bundle(target_os="linux", mode="release", localversion="", bundle_types=None, version_override="", log_callback=None):
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
        # 1. Portable .tar.gz bundle
        if "all" in bundle_types or "zip" in bundle_types or "tar" in bundle_types:
            tar_name = f"Attune-{full_ver}-Linux-x86_64-{mode.capitalize()}.tar.gz"
            tar_path = dist_dir / tar_name
            bundle_dir = REPO_ROOT / "apps" / "attune_app" / "build" / "linux" / "x64" / mode.lower() / "bundle"
            if bundle_dir.exists():
                log(f"[Package] Creating portable tarball {tar_path}...")
                subprocess.check_call(
                    ["tar", "-czf", str(tar_path), "-C", str(bundle_dir.parent), "bundle"],
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
        # 1. Portable .zip bundle
        runner_dir = REPO_ROOT / "apps" / "attune_app" / "build" / "windows" / "x64" / "runner" / mode.capitalize()
        if runner_dir.exists() and ("all" in bundle_types or "zip" in bundle_types):
            zip_name = f"Attune-{full_ver}-Windows-x86_64-{mode.capitalize()}.zip"
            zip_path = dist_dir / zip_name
            log(f"[Package] Creating portable zip {zip_path}...")
            shutil.make_archive(str(dist_dir / f"Attune-{full_ver}-Windows-x86_64-{mode.capitalize()}"), "zip", runner_dir)
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
                )
                for line in proc.stdout:
                    log(f"  [iscc] {line.rstrip()}")
                proc.wait()
            else:
                log("[Package] ISCC compiler not found; skipping Windows setup creation.")

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

    # Version IDs & Configuration Card
    config_card = tk.LabelFrame(
        main_frame,
        text=" Component Version IDs & Kernel-Style Localversion ",
        bg=card_color,
        fg=accent_color,
        font=("Segoe UI", 10, "bold"),
        padx=14,
        pady=10,
    )
    config_card.pack(fill=tk.X, pady=(0, 10))

    cur_pub_ver, cur_pub_build = get_pubspec_version()
    cur_rust_ver = get_rust_crate_version("attune-ffi")

    # Grid rows
    tk.Label(config_card, text="Flutter App Version:", bg=card_color, fg=fg_color).grid(row=0, column=0, sticky="w", pady=4)
    ent_flutter_ver = tk.Entry(config_card, width=12, bg="#2D3748", fg=fg_color, insertbackground=fg_color)
    ent_flutter_ver.insert(0, cur_pub_ver)
    ent_flutter_ver.grid(row=0, column=1, sticky="w", padx=6, pady=4)

    tk.Label(config_card, text="Rust Crate Version:", bg=card_color, fg=fg_color).grid(row=0, column=2, sticky="w", padx=(16, 0), pady=4)
    ent_rust_ver = tk.Entry(config_card, width=12, bg="#2D3748", fg=fg_color, insertbackground=fg_color)
    ent_rust_ver.insert(0, cur_rust_ver)
    ent_rust_ver.grid(row=0, column=3, sticky="w", padx=6, pady=4)

    tk.Label(config_card, text="Localversion String:", bg=card_color, fg=fg_color).grid(row=1, column=0, sticky="w", pady=4)
    ent_localver = tk.Entry(config_card, width=20, bg="#2D3748", fg=fg_color, insertbackground=fg_color)
    ent_localver.insert(0, "-rc1")
    ent_localver.grid(row=1, column=1, columnspan=2, sticky="w", padx=6, pady=4)

    tk.Label(config_card, text="(e.g., -git, -custom, -beta like Linux CONFIG_LOCALVERSION)", bg=card_color, fg="#718096", font=("Segoe UI", 8)).grid(row=1, column=2, columnspan=2, sticky="w", padx=(10, 0))

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
    cb_target = ttk.Combobox(target_card, textvariable=target_os_var, values=["linux", "windows"], width=10, state="readonly")
    cb_target.grid(row=0, column=1, sticky="w", padx=6, pady=2)

    tk.Label(target_card, text="Build Mode:", bg=card_color, fg=fg_color).grid(row=0, column=2, sticky="w", padx=(16, 0), pady=2)
    tk.Radiobutton(target_card, text="Release", variable=mode_var, value="release", bg=card_color, fg=fg_color, selectcolor="#2D3748").grid(row=0, column=3, sticky="w", padx=4)
    tk.Radiobutton(target_card, text="Debug", variable=mode_var, value="debug", bg=card_color, fg=fg_color, selectcolor="#2D3748").grid(row=0, column=4, sticky="w", padx=4)

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
        loc = ent_localver.get().strip()
        ver = ent_flutter_ver.get().strip()
        rver = ent_rust_ver.get().strip()
        m = mode_var.get()
        info = etch_build_info(mode=m, localversion=loc, version_override=ver, rust_version_override=rver)
        gui_log(f"Etched build info: v{info['full_version']} ({info['build_mode']}) commit {info['git_commit']}")

    def on_patch_notes():
        loc = ent_localver.get().strip()
        ver = ent_flutter_ver.get().strip()
        notes = generate_patch_notes(max_commits=25, localversion=loc, version_override=ver)
        gui_log(f"Generated patch_notes.txt with {len(notes.splitlines())} lines.")

    def on_compile_rust():
        m = mode_var.get()
        try:
            lib = compile_rust(mode=m, log_callback=gui_log)
            gui_log(f"✓ Rust build succeeded: {lib}")
        except Exception as e:
            gui_log(f"✗ Rust build error: {e}")

    def on_compile_flutter():
        m = mode_var.get()
        t = target_os_var.get()
        try:
            compile_flutter(target_os=t, mode=m, log_callback=gui_log)
            gui_log(f"✓ Flutter build succeeded for {t} ({m}).")
        except Exception as e:
            gui_log(f"✗ Flutter build error: {e}")

    def on_full_build():
        loc = ent_localver.get().strip()
        ver = ent_flutter_ver.get().strip()
        rver = ent_rust_ver.get().strip()
        m = mode_var.get()
        t = target_os_var.get()
        gui_log("==================================================")
        gui_log(f"Starting Full Attune Build: {t} {m} v{ver}{loc}")
        gui_log("==================================================")
        try:
            etch_build_info(mode=m, localversion=loc, version_override=ver, rust_version_override=rver)
            generate_patch_notes(max_commits=25, localversion=loc, version_override=ver)
            compile_rust(mode=m, log_callback=gui_log)
            compile_flutter(target_os=t, mode=m, log_callback=gui_log)
            artifacts = package_bundle(target_os=t, mode=m, localversion=loc, version_override=ver, log_callback=gui_log)
            gui_log(f"🎉 Full build & packaging completed! Artifacts: {artifacts}")
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
        choices=["linux", "windows", "all"],
        default=platform.system().lower(),
        help="Target platform operating system (default: current host OS)",
    )
    parser.add_argument(
        "--mode",
        choices=["release", "debug"],
        default="release",
        help="Compilation mode (default: release)",
    )
    parser.add_argument(
        "--version",
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
        help="Kernel-style localversion suffix string (e.g. -git, -rc1, -miku)",
    )
    parser.add_argument(
        "--step",
        choices=["all", "assess", "etch", "patchnotes", "rust", "flutter", "package"],
        default="all",
        help="Specific build pipeline step to execute (default: all)",
    )
    parser.add_argument(
        "--bundle",
        choices=["all", "appimage", "innosetup", "zip", "none"],
        default="all",
        help="Packaging artifact formats to produce (default: all)",
    )

    args = parser.parse_args()

    # Determine whether to launch GUI:
    # If explicitly requested with --gui, or if no CLI step flags were passed AND DISPLAY is present and not --no-gui
    has_display = bool(os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY") or platform.system() == "Windows")
    if args.gui or (not args.no_gui and len(sys.argv) == 1 and has_display):
        launch_gui()
        return

    # CLI Pipeline Execution
    pub_ver, _ = get_pubspec_version()
    effective_ver = args.version if args.version else pub_ver
    print("==============================================================================")
    print(" Attune Build & Packaging Orchestrator (Headless CLI)")
    print(f" Target: {args.target} | Mode: {args.mode} | Version: '{effective_ver}' | Localversion: '{args.localversion}'")
    print(" Distributed under the MIT License")
    print("==============================================================================")

    step = args.step

    if step in ("all", "assess"):
        print("\n--- [Step: Assess Toolchains] ---")
        toolchains = assess_toolchains()
        for k, v in toolchains.items():
            status = "✓ OK" if v.get("status") else "✗ MISSING"
            print(f"  [{status}] {k:15}: {v.get('version')}")

    if step in ("all", "etch"):
        print("\n--- [Step: Etch Compiler Metadata into build_info.json] ---")
        etch_build_info(
            mode=args.mode,
            localversion=args.localversion,
            version_override=args.version,
            rust_version_override=args.rust_version,
        )

    if step in ("all", "patchnotes"):
        print("\n--- [Step: Generate Patch Notes from Git Commits] ---")
        generate_patch_notes(
            max_commits=30,
            localversion=args.localversion,
            version_override=args.version,
        )

    if step in ("all", "rust"):
        print("\n--- [Step: Compile Native Rust DSP Engine] ---")
        compile_rust(mode=args.mode)

    if step in ("all", "flutter"):
        print("\n--- [Step: Compile Flutter Application] ---")
        compile_flutter(target_os=args.target, mode=args.mode)

    if step in ("all", "package"):
        print("\n--- [Step: Package Standalone Bundles] ---")
        bundles = [args.bundle] if args.bundle != "all" else ["all"]
        artifacts = package_bundle(
            target_os=args.target,
            mode=args.mode,
            localversion=args.localversion,
            bundle_types=bundles,
            version_override=args.version,
        )
        print(f"\nSuccessfully generated {len(artifacts)} build artifact(s):")
        for art in artifacts:
            print(f"  -> {art}")

    print("\n[Done] Attune pipeline completed successfully.")


if __name__ == "__main__":
    main()
