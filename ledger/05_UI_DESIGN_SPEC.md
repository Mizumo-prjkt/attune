# 05 — Phase 4 UI & Design System Specification ("Liquid Material Glass")

This document contains the **mandatory execution instructions for any AI Coder Agent** building or modifying the Flutter Frontend (`Phase 4`).

Reference visual mockups are stored locally in:
- [`ledger/assets/mockup_screens_reference.png`](file:///home/miku/Documents/attune/ledger/assets/mockup_screens_reference.png) — Screen structure reference (*note mandatory modifications in Sections 3 & 4 below!*).
- [`ledger/assets/mockup_typography_reference.png`](file:///home/miku/Documents/attune/ledger/assets/mockup_typography_reference.png) — Typography (`Sora` + `Plus Jakarta Sans`) and fallback palette reference.

---

## 1. Typography Mandate (Strictly No System Fallback to DejaVu)

The user explicitly selected the typography pairing from [`mockup_typography_reference.png`](file:///home/miku/Documents/attune/ledger/assets/mockup_typography_reference.png). You **must** bundle or load these exact typefaces so Linux, Desktop, and Mobile never fall back to system `DejaVu Sans`:

- **Display, Headlines, Track Titles, Section Headers & Badges (`Sora`)**:
  - Weights: `w600` (SemiBold), `w700` (Bold), `w500` (Medium).
  - Letter-spacing: `-0.02em` on large titles; `+0.02em` on small uppercase section labels.
  - **Tabular Figures (`FontFeature.tabularFigures()`)**: Mandatory on all timestamps (`02:44`), countdowns, and live DSP metrics (`171.4 BPM`) so digits never jitter horizontally.
- **Body, Artist Subtitles, Queue Lists, Search & Lyrics (`Plus Jakarta Sans`)**:
  - Weights: `w400` (Regular), `w500` (Medium), `w600` (SemiBold), `w700` (Bold for active lyrics line).

---

## 2. True "Material You" Color Dynamics + Liquid Glass Materiality

The reference mockup screenshot (`mockup_screens_reference.png`) failed to properly execute **Material You Dynamic Color** and **Liquid Glass**, using hardcoded neon borders on flat dark grey boxes instead. The Coder Agent **must** fix this by implementing the **4-Layer Liquid Material Glass Stack**:

### 2.1 Material You Dynamic Color Engine (`material_color_utilities`)
1. **Live Album-Art HCT Extraction**:
   - Extract `Primary`, `Secondary`, and `Tertiary` seed tones in perceptual **HCT (Hue-Chroma-Tone)** space from the active track's cover art using `QuantizerCelebi` + `Score` from `material_color_utilities`.
   - Generate the active `ColorScheme` using Material 3's `SchemeVibrant` (default, giving rich, luminous tonal accents derived from the album art) or `SchemeTonalSpot` (selectable in Settings).
2. **Transition-Synchronized Palette Morphing**:
   - When `DualDeckMixer` is executing a transition (`progress` $\in [0.0, 1.0]$), smoothly interpolate (`ColorScheme.lerp`) the entire UI palette and background ambient mesh from **Track A's HCT palette $\rightarrow$ Track B's HCT palette** in real time alongside the audio crossfade!
3. **Manual Fallback Palette**:
   - When a track has no embedded cover art—or when the user toggles off *Dynamic Cover Theming* in Settings—use the fallback HCT seeds from the style sheet:
     - Primary Seed: `#2DD4BF` (Prismatic Mint/Teal)
     - Secondary Seed: `#A855F7` (Electric Violet)
     - Tertiary Seed: `#FB7185` (Coral Rose)
     - Deep Canvas Base: `#090A10`

### 2.2 Walled Garden Music Algorithm-Style "Liquid Glass" Surface Architecture
Never render flat opaque dark grey cards (`#14161F`). Every card, pill, navigation bar, and panel must use a shared `LiquidGlassContainer` widget composed of:
1. **Living Ambient Artwork Mesh (Background Layer)**:
   - Behind the entire scaffold sits an animated canvas rendering 4–5 heavily diffused (`sigma: 90px`) radial color orbs sampled from the active HCT palette (`primary`, `secondary`, `tertiary`, `primaryContainer`), gently breathing with playback state. This gives the glass surfaces real underlying color and light to refract.
2. **Frosted Backdrop Blur (`BackdropFilter`)**:
   - `ImageFilter.blur(sigmaX: lensBlur, sigmaY: lensBlur)` where `lensBlur` defaults to `36.0` (configurable in Settings via the *Liquid Glass Translucency* lens selector: `16px Subtle Refract`, `48px Vibrant Prism`, `64px Frosted Deep`).
3. **Tonal Glass Tint Fill**:
   - Fill with `colorScheme.surfaceContainerHigh.withValues(alpha: 0.14)` blended with a subtle `colorScheme.surfaceTint.withValues(alpha: 0.08)` overlay.
   - For active interactive pills/buttons, use `colorScheme.primary.withValues(alpha: 0.24)` with `colorScheme.onPrimaryContainer` text.
4. **Diagonal Specular Rim Lighting (`1px` Gradient Border)**:
   - Every glass surface has a `1.0px` border painted with a top-left to bottom-right `LinearGradient`:
     - Top-Left catch light: `Colors.white.withValues(alpha: 0.24)`
     - Mid-edge tonal tint: `colorScheme.primary.withValues(alpha: 0.12)`
     - Bottom-Right shadow edge: `Colors.white.withValues(alpha: 0.04)`

---

## 3. Mandatory Content & Badge Rules (Deviations from Screenshot)

### 3.1 Strictly Remove All "Dolby Atmos" / "MQA" / "Spatial 3D" Mentions
- Do **NOT** include `"Dolby Atmos"`, `"DOLBY ATMOS SYNC"`, `"HI-RES MQA"`, or `"Spatial 3D"` anywhere in *Now Playing*, *Queue*, *Lyrics*, *Library*, or *Settings*.
- Instead, inspect the actual decoded stream metadata (`format`, `sample_rate`, `codec`) and display a **Human-Friendly Audio Quality Glass Pill** on *Now Playing* and *Queue* according to this exact table:

| Stream / File Condition | Exact Badge Text to Display | Glass Token Style |
| :--- | :--- | :--- |
| **High-Quality Lossless**: `FLAC`, `WAV`, `ALAC`, or `AIFF` with `sample_rate >= 44100 Hz` (e.g., 44.1 kHz, 48 kHz, 96 kHz, 192 kHz; 16-bit or 24-bit). | **`Very Good Quality Audio`** | `primaryContainer` glass tint (`alpha: 0.22`) + `Sora w600` |
| **Mid/Low Sample Rate FLAC or Standard Clean Lossy**: `FLAC`/`WAV` with `sample_rate <= 32000 Hz` (e.g., `16 kHz` or mid-quality profile) **OR** standard open/common formats (`MP3`, `AAC`, `OGG Vorbis`, `Opus`). | **`Good Quality Audio`** | `secondaryContainer` glass tint (`alpha: 0.20`) + `Sora w600` |
| **Dolby / MQA / Unnatural or Proprietary Codecs**: Any file or container using Dolby codecs (`AC-3`, `E-AC-3`, `AC-4`, `Dolby Atmos`), `MQA`, DRM-wrapped streams, or unnatural proprietary codecs. | **`Proprietary Audio Codec`** | `tertiaryContainer` / amber glass tint (`alpha: 0.24`) + `Sora w600` |

### 3.2 Replace the Bulky "Smart Transition AI" Box with a Subtle Inline Indicator
- Do **NOT** build the giant `"Smart Transition AI"` box shown at the bottom of the Queue screenshot.
- Instead, keep transition feedback **subtle, sleek, and integrated**:
  - Render a **compact, low-profile translucent glass connector pill** between the *Now Playing* banner and the first *Up Next* track in the Queue (and inside the subtle *"Up Next"* preview bar on *Now Playing*).
  - It displays a delicate dual-wave micro-icon and concise transition info from `attune-core`'s `.ptr` recipe: e.g., `Riff Handoff • 1.4s` or `Tail Freeze • 3.5s` or `Sustain Bridge • 2.5s`.
  - **Expand on Tap**: Only when the user taps this subtle transition pill does it open a clean glass drawer/modal containing the **Dual-Deck Transition Inspector** (Deck A/B cue-zone waveforms, 3-band EQ isolator bars, vocal protection status, and reverb freeze wash curve).

---

## 4. Unified Responsive Layout: Tablet & Desktop Share the Exact Same UI Elements

To prevent duplicate code and ensure visual consistency, **Mobile, Tablet, and Desktop share 100% of the core Glass Feature Widgets**:
1. `NowPlayingStage` (Artwork card, subtle Cue-Zone wave progress bar, `Sora` title/artist, Human-Friendly Audio Quality badge, transport controls, and mini Up-Next pill).
2. `PlaylistQueueStage` (Now Playing mini-header, subtle inline transition connector pill, Up Next / Favorites / History chips, Smart Shuffle / Add Songs / Autoplay toggles, and Queued Streams list).
3. `SynchronizedLyricsStage` (walled garden music algorithm-style scrolling lyrics with blurred glass active-singing focus card and *"Instrumental Wave"* pill).
4. `MusicLibraryStage` (Search bar, Playlists/Albums/Artists filter pills, Liked Songs / Heavy Rotation / Local Audio cards, Recently Played carousel, and Made For You grid).
5. `SettingsAndAudioStage` (Active Lens profile, Liquid Glass Translucency selector, Dynamic Cover Theming toggle + manual fallback palette, Visualizer mode, Lyrics display mode, Audio Engine fidelity, and External Compute Node / Cache Sync controls).

### Adaptive Shell Breakpoints (`ResponsiveGlassScaffold`)
- **Mobile Portrait (`width < 720px`)**:
  - Renders the active stage full-screen with the **Floating Glass Bottom Pill Bar** (`Playing`, `Queue`, `Lyrics`, `Library`) and top-right Settings/Cast icon triggers, matching the 5-screen flow in [`mockup_screens_reference.png`](file:///home/miku/Documents/attune/ledger/assets/mockup_screens_reference.png).
- **Shared Tablet & Desktop Shell (`width >= 720px`)**:
  - **Tablet and Desktop use the exact same layout**:
    - **Left Floating Glass Navigation Rail**: Switches primary views (`Playing`, `Library`, `Settings`) without taking up bottom vertical space.
    - **Center-Left Primary Stage**: Displays `NowPlayingStage` (or `MusicLibraryStage` / `SettingsAndAudioStage` when selected, with a persistent glass mini-transport bar).
    - **Right Docked Contextual Glass Pane**: Permanently docks the `PlaylistQueueStage` and `SynchronizedLyricsStage` (with a sleek glass segmented switcher at the top on medium tablets `720px–1150px`, or side-by-side 3-pane split on wide tablets/desktops `>= 1150px`).
