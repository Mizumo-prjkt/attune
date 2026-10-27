# 00 — Attune Master Knowledge Ledger

- **Project Name**: Attune
- **Current Phase**: `Phase 3 COMPLETED -> Ready for Phase 4 (Flutter Frontend & Transition Inspector)`
- **Last Updated**: 2026-10-27
- **Active Coding Status**: Phase 3 Portable Cache & External Compute Node fully implemented and verified. SQLite storage (`CacheDatabase`), portable exchange bundle (`CachePack`), background library scanner with pairwise precomputation (`LibraryScanner`), lightweight HTTP node server and sync client (`NodeServer` / `NodeClient`), and cache-integrated JIT engine (`JitPlanner::plan_cached_or_jit`) fully operational.

---

## 1. Executive Summary

**Attune** is an open-source, cross-platform music player built around a professional DJ-grade, audio-profile-aware **Transition & Remixing Engine**. Unlike standard crossfaders or simple BPM matchers, Attune analyzes the **actual audio profile** of tracks—including sustained vocal pitch/formants, local harmonic progression, beat/phrase grids, dominant instrument signatures (e.g., distorted punk guitar, brass, synth, vocal grit/growl), and macro-genre aggression (Aggressive Rock/Punk vs. Slow Rock vs. Pop)—to craft seamless, musically intelligent transitions.

Furthermore, Attune decouples **Audio Profiling / Transition Planning** from **Real-Time Playback Execution**, allowing heavy analysis to run:
1. **On-Demand (JIT)** via a 45-second Head/Tail fast-pass,
2. **In the Background** on the local device when idle, or
3. **Offloaded to an External Compute Device** (Desktop/Home Server/Cloud Worker) that exports a compact, portable reference cache (`.tap` and `.ptr` files + optional 5–8s isolated vocal/instrument stem bridge clips) for low-powered playback devices.

---

## 2. Roadmap & Phase Gates

| Phase | Milestone | Gate Criteria before Next Phase | Status |
| :--- | :--- | :--- | :--- |
| **Phase 0** | **Knowledge Ledger & Spec Lock** | Complete `/ledger/` docs and lock down core tech stack + cache options with user. | **COMPLETED** |
| **Phase 1** | **Headless DSP Sandbox (`attune-cli`)** | CLI extracts 5-layer `.tap` profiles, generates `.ptr` recipes, and renders benchmark transitions (Whitney $\rightarrow$ Mariah, Ado $\rightarrow$ Punk/Rock, Slow Rock $\rightarrow$ Pop) to WAV for listening verification. | **COMPLETED** |
| **Phase 2** | **Real-Time Dual-Deck Native Engine** | Lock-free dual-deck playback mixer executing `.ptr` time-stretch, formant pitch-shift, and 3-band EQ curves in real time + On-Demand 45s JIT Fast-Pass + real-time audio driver + hardware sample rate adaptation + vocal integrity protection. | **COMPLETED** |
| **Phase 3** | **Portable Cache & External Compute Node** | SQLite + blob cache indexing, acoustic fingerprinting, Demucs stem-bridge schema, portable CachePack export/import, and LAN HTTP REST sync between external compute worker and playback client. | **COMPLETED** |
| **Phase 4** | **Flutter Frontend & Transition Inspector** | Connect Flutter UI via FFI (`flutter_rust_bridge`), starting with a visual waveform/curve Transition Inspector and expanding into the full music player UI. | **READY TO START** |

---

## 3. Architectural Decisions Status

| ID | Decision Topic | Locked Architecture Baseline | Status |
| :--- | :--- | :--- | :--- |
| **ADR-001** | Development Order | Algorithm & Headless CLI first; Flutter UI skeleton second. | **LOCKED** |
| **ADR-002** | Native Engine Language | Hybrid **Rust** orchestrator (`attune-core` via `flutter_rust_bridge`) wrapping **C/C++ DSP primitives** (`Signalsmith Stretch` / `RubberBand` + `miniaudio`). | **LOCKED** |
| **ADR-003** | Compute Offloading & Cache | 3-Tier Compute (On-Demand 45s Head/Tail, Local Background, External Worker Node) producing portable `.tap` and `.ptr` caches. | **LOCKED** |
| **ADR-004** | 5-Layer Audio Profiling | DSP + Ultra-Light Quantized Audio Tagging Model (~2–4 MB ONNX on device + deeper model on External Node) for Pitch/Formant Sustain, Chroma CQT, Beat/Phrase, 4-Band Energy, and Instrument/Vibe/Aggression (Ado/Punk/Rock/Pop). | **LOCKED** |
| **ADR-005** | External Node Stem Bridges | Export `.tap`/`.ptr` metadata caches by default, plus optionally pre-render short 5–8s isolated vocal/instrument stem bridges (via Demucs) for hero transitions. | **LOCKED** |
| **ADR-006** | Audio Ingestion & Spectral Extraction | Pure Rust decoding pipeline via `symphonia`, WAV rendering via `hound`, and RealFFT for 20Hz frame extraction. | **LOCKED** |
| **ADR-007** | Silence Gating & Boundary Clipping | Dynamic RMS -45 dBFS silence gating and strict audible boundary clipping in `AudioProfiler` and `TransitionPlanner` to eliminate dead air / pre-gap gaps in transitions. | **LOCKED** |
| **ADR-008** | Real-Time Dual-Deck Engine & cpal Output Driver | Real-time sample-accurate `DualDeckMixer` with 3-band isolator biquad EQ, Hermite interpolation resampler, reverb freeze wash, soft limiter, and `cpal` device playback. | **LOCKED** |
| **ADR-009** | Hardware Sample Rate Adaptation & 1-Bar Phrased Handoff | Fractional cubic Hermite playhead resampling ($\Delta \text{playhead} = \frac{F_{\text{track}}}{F_{\text{device}}} \times \text{stretch} \times \text{pitch}$) ensuring exact 1.000x speed and 100% native pitch across mismatched hardware DAC rates, plus 1-bar (4-beat) musical riff handoffs. | **LOCKED** |
| **ADR-010** | Vocal Integrity & Reverb Spillover | Multi-layer vocal activity inspection in `CueZone`, anchor cuts at/after final vocal resolution, flat mid/high EQ during active singing, and post-transition reverb freeze decay. | **LOCKED** |
| **ADR-011** | Portable Cache, CachePack & Node Sync Protocol | Bundled SQLite cache repository (`CacheDatabase`), portable `.pack.json` bundle (`CachePack`), background scanner (`LibraryScanner`), REST HTTP sync server (`tiny_http`), and sync client (`ureq`). | **LOCKED** |
| **ADR-012** | Liquid Material Glass UI, Subtle Transition Pills, Honest Audio Quality Badges & Shared Tablet/Desktop Shell | `Sora` + `Plus Jakarta Sans` typography, Material You HCT dynamic cover color + Apple `BackdropFilter` Liquid Glass (`ledger/05_UI_DESIGN_SPEC.md`), subtle inline transition connector (no bulky AI card), human-friendly audio quality pills (`Very Good Quality Audio`, `Good Quality Audio`, `Proprietary Audio Codec`—zero Dolby Atmos hype), and shared Tablet/Desktop multi-pane layout (`>= 720px`). | **LOCKED** |

---

## 4. Session Handoff Checklist (For Next AI Session — Phase 4 Coder Agent)

1. **MANDATORY FIRST READ**: Read [`ledger/05_UI_DESIGN_SPEC.md`](file:///home/miku/Documents/attune/ledger/05_UI_DESIGN_SPEC.md) and inspect the reference images in [`ledger/assets/mockup_screens_reference.png`](file:///home/miku/Documents/attune/ledger/assets/mockup_screens_reference.png) and [`ledger/assets/mockup_typography_reference.png`](file:///home/miku/Documents/attune/ledger/assets/mockup_typography_reference.png).
2. **Execute Phase 4 (Flutter Frontend & "Liquid Material Glass" UI)**:
   - Scaffold the Flutter application (e.g., `apps/attune_app` or root Flutter module) connected to `attune-core` via FFI (`flutter_rust_bridge` or native C FFI shim).
   - **Typography**: Configure `Sora` (Headlines, Titles, Pill Badges, Tabular Timestamps) and `Plus Jakarta Sans` (Body, Subtitles, Queue, Lyrics) so Linux/Desktop/Mobile never fall back to `DejaVu Sans`.
   - **Theme & Glass Materiality**: Implement the **4-Layer Liquid Material Glass Stack** (`AmbientArtworkMesh` background + `BackdropFilter` Gaussian blur + Material You HCT dynamic `ColorScheme` extracted from album art with transition-synced palette morphing + `1px` diagonal specular rim border).
   - **Audio Quality Badge Rules (Strictly No "Dolby Atmos" / "MQA" Mentions)**:
     - High-quality lossless (`FLAC`/`WAV`/`ALAC` $\ge 44.1\text{ kHz}$): display **`Very Good Quality Audio`**.
     - Mid/low sample rate `FLAC` ($\le 32\text{ kHz}$, e.g. $16\text{ kHz}$) or standard lossy (`MP3`/`AAC`/`OGG`/`Opus`): display **`Good Quality Audio`**.
     - Dolby (`AC-3`/`E-AC-3`/`AC-4`/`Atmos`), `MQA`, or unnatural codecs: display **`Proprietary Audio Codec`**.
   - **Subtle Transition Indicator**: Do **not** build the large bottom `"Smart Transition AI"` box from the screenshot; instead render a subtle, compact inline glass pill between *Now Playing* and *Up Next* (e.g., `Riff Handoff • 1.4s`) that opens the expandable **Dual-Deck Transition Inspector** drawer when tapped.
   - **Shared Tablet & Desktop Shell (`>= 720px`)**: Use a single unified set of glass stage widgets shared across Mobile (`< 720px` bottom glass pill bar) and Tablet/Desktop (`>= 720px` left glass navigation rail + multi-pane stage).
3. Keep updating [`00_MASTER_LEDGER.md`](file:///home/miku/Documents/attune/ledger/00_MASTER_LEDGER.md) and record any new decisions in [`01_ADR_LOG.md`](file:///home/miku/Documents/attune/ledger/01_ADR_LOG.md).




