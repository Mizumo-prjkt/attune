# 00 — Attune Master Knowledge Ledger

- **Project Name**: Attune
- **Current Phase**: `Phase 5 COMPLETED -> All Core MVP Milestones Achieved (Ready for Packaging & Deployment)`
- **Last Updated**: 2026-10-27
- **Active Coding Status**: Walled Garden Music Algorithm 4-Stage DSP Lookahead Pipeline (Archetype 3: `EqSwap`) fully implemented and verified across real library tracks (*Happy Synthesizer* -> *Hello-How are you*, *Luka Luka★Night Fever* -> *Rabbit Hole*). Features dynamic cue-point detection on permanent energy drop (< -18 dB) clamped to vocal phrase completion, constant-power gain curves ($G_A^2 + G_B^2 = 1.0$), 180 Hz Linkwitz-Riley 3-band DJ bass swap, conditional beat-grid alignment ($\le 6\%$ BPM gate), and Camelot harmonic dissonance damping. Procedural 8-bit synthetic note generation completely scrapped. 159 genuine local tracks ingested, real-time dual-deck hardware playback controls active, all 10 Flutter unit/widget tests and all 17 Rust workspace tests pass with 0 errors and 0 warnings.

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
| **Phase 4** | **Flutter Frontend & Transition Inspector** | Multi-Activity Android architecture for native Predictive Back, "Liquid Material Glass" design system, honest audio quality badges, subtle transition connector pill, unified responsive scaffold (<720px / >=720px), and native Rust C-ABI engine bridge (`attune-ffi`). | **COMPLETED** |
| **Phase 5** | **Full Integration & End-to-End Real-Time Playback** | Connect Flutter UI playhead and stage triggers to live `DualDeckMixer` via FFI, centralized cross-platform audio storage service (`AudioStorageService`), zero-dummy library scanning, genre filtering, genuine album art rendering, and live DSP telemetry. | **COMPLETED** |

---

## 3. Architectural Decisions Status

| ID | Decision Topic | Locked Architecture Baseline | Status |
| :--- | :--- | :--- | :--- |
| **ADR-001** | Development Order | Algorithm & Headless CLI first; Flutter UI skeleton second. | **LOCKED** |
| **ADR-002** | Native Engine Language | Hybrid **Rust** orchestrator (`attune-core` via `flutter_rust_bridge` / C FFI) wrapping **C/C++ DSP primitives** (`Signalsmith Stretch` / `RubberBand` + `miniaudio`). | **LOCKED** |
| **ADR-003** | Compute Offloading & Cache | 3-Tier Compute (On-Demand 45s Head/Tail, Local Background, External Worker Node) producing portable `.tap` and `.ptr` caches. | **LOCKED** |
| **ADR-004** | 5-Layer Audio Profiling | DSP + Ultra-Light Quantized Audio Tagging Model (~2–4 MB ONNX on device + deeper model on External Node) for Pitch/Formant Sustain, Chroma CQT, Beat/Phrase, 4-Band Energy, and Instrument/Vibe/Aggression (Ado/Punk/Rock/Pop). | **LOCKED** |
| **ADR-005** | External Node Stem Bridges | Export `.tap`/`.ptr` metadata caches by default, plus optionally pre-render short 5–8s isolated vocal/instrument stem bridges (via Demucs) for hero transitions. | **LOCKED** |
| **ADR-006** | Audio Ingestion & Spectral Extraction | Pure Rust decoding pipeline via `symphonia`, WAV rendering via `hound`, and RealFFT for 20Hz frame extraction. | **LOCKED** |
| **ADR-007** | Silence Gating & Boundary Clipping | Dynamic RMS -45 dBFS silence gating and strict audible boundary clipping in `AudioProfiler` and `TransitionPlanner` to eliminate dead air / pre-gap gaps in transitions. | **LOCKED** |
| **ADR-008** | Real-Time Dual-Deck Engine & cpal Output Driver | Real-time sample-accurate `DualDeckMixer` with 3-band isolator biquad EQ, Hermite interpolation resampler, reverb freeze wash, soft limiter, and `cpal` device playback. | **LOCKED** |
| **ADR-009** | Hardware Sample Rate Adaptation & 1-Bar Phrased Handoff | Fractional cubic Hermite playhead resampling ($\Delta \text{playhead} = \frac{F_{\text{track}}}{F_{\text{device}}} \times \text{stretch} \times \text{pitch}$) ensuring exact 1.000x speed and 100% native pitch across mismatched hardware DAC rates, plus 1-bar (4-beat) musical riff handoffs. | **LOCKED** |
| **ADR-010** | Vocal Integrity & Reverb Spillover | Multi-layer vocal activity inspection in `CueZone`, anchor cuts at/after final vocal resolution, flat mid/high EQ during active singing, and post-transition reverb freeze decay. | **LOCKED** |
| **ADR-011** | Portable Cache, CachePack & Node Sync Protocol | Bundled SQLite cache repository (`CacheDatabase`), portable `.pack.json` bundle (`CachePack`), background scanner (`LibraryScanner`), REST HTTP sync server (`tiny_http`), and sync client (`ureq`). | **LOCKED** |
| **ADR-012** | Liquid Material Glass UI, Subtle Transition Pills, Honest Audio Quality Badges & Shared Tablet/Desktop Shell | `Sora` + `Plus Jakarta Sans` typography, Material You HCT dynamic cover color + walled garden music algorithm-grade `BackdropFilter` Liquid Glass (`ledger/05_UI_DESIGN_SPEC.md`), subtle inline transition connector (no bulky AI card), human-friendly audio quality pills (`Very Good Quality Audio`, `Good Quality Audio`, `Proprietary Audio Codec`—zero Dolby Atmos hype), and shared Tablet/Desktop multi-pane layout (`>= 720px`). | **LOCKED** |
| **ADR-013** | Multi-Activity Android Predictive Back & Rust C-ABI FFI Bridge | Native independent Activities for Android (`MainActivity`, `SettingsActivity`, `TransitionInspectorActivity`) with `android:enableOnBackInvokedCallback="true"` for OS-level window peeking; dedicated `crates/attune-ffi` exporting `libattune_engine.so` cdylib to Dart FFI (`AttuneEngineBridge`). | **LOCKED** |
| **ADR-014** | Centralized Cross-Platform Storage Service & Real-Time Playback Controls | Dedicated `AudioStorageService` resolving Android 11+ scoped storage, Linux XDG/Music, and Windows paths; standard musical genre filtering (`All Music`, `Rock`, `Pop`, `Electronic`, `Jazz`, `Acoustic`); dynamic ingestion of 159 real tracks; thread-safe C-ABI player session in `attune-ffi`. | **LOCKED** |
| **ADR-015** | Non-Blocking Profiling, Library Cache, Stage Restructure & Neon Mini-Player | Decoupled CQT profiling from UI isolate, cached 159 tracks to JSON (< 5ms load), Startup Setup Wizard (`StartupSetupStage`), dedicated `HomeStage`, `AlbumsStage`, `ProfileAndStatsStage`, embedded lyrics toggle in `NowPlayingStage`, and floating neon mini-player. | **LOCKED** |
| **ADR-016** | Multi-Core Worker Offloading, ALSA Underrun Elimination, Deck Normalization & Mid-Animation Liquid Glass Morph | Parallelized cue zone profiling (`rayon::join`), background OS decoding (`std::thread::spawn`), worker isolate scanning (`Isolate.run`), fixed 2048-frame CPAL buffer margin, Deck A normalized handshake swap upon transition finish, and sinusoidal liquid frosted glass morph overlay (`AnimationController` + `BackdropFilter` $\sigma=48\text{px}$). | **LOCKED** |
| **ADR-017** | Synchronous Deck Invalidation, Cue-Zone Auto-Transition Timing & Queue Index Sync | Synchronously clear Deck A in memory before background audio decoding to eliminate 1-track UI/audio desync; align smart shuffle transition triggers strictly to genuine cue-zone timing (`track_a_transition_start_ms`) to preserve full vocal phrase completion; synchronize `_upNextQueueIndex` and `_currentQueueIndex` across deck swaps. | **LOCKED** |
| **ADR-018** | Decoupled Session Mutex Architecture, Non-Blocking Shuffle Execution & Release Binary Priority | Decouple `PLAYBACK_SESSION` from `DualDeckMixer` via `get_session_mixer()` to eliminate nested mutex contention across player FFI calls; execute Deck B preloading and pairwise transition planning in non-blocking microtasks; prioritize `release` binary paths; guard zero-BPM calculation. | **LOCKED** |
| **ADR-019** | Generative Melodic Bridge Fill Transition Archetype (`ARCHETYPE_6_GENERATIVE_MELODIC_FILL`), PolyBLEP SuperSaw Synthesis, Octave Tempo Folding & TPT SVF Stabilization | Add Archetype 6 generative fill to transition planner; octave-fold tempos into upbeat range [125, 220] BPM; procedural PolyBLEP SuperSaw FM synth with 16th-note cadential sequence, Vadim Zavalishin TPT state-variable filter riser (unconditionally stable up to Nyquist), noise sweep, snare roll, and dotted-16th ping-pong reverb spillover. | **LOCKED** |
| **ADR-020** | Scrapping Procedural Chiptune Fill & Implementing Walled Garden Music Algorithm 4-Stage DSP Lookahead Pipeline | Scrap procedural 8-bit note synthesis; implement walled garden music algorithm 4-stage pipeline with dynamic cue detection (< -18 dB outro drop, > -24 dB intro rise), constant-power cosine/sine crossfades ($G_A^2 + G_B^2 = 1.0$), 180 Hz frequency-split DJ bass swap, and conditional beat-grid alignment ($\le 6\%$ BPM gate) with Camelot dissonance damping. | **LOCKED** |

---

## 4. Session Handoff Checklist (For Next AI Session — Production Release & Platform Packaging)

1. **Review Current Codebase**:
   - Flutter app in `apps/attune_app/` with complete "Liquid Material Glass" design system, responsive scaffold, multi-activity Android declarations, and centralized `AudioStorageService`.
   - Rust crates in `crates/` (`attune-core`, `attune-cli`, `attune-dsp-ffi`, `attune-ffi`).
   - Dart FFI engine bridge and `PlaybackController` managing dual decks, queue, and real-time transition triggers.
2. **Next Steps (Phase 6 / Packaging & Distribution)**:
   - Linux packaging: AppImage / Flatpak bundle with compiled `libattune_engine.so`.
   - Android release: Gradle build verification for APK/AAB with NDK toolchain bundling native libraries for `arm64-v8a` and `x86_64`.
   - Automated integration tests across full playlists and simulated background playback.
3. Keep updating [`00_MASTER_LEDGER.md`](file:///home/miku/Documents/attune/ledger/00_MASTER_LEDGER.md), [`01_ADR_LOG.md`](file:///home/miku/Documents/attune/ledger/01_ADR_LOG.md), and [`04_TRANSITION_LAB_LOG.md`](file:///home/miku/Documents/attune/ledger/04_TRANSITION_LAB_LOG.md).





