#!/usr/bin/env bash
# Attune Linux AppImage Packaging Script
# Assembles AppDir with relative lib resolution and produces standalone AppImage

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

BUNDLE_DIR="apps/attune_app/build/linux/x64/${MODE}/bundle"
if [ ! -d "$BUNDLE_DIR" ]; then
    echo "ERROR: Flutter Linux bundle not found at: $BUNDLE_DIR"
    echo "Please build the Flutter Linux app first: flutter build linux --${MODE}"
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
mkdir -p "$APPDIR/usr/lib"
mkdir -p "$APPDIR/lib"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$OUTPUT_DIR"

# 1. Copy Flutter Bundle Executable & Data
echo "-> Copying Flutter bundle assets..."
cp -r "${BUNDLE_DIR}/"* "$APPDIR/usr/bin/"

# 2. Bundle native engine library into relative lib dirs
echo "-> Packaging native Rust DSP engine (relative lib)..."
cp "$RUST_LIB" "$APPDIR/usr/lib/libattune_engine.so"
cp "$RUST_LIB" "$APPDIR/lib/libattune_engine.so"
mkdir -p "$APPDIR/usr/bin/lib"
cp "$RUST_LIB" "$APPDIR/usr/bin/lib/libattune_engine.so"

# Also copy Flutter runtime libraries to usr/lib
if [ -d "${BUNDLE_DIR}/lib" ]; then
    cp -r "${BUNDLE_DIR}/lib/"* "$APPDIR/usr/lib/" 2>/dev/null || true
fi

# 3. Desktop Entry and Icons
echo "-> Setting up desktop entry, icons and AppRun..."
cp packaging/linux/attune.desktop "$APPDIR/attune.desktop"
cp packaging/linux/attune.desktop "$APPDIR/usr/share/applications/attune.desktop"
cp packaging/linux/attune.png "$APPDIR/attune.png"
cp packaging/linux/attune.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/attune.png"
cp packaging/linux/AppRun "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun"
chmod +x "$APPDIR/usr/bin/attune_app"

# 4. Create AppImage using appimagetool
APPIMAGE_BIN="${OUTPUT_DIR}/Attune-${FULL_VERSION}-x86_64-${MODE}.AppImage"

# Check for appimagetool in PATH or local cache
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
