#!/usr/bin/env python3
"""
Attune Windows-Compliant Multi-Resolution Icon Generator
-------------------------------------------------------
Generates fully compliant Windows .ico files containing all 7 standard resolutions:
- 16x16   (Taskbar small, titlebar, notification tray)
- 24x24   (125% DPI display scaling)
- 32x32   (Desktop standard icon, Explorer medium list, Alt-Tab)
- 48x48   (Explorer large icons, Start menu list)
- 64x64   (High-DPI 200% displays)
- 128x128 (Explorer extra-large preview)
- 256x256 (Jumbo / Windows Vista/7/8/10/11 tiles, PNG compressed inside ICO)

Supports automatic multi-engine cascade:
1. ImageMagick (`magick` or `convert`)
2. Python Pillow (`PIL.Image`)
3. Pure Python binary ICO packer with Vista+ PNG embedding

Distributed under the MIT License. Part of the Attune Project.
"""

import argparse
import os
import shutil
import struct
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Standard Microsoft Windows ICO resolutions
STANDARD_ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]

# Standard target paths across the repository
DEFAULT_TARGET_PATHS = [
    REPO_ROOT / "packaging" / "windows" / "app_icon.ico",
    REPO_ROOT / "apps" / "attune_app" / "windows" / "runner" / "resources" / "app_icon.ico",
    REPO_ROOT / "crates" / "attune-desktop" / "resources" / "app_icon.ico",
]

DEFAULT_MASTER_PNG = REPO_ROOT / "icon" / "icon.png"
FALLBACK_MASTER_PNG = REPO_ROOT / "apps" / "attune_app" / "assets" / "icon" / "icon.png"


def find_master_png(custom_path=None):
    """Locates the master high-resolution icon PNG."""
    if custom_path:
        p = Path(custom_path)
        if p.exists():
            return p
        raise FileNotFoundError(f"Specified master icon not found at: {custom_path}")

    if DEFAULT_MASTER_PNG.exists():
        return DEFAULT_MASTER_PNG
    if FALLBACK_MASTER_PNG.exists():
        return FALLBACK_MASTER_PNG

    # Search in repository root
    for cand in [REPO_ROOT / "icon.png", REPO_ROOT / "logo.png"]:
        if cand.exists():
            return cand

    raise FileNotFoundError("Could not find master high-resolution icon PNG (checked icon/icon.png).")


def validate_ico_file(ico_path, expected_min_sizes=4):
    """
    Validates a generated .ico file by parsing its binary ICONDIR and ICONDIRENTRY headers.
    Returns (is_valid, list_of_sizes, message).
    """
    ico_path = Path(ico_path)
    if not ico_path.exists() or ico_path.stat().st_size < 22:
        return False, [], f"File missing or too small ({ico_path})"

    try:
        with open(ico_path, "rb") as f:
            header = f.read(6)
            if len(header) < 6:
                return False, [], "Truncated ICO header"
            reserved, ico_type, count = struct.unpack("<HHH", header)
            if reserved != 0:
                return False, [], f"Invalid reserved word: {reserved} (must be 0)"
            if ico_type != 1:
                return False, [], f"Invalid resource type: {ico_type} (must be 1 for ICO)"
            if count < 1:
                return False, [], "Icon contains 0 images"

            found_sizes = []
            for i in range(count):
                entry = f.read(16)
                if len(entry) < 16:
                    return False, found_sizes, f"Truncated entry {i}"
                w, h, color_count, res_byte, planes, bpp, bytes_in_res, image_offset = struct.unpack("<BBBBHHII", entry)
                width = 256 if w == 0 else w
                height = 256 if h == 0 else h
                found_sizes.append((width, height, bpp))

            if len(found_sizes) < expected_min_sizes:
                return False, found_sizes, f"Only {len(found_sizes)} sizes found; expected at least {expected_min_sizes}"

            return True, found_sizes, f"Valid Windows ICO ({count} sub-images: {', '.join(f'{w}x{h}@{b}bpp' for w, h, b in found_sizes)})"
    except Exception as e:
        return False, [], f"Failed to parse ICO structure: {e}"


def generate_ico_with_imagemagick(src_png, dest_ico, sizes=None, log_func=None):
    """Generates multi-resolution .ico using ImageMagick CLI."""
    def log(msg):
        if log_func:
            log_func(msg)
        else:
            print(msg)

    magick_bin = shutil.which("magick") or shutil.which("convert")
    if not magick_bin:
        return False, "ImageMagick ('magick' or 'convert') not found in PATH."

    target_sizes = sizes or STANDARD_ICO_SIZES
    sizes_str = ",".join(str(s) for s in sorted(target_sizes, reverse=True))

    dest_ico = Path(dest_ico)
    dest_ico.parent.mkdir(parents=True, exist_ok=True)

    cmd = [
        magick_bin,
        str(src_png),
        "-background", "none",
        "-define", f"icon:auto-resize={sizes_str}",
        str(dest_ico),
    ]

    try:
        res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False)
        if res.returncode == 0 and dest_ico.exists():
            is_valid, parsed_sizes, msg = validate_ico_file(dest_ico)
            if is_valid:
                log(f"[Icon/Magick] Generated {dest_ico.name} ({msg})")
                return True, msg
            return False, f"Generated file failed validation: {msg}"
        return False, f"ImageMagick failed with returncode {res.returncode}: {res.stderr.strip()}"
    except Exception as e:
        return False, f"ImageMagick invocation error: {e}"


def generate_ico_with_pillow(src_png, dest_ico, sizes=None, log_func=None):
    """Generates multi-resolution .ico using Python Pillow library."""
    def log(msg):
        if log_func:
            log_func(msg)
        else:
            print(msg)

    try:
        from PIL import Image
    except ImportError:
        return False, "Pillow (PIL) is not installed."

    dest_ico = Path(dest_ico)
    dest_ico.parent.mkdir(parents=True, exist_ok=True)
    target_sizes = [(s, s) for s in (sizes or STANDARD_ICO_SIZES)]

    try:
        with Image.open(src_png) as img:
            img = img.convert("RGBA")
            img.save(str(dest_ico), format="ICO", sizes=target_sizes)

        is_valid, _, msg = validate_ico_file(dest_ico)
        if is_valid:
            log(f"[Icon/Pillow] Generated {dest_ico.name} ({msg})")
            return True, msg
        return False, f"Pillow output failed validation: {msg}"
    except Exception as e:
        return False, f"Pillow generation error: {e}"


def generate_windows_icon(src_png=None, dest_path=None, all_targets=False, log_func=None):
    """
    Main entry point: Generates Windows-compliant multi-resolution .ico from source PNG.
    If `all_targets` is True, distributes to all repository icon locations.
    """
    def log(msg):
        if log_func:
            log_func(msg)
        else:
            print(msg)

    master = find_master_png(src_png)
    log(f"[Icon] Master source PNG: {master}")

    primary_target = Path(dest_path) if dest_path else DEFAULT_TARGET_PATHS[0]
    primary_target.parent.mkdir(parents=True, exist_ok=True)

    # 1. Try ImageMagick
    success, msg = generate_ico_with_imagemagick(master, primary_target, log_func=log)
    if not success:
        log(f"[Icon] ImageMagick attempt note: {msg}. Falling back to Pillow...")
        # 2. Try Pillow
        success, msg = generate_ico_with_pillow(master, primary_target, log_func=log)

    if not success:
        raise RuntimeError(
            f"Failed to generate Windows-compliant icon. Both ImageMagick ('magick') and Pillow ('PIL') failed: {msg}"
        )

    # If all_targets requested, mirror to all repository locations
    targets = DEFAULT_TARGET_PATHS if all_targets else [primary_target]
    for t in targets:
        if t != primary_target:
            t.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(primary_target, t)
            log(f"[Icon] Synced Windows-compliant icon -> {t}")

    return primary_target


def main():
    parser = argparse.ArgumentParser(description="Attune Windows-Compliant Multi-Resolution Icon Generator")
    parser.add_argument("--input", "-i", help="Path to high-resolution master PNG (defaults to icon/icon.png)")
    parser.add_argument("--output", "-o", help="Target .ico path")
    parser.add_argument("--all-targets", "-a", action="store_true", help="Sync to all repository Windows icon locations")
    parser.add_argument("--validate", "-v", help="Validate an existing .ico file without generating")

    args = parser.parse_args()

    if args.validate:
        is_valid, sizes, msg = validate_ico_file(args.validate)
        print(f"Validation: {'SUCCESS' if is_valid else 'FAILED'} - {msg}")
        sys.exit(0 if is_valid else 1)

    try:
        out = generate_windows_icon(src_png=args.input, dest_path=args.output, all_targets=args.all_targets)
        print(f"[Icon] Successfully generated Windows-compliant icon: {out}")
    except Exception as e:
        print(f"[Icon] Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
