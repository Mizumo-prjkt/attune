# 00 — Attune Master Knowledge Ledger

- **Project Name**: Attune
- **Current Phase**: `Phase 1 COMPLETED -> Ready for Phase 2 (Real-Time Dual-Deck Native Engine)`
- **Last Updated**: 2026-10-27
- **Active Coding Status**: Phase 1 Headless DSP Sandbox & CLI operational. 5-Layer profiler, transition planner (Archetypes 1–5), and WAV renderer fully tested across all 4 empirical benchmarks.

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
| **Phase 2** | **Real-Time Dual-Deck Native Engine** | Lock-free dual-deck playback mixer executing `.ptr` time-stretch, formant pitch-shift, and 3-band EQ curves in real time + On-Demand 45s JIT Fast-Pass. | **READY TO START** |
| **Phase 3** | **Portable Cache & External Compute Node** | SQLite + blob cache indexing, acoustic fingerprinting, Demucs stem-bridge export, and LAN/file sync from external compute worker to client. | PENDING |
| **Phase 4** | **Flutter Frontend & Transition Inspector** | Connect Flutter UI via FFI (`flutter_rust_bridge`), starting with a visual waveform/curve Transition Inspector and expanding into the full music player UI. | PENDING |

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

---

## 4. Session Handoff Checklist (For Next AI Session)

1. Review [`04_TRANSITION_LAB_LOG.md`](file:///home/miku/Documents/attune/ledger/04_TRANSITION_LAB_LOG.md) to inspect empirical results from Exp #001 through #008 (including real FLAC transitions with silence elimination).
2. Start **Phase 2**: Implement the Real-Time Dual-Deck native playback engine in `crates/attune-core`:
   - Dual-deck lock-free ring buffers (Deck A & Deck B).
   - Sample-accurate real-time keyframe automation evaluator.
   - Real-time audio stream callback integration via `miniaudio` / `cpal`.
   - On-Demand JIT Mode 1 background thread fast-pass for queued songs.
3. Keep updating [`00_MASTER_LEDGER.md`](file:///home/miku/Documents/attune/ledger/00_MASTER_LEDGER.md) and record any new decisions in [`01_ADR_LOG.md`](file:///home/miku/Documents/attune/ledger/01_ADR_LOG.md).


