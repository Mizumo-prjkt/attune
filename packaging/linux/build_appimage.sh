#!/usr/bin/env bash
# Attune Linux AppImage Packaging Script
# Assembles AppDir with Native Rust attune-desktop, signed dynamic C-ABI components,
# and anti-tamper integrity manifest to produce a standalone AppImage.

set -euo pipefail

MODE="release"
VERSION=""
LOCALVERSION=""
OUTPUT_DIR="dist/linux"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode)
            MODE="$2"
            shift 2
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --localversion)
            LOCALVERSION="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

cd "$REPO_ROOT"

# Determine app version from pubspec.yaml if not explicitly supplied
if [ -z "$VERSION" ]; then
    VERSION=$(grep '^version:' apps/attune_app/pubspec.yaml | head -n 1 | awk '{print $2}' | cut -d'+' -f1)
fi
FULL_VERSION="${VERSION}${LOCALVERSION}"

echo "=========================================================="
echo " Packaging Attune AppImage: v${FULL_VERSION} (${MODE})"
echo "=========================================================="

NATIVE_BIN="target/${MODE}/attune-desktop"
BUNDLE_DIR="apps/attune_app/build/linux/x64/${MODE}/bundle"

if [ ! -f "$NATIVE_BIN" ] && [ ! -d "$BUNDLE_DIR" ]; then
    echo "ERROR: Neither Native Rust binary ($NATIVE_BIN) nor Flutter bundle ($BUNDLE_DIR) found."
    echo "Please compile Attune Desktop first: cargo build -p attune-desktop $([ "$MODE" == "release" ] && echo "--release")"
    exit 1
fi

RUST_LIB="target/${MODE}/libattune_engine.so"
if [ ! -f "$RUST_LIB" ]; then
    echo "ERROR: Compiled Rust engine library not found at: $RUST_LIB"
    echo "Please compile the native library first: cargo build -p attune-ffi $([ "$MODE" == "release" ] && echo "--release")"
    exit 1
fi

APPDIR="build/AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/bin/lib"
mkdir -p "$APPDIR/usr/lib"
mkdir -p "$APPDIR/lib"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$OUTPUT_DIR"

# 1. Copy Native Rust Desktop Executable (primary) or legacy bundle
if [ -f "$NATIVE_BIN" ]; then
    echo "-> Packaging Native Rust Desktop executable ($NATIVE_BIN)..."
    cp "$NATIVE_BIN" "$APPDIR/usr/bin/attune-desktop"
    chmod +x "$APPDIR/usr/bin/attune-desktop"
elif [ -d "$BUNDLE_DIR" ]; then
    echo "-> Copying legacy Flutter bundle assets..."
    cp -r "${BUNDLE_DIR}/"* "$APPDIR/usr/bin/"
    chmod +x "$APPDIR/usr/bin/attune_app" 2>/dev/null || true
fi

# 2. Bundle all dynamic C-ABI components (.so) & anti-tamper manifest
echo "-> Packaging native Rust DSP engine and dynamic C-ABI components..."
for SO_NAME in libattune_engine.so libattune_watchguard.so libattune_procmgr.so libattune_music_metadata.so libattune_music_metadata_changer.so; do
    SO_SRC="target/${MODE}/${SO_NAME}"
    if [ -f "$SO_SRC" ]; then
        cp "$SO_SRC" "$APPDIR/usr/lib/${SO_NAME}"
        cp "$SO_SRC" "$APPDIR/lib/${SO_NAME}"
        cp "$SO_SRC" "$APPDIR/usr/bin/lib/${SO_NAME}"
        echo "   + Bundled ${SO_NAME}"
    fi
done

if [ -f "target/${MODE}/attune_integrity.json" ]; then
    cp "target/${MODE}/attune_integrity.json" "$APPDIR/usr/bin/attune_integrity.json"
    cp "target/${MODE}/attune_integrity.json" "$APPDIR/usr/bin/lib/attune_integrity.json"
    cp "target/${MODE}/attune_integrity.json" "$APPDIR/usr/lib/attune_integrity.json"
    echo "   + Bundled attune_integrity.json anti-tamper manifest"
fi

# 2b. Bundle cross-distro non-base shared libraries (libxdo, libXtst, libXinerama)
# and create SONAME aliases (libxdo.so.3 & libxdo.so.4) so binaries built on Ubuntu/Debian
# run on Arch/Fedora and vice-versa even when xdotool is not installed on the target host.
echo "-> Bundling cross-distro compatibility shared libraries (libxdo, libXtst, libXinerama)..."
if [ -f "$NATIVE_BIN" ]; then
    XDO_PATH=$(ldd "$NATIVE_BIN" 2>/dev/null | awk '/libxdo\.so/ {print $3}' | head -n 1 || true)
    if [ -z "$XDO_PATH" ] || [ ! -f "$XDO_PATH" ]; then
        for CAND in /usr/lib/x86_64-linux-gnu/libxdo.so* /usr/lib64/libxdo.so* /usr/lib/libxdo.so*; do
            if [ -f "$CAND" ]; then
                XDO_PATH="$CAND"
                break
            fi
        done
    fi
    if [ -n "$XDO_PATH" ] && [ -f "$XDO_PATH" ]; then
        for ALIAS in libxdo.so libxdo.so.3 libxdo.so.4; do
            cp -L "$XDO_PATH" "$APPDIR/usr/lib/${ALIAS}"
            cp -L "$XDO_PATH" "$APPDIR/lib/${ALIAS}"
            cp -L "$XDO_PATH" "$APPDIR/usr/bin/lib/${ALIAS}"
        done
        echo "   + Bundled ${XDO_PATH} as libxdo.so.{3,4}"
    fi

    for EXTRA_LIB in libXtst.so.6 libXinerama.so.1; do
        EXTRA_PATH=$(ldd "$NATIVE_BIN" 2>/dev/null | awk -v lib="$EXTRA_LIB" '$1 == lib {print $3}' | head -n 1 || true)
        if [ -z "$EXTRA_PATH" ] || [ ! -f "$EXTRA_PATH" ]; then
            for CAND in "/usr/lib/x86_64-linux-gnu/${EXTRA_LIB}" "/usr/lib64/${EXTRA_LIB}" "/usr/lib/${EXTRA_LIB}"; do
                if [ -f "$CAND" ]; then
                    EXTRA_PATH="$CAND"
                    break
                fi
            done
        fi
        if [ -n "$EXTRA_PATH" ] && [ -f "$EXTRA_PATH" ]; then
            cp -L "$EXTRA_PATH" "$APPDIR/usr/lib/${EXTRA_LIB}"
            cp -L "$EXTRA_PATH" "$APPDIR/lib/${EXTRA_LIB}"
            cp -L "$EXTRA_PATH" "$APPDIR/usr/bin/lib/${EXTRA_LIB}"
            echo "   + Bundled ${EXTRA_LIB} (${EXTRA_PATH})"
        fi
    done
fi

# 3. Desktop Entry and Icons
echo "-> Setting up desktop entry, icons and AppRun..."
cp packaging/linux/attune.desktop "$APPDIR/attune.desktop"
cp packaging/linux/attune.desktop "$APPDIR/usr/share/applications/attune.desktop"
cp packaging/linux/attune.png "$APPDIR/attune.png"
cp packaging/linux/attune.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/attune.png"
cp packaging/linux/AppRun "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun"

# 4. Create AppImage using appimagetool
APPIMAGE_BIN="${OUTPUT_DIR}/Attune-${FULL_VERSION}-x86_64-${MODE}.AppImage"

TOOL="appimagetool"
if ! command -v appimagetool &> /dev/null; then
    TOOL_PATH="/tmp/appimagetool-x86_64.AppImage"
    if [ ! -f "$TOOL_PATH" ]; then
        echo "-> appimagetool not found in PATH. Downloading standalone appimagetool..."
        curl -fsSL -o "$TOOL_PATH" "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" || \
        curl -fsSL -o "$TOOL_PATH" "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
        chmod +x "$TOOL_PATH"
    fi
    TOOL="$TOOL_PATH"
fi

echo "-> Generating AppImage: $APPIMAGE_BIN"
ARCH=x86_64 "$TOOL" "$APPDIR" "$APPIMAGE_BIN"

echo "=========================================================="
echo " Successfully created AppImage:"
echo " ${APPIMAGE_BIN}"
echo " Size: $(du -h "${APPIMAGE_BIN}" | awk '{print $1}')"
echo "=========================================================="
