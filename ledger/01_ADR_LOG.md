# 01 — Architecture Decision Records (ADR Log)

This file records all major architectural decisions for **Attune**, including context, considered alternatives, and rationale. Future AI sessions must respect `LOCKED` decisions unless explicitly instructed by the user to revise them.

---

## ADR-001: Algorithm-First, Headless-First Development Order
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**: Building a music player UI in Flutter before proving the transition/mixing algorithm leads to wasted UI rework and slow iteration cycles when testing DSP math.
- **Decision**: We will build and validate the **Audio Analysis & Transition Engine** as a standalone headless library and CLI (`attune-core` + `attune-cli`) first. The Flutter UI skeleton will only be built after the transition algorithm is verified on real benchmark songs.
- **Consequences**:
  - Rapid CLI-based iteration (render transition windows directly to `.wav` and inspect `.tap`/`.ptr` JSON/binary dumps).
  - The headless CLI naturally doubles as our **External Compute Node** daemon for processing audio on a desktop/server!

---

## ADR-002: Hybrid Rust Core + C/C++ DSP Primitives with Flutter UI
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**:
  - Flutter/Dart cannot execute real-time, lock-free sample-accurate dual-deck audio mixing, formant-preserving pitch shifting, or phase-locked time-stretching on its own.
  - We evaluated Pure C++20 vs. Pure Rust vs. a Hybrid Rust + C/C++ DSP architecture.
- **Decision**:
  - **Frontend**: Flutter (Cross-platform UI, state visualization, library browsing).
  - **Core Engine & Orchestration (`attune-core`)**: **Rust** bridged to Flutter via `flutter_rust_bridge` v2. Rust manages the dual-deck playback state machine, thread-safe ring buffers, background worker queues, SQLite/binary cache ledger, and External Compute Node LAN/HTTP synchronization.
  - **DSP & Audio I/O Primitives**: Statically linked C/C++ battle-tested libraries inside Rust where pure Rust equivalents are less mature—specifically `Signalsmith Stretch` (or `RubberBand`) for formant-preserving pitch/time shifting, `miniaudio` for glitch-free cross-platform audio callback I/O, and `ONNX Runtime` / `tract` for lightweight quantized instrument/vibe neural heads.
- **Consequences**:
  - Memory safety and fearless concurrency for background/on-demand analysis threads.
  - Zero-boilerplate FFI generation between Flutter and native code.
  - Best-in-class vocal pitch/formant preservation from C++ DSP libraries.

---

## ADR-003: Three-Tier Compute & Portable Content-Addressable Cache
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**: Full-track multi-layer audio profiling (especially neural instrument tagging, pitch tracking, and optional stem separation) can be CPU/battery-intensive on lower-end mobile devices. However, users still want instant transitions when picking songs on the fly.
- **Decision**:
  1. Separate **Analysis/Planning** (produces `.tap` Track Analysis Profiles and `.ptr` Pairwise Transition Recipes) from **Real-Time Playback** (consumes `.ptr` automation keyframes with minimal CPU).
  2. Support three execution modes sharing the exact same cache format:
     - **Mode 1 — On-Demand JIT Fast-Pass**: Decodes and analyzes only the **last 45s of Track A** and **first 45s of Track B** when an unanalyzed song is queued next.
     - **Mode 2 — Local Background Idle Worker**: Processes full tracks at low OS thread priority when idle/charging.
     - **Mode 3 — External Compute Node**: Runs on an external PC, NAS, or cloud server to process libraries (with deeper ML models) and syncs the `.tap`/`.ptr` reference cache to the playback device.
  3. Key all cache entries by a **Content-Addressable Acoustic Hash** (rather than local file paths) so caches generated on an external PC work immediately on a phone.

---

## ADR-004: Five-Layer Audio Profiling with Hybrid DSP + Ultra-Light Quantized Audio Tagging
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**:
  - Two songs can share the same BPM and key yet fail miserably in a transition if their vocal phrasing, instrument texture, or rhythmic aggression clash.
  - Conversely, two songs with very different global metadata can transition iconically if their local audio profiles align—for example:
    - **Vocal Sustain Match**: *Whitney Houston ("I Will Always Love You")* $\rightarrow$ *Mariah Carey ("All I Want For Christmas Is You")* via a held *"youuuuu"* pitch/formant bridge.
    - **Instrument & Vibe Match**: An *Ado* track transitioning into a punk or J-Rock track that shares overdriven guitar presence, vocal grit (*ganari*), or aggressive snare transients.
    - **Genre Energy Differentiation**: Knowing whether a track segment is **Aggressive Rock/Punk**, **Slow Rock**, or **Pop** so we don't apply a slow 12-second dreamy crossfade to high-octane punk tracks (which creates muddy distortion clashes) or slam a hard punk cut onto a soft ballad without a proper breakdown/reverb bridge.
- **Decision**:
  - Extract **Five Layers** per Cue Zone:
    1. Micro-Pitch & Sustained Formant Contour ($f_0, F_1, F_2$)
    2. Local Harmonic Chroma (12-bin CQT)
    3. Dynamic Rhythm, Beat & Phrase Grid
    4. 4-Band Spectral Energy Envelopes
    5. **Instrument Signature, Vibe & Genre-Aggression Layer**: Uses a **Hybrid DSP + Ultra-Light Quantized Audio Tagging Model** (~2–4 MB quantized ONNX/TFLite model like MobileNetV3-Audio/EffNet on mobile devices, paired with a deeper model on the External Compute Node) plus mathematical DSP descriptors (attack slope, spectral roughness, sub-beat onset density) to output Active Instrument Tags, Vocal Grit Detection, Aggression Index $\mathcal{A} \in [0, 1]$, and Groove Family Vectors.

---

## ADR-005: External Node Metadata Cache + Optional Pre-Rendered Stem Bridges
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**: Sometimes a transition between two complex tracks (such as isolating Whitney Houston's held vocal while removing her backing band, or isolating an incoming punk drum fill / guitar riff on an Ado transition) is dramatically cleaner with 4-stem source separation (Demucs), which is too heavy to run in real time on low-end mobile phones.
- **Decision**:
  - By default, the External Compute Node exports lightweight `.tap` and `.ptr` metadata caches (~15–30 KB per track).
  - Additionally, for high-value "hero" transitions (`ARCHETYPE_1_SUSTAIN_BRIDGE` and `ARCHETYPE_2_RIFF_HANDOFF`) where background instrumentation would otherwise clash, the External Compute Node can **optionally pre-render a short 5–8 second isolated vocal/instrument stem bridge snippet** (via Demucs/HTDemucs) and attach its URI (`optional_stem_bridge_uri`) to the `.ptr` recipe for effortless playback on weaker devices.

---

## ADR-006: Pure Rust Audio Ingestion, RealFFT Spectral Frame Pipeline & Headless CLI Harness
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**: In Phase 1, we require a cross-platform, zero-system-dependency headless audio decoder, 5-layer feature extractor, and WAV transition renderer capable of running on any desktop OS, CI runner, or headless container without requiring external FFmpeg shared libraries or complex toolchains.
- **Decision**:
  - Use `symphonia` (v0.5.x) as the pure Rust media format probing and decoding engine, supporting WAV, MP3, FLAC, AAC, OGG, and ALAC.
  - Use `realfft` (v3.5) for real-to-complex forward Fourier transforms on 2048-point Hann-windowed frames at 20 Hz (50 ms hop size).
  - Use `hound` (v3.5) for deterministic, lossless 16-bit PCM WAV rendering of transition recipes.
  - Implement the `attune-cli` binary with `clap` (v4.5) providing `analyze`, `plan`, `render`, `transition`, and `benchmarks` subcommands.
- **Consequences**:
  - The entire DSP analysis and transition planning engine builds and tests headlessly with `cargo test` in seconds.
  - All 4 benchmark transitions are fully automated and reproducible without external runtime audio daemon dependencies.

---

## ADR-007: Dynamic Silence Boundary Detection & Pre-Gap/Post-Gap Clipping
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**: Studio lossless audio tracks (FLAC/WAV) frequently contain digital silence, dither noise, or room tone at the start (pre-gap: 200ms–1.5s) and end (post-gap/fade: 1s–4s). Naive extraction from $t=0$ and $t=\text{duration}-45\text{s}$ causes the outgoing track's cut/freeze anchor to land inside trailing dead air, while the incoming track starts during leading silence, producing an audible silence hole (drop to $-120\text{ dBFS}$) during transitions.
- **Decision**:
  - Add fast, allocation-free RMS silence boundary scanners (`find_audible_start_ms(-45.0)` and `find_audible_end_ms(-45.0)`) in `AudioBuffer` operating in 20 ms window chunks.
  - `AudioProfiler::profile_track` strictly bounds `ZoneType::ENTRY` to start at `audible_start_ms` and `ZoneType::EXIT` to terminate at `audible_end_ms`.
  - `TransitionPlanner` anchors transition cuts and freeze triggers to the last musical downbeat/beat within `[audible_start, audible_end]`, and starts Track B at `entry_b.start_ms` (first audible transient).
  - In `TransitionRenderer`, pre-populate the freeze buffer (`freeze_buf`) with the chord hit at transition start so the reverb freeze wash has harmonic energy from sample 0.
- **Consequences**:
  - Eliminates all dead air / silence gaps between tracks.
  - Transitions are punchy, continuous, and immediately musical across all archetypes.

---

## ADR-008: Real-Time Dual-Deck Engine, Biquad Isolator EQ & cpal Output Driver
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**: In Phase 2, the player engine requires real-time, sample-accurate, lock-free dual-deck playback mixing with zero allocations in the audio thread callback, executing dynamic 3-band isolator EQ curves, reverb freeze washes, speed/pitch resampling, and automated transition keyframe execution through physical system audio hardware.
- **Decision**:
  - Implement `ThreeBandEq` in `attune-dsp-ffi` using Direct Form II Transposed biquad filters (Low Shelf at 200 Hz, Peaking at 1000 Hz with $Q=0.707$, High Shelf at 4000 Hz).
  - Implement `ReverbFreeze` with recirculating diffusion network and continuous damping decay.
  - Implement `Deck` in `attune-core::engine` managing per-deck playback state, Hermite cubic interpolation fractional playhead tracking, gain, EQ, and freeze wash.
  - Implement `DualDeckMixer` in `attune-core::engine` with sample-accurate transition state machine that automates Deck A and Deck B across the recipe duration and cleanly transfers active deck control to Deck B on completion.
  - Integrate `cpal` (v0.18) via `AudioOutputDriver` running the mixer inside the real-time audio thread callback, with graceful fallback to software stepping in headless environments.
  - Implement `JitPlanner` for On-Demand Mode 1 fast-pass planning (<150 ms) for newly queued tracks.
---

## ADR-009: Hardware Sample Rate Resampling Adaptation & 1-Bar Phrased Transition Automation
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**: Lossless audio libraries predominantly consist of standard Red Book CD tracks encoded at $44,100\text{ Hz}$, whereas modern Linux (ALSA / PipeWire / PulseAudio), macOS (CoreAudio), and Windows (WASAPI) hardware audio drivers default to $48,000\text{ Hz}$ or $96,000\text{ Hz}$. Naive 1.0-frame stepping per output callback sample caused tracks to play $8.84\%$ fast ($\frac{48,000}{44,100} = 1.0884$) and shifted pitch upward by $+146.7\text{ cents}$ ($\approx +1.5\text{ semitones sharp}$). In addition, a 2-beat ($700\text{ ms}$) riff handoff cut was too brief for incoming intro phrases to establish musical groove, sounding abrupt and rushed.
- **Decision**:
  - Store `output_sample_rate` ($F_{\text{device}}$) in `Deck` and query hardware native rate via `AudioOutputDriver::default_sample_rate()`.
  - Step the playhead continuously per output frame by the fractional rate ratio:
    $$\Delta \text{playhead} = \frac{F_{\text{track}}}{F_{\text{device}}} \times \text{stretch\_ratio} \times \text{pitch\_ratio}$$
  - Interpolate fractional playhead positions via 4-point cubic Hermite resampling (`interpolate_hermite`), yielding zero pitch drift and exact $1.0000\times$ speed across any device sample rate.
  - Compute `seek_ms` and `current_position_ms` strictly against the loaded audio buffer's native sample rate (`audio.sample_rate`).
  - Synchronize `DualDeckMixer::set_output_sample_rate` dynamically when the audio driver initializes.
  - Expand `punch_duration_ms` in `build_archetype_2_riff_handoff_recipe` to 4 beats ($1\text{ full 4/4 bar} = 1400\text{ ms}$ at 171.6 BPM), using 5 intentional keyframe automation points across the measure (Beat 1 entry at $-18\text{ dB}$, Beat 2 notch carving at $-6\text{ dB}$, Beat 3 melodic handoff, Beat 4 pre-drop duck at $-8\text{ dB}$, and Bar 2 Downbeat slam cut with a smooth $0.20$ reverb splash).
  - Smooth `build_archetype_4_tail_freeze_recipe` with an equal-power entry curve and 4-point decay envelope.
- **Consequences**:
  - Tracks play back at 100% native pitch ($A_4 = 440.0\text{ Hz}$) and sample-accurate speed regardless of hardware DAC configuration.
  - Transition handoffs have full measure musical phrasing, breathing room, and professional DJ groove.




---

## ADR-010: Vocal Integrity Protection & Post-Transition Reverb Spillover
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**: During transition planning on vocal-heavy tracks (such as *Rabbit Hole* $\to$ *(Not) A Devil*), earlier planner logic selected `a_cut_point` strictly from grid downbeats (`downbeats_ms.rfind(|&db| db <= z.end_ms)`), which landed at $157,755\text{ ms}$ while the vocalist was still actively singing the final syllable ("-ru" ending at $157,960\text{ ms}$). Additionally, mid-band EQ was ducked by $-12\text{ dB}$ on beat 4, muffling the voice right before the cut. Furthermore, upon transition completion, Deck A was stopped and muted immediately, truncating lingering reverb freeze tails and creating an abrupt drop.
- **Decision**:
  1. Add vocal boundary inspection methods to `CueZone`:
     - `is_vocal_active_at_ms(ms)`: checks whether `instrument_activations` indicates vocal presence ($>0.50$) with valid $f_0$.
     - `last_vocal_end_ms()`: computes the exact timestamp where the final vocal phrase definitively resolves.
     - `has_vocal_between_ms(start_ms, end_ms)`: verifies vocal activity across the handoff window.
  2. Implement vocal protection in `TransitionPlanner`:
     - In `RiffHandoff` and `TailFreeze`, anchor `a_cut_point = max(a_cut_point, last_vocal_end_ms)`. The outgoing singer is never cut short mid-word or mid-syllable.
     - When active vocals exist in the handoff window, keep Deck A's mid and high EQ flat at $0.0\text{ dB}$ so vocal clarity and presence are uncompromised; carve headroom exclusively from sub-bass ($-6\text{ dB} \to -18\text{ dB}$) to prevent kick/bass clashing with Track B.
     - On resolution, send $0.45$ reverb freeze so the final vocal consonant washes over the handoff into Track B.
  3. Implement decaying reverb spillover in `Deck` and `DualDeckMixer`:
     - Add `finish_with_tail()` to `Deck`, setting `DeckState::Finished` instead of stopping DSP computation.
     - In `DualDeckMixer::process_stereo_frame`, when playback transfers to `ActiveDeck::DeckB`, Deck A's audio buffer stops advancing but its internal reverb diffusion network continues processing and mixing into the stereo stream until naturally decayed, eliminating dry cuts.
- **Consequences**:
  - Outgoing vocals conclude completely and naturally with full intelligibility.
  - Reverb washes spill seamlessly over incoming downbeats for studio-grade transition continuity.

---

## ADR-011: Portable SQLite Cache Database, Portable CachePack Bundling & External Node Sync Protocol
- **Date**: 2026-10-27
- **Status**: `LOCKED`
- **Context**: In Phase 3, Attune requires a persistent, content-addressable cache repository and external compute offloading protocol (Mandate 3). Lightweight playback clients (mobile phones, embedded players) cannot afford heavy, battery-draining 5-layer DSP analysis on every track queue change. Users require the ability to run an External Compute Node on a desktop or home server that scans audio libraries, precomputes All-Pairs transition matrices, and synchronizes `.tap` profiles, `.ptr` recipes, and stem bridge references to playback clients via portable files or local network HTTP sync.
- **Decision**:
  1. **SQLite Content-Addressable Cache (`CacheDatabase`)**:
     - Integrate pure C bundled SQLite via `rusqlite` (with `bundled` feature), guaranteeing zero system package dependencies across Linux, macOS, Windows, Android, and iOS.
     - Persist `tracks` table keyed by content-addressable `audio_hash`, `transitions` table keyed by composite `(track_a_hash, track_b_hash)`, and `stem_bridges` table keyed by `(track_a_hash, track_b_hash, stem_type)`.
     - Enable Write-Ahead Logging (`PRAGMA journal_mode = WAL;`) for high-throughput concurrent reads.
  2. **Portable CachePack Archive Format (`CachePack`)**:
     - Implement a portable, self-contained JSON exchange bundle containing schema version, generation timestamp, `.tap` profiles, `.ptr` recipes, and stem bridge records.
     - Provide atomic database export (`from_db`) and import (`import_into_db`) for offline cross-machine synchronization.
  3. **Headless Library Scanner & Smart Shuffle Precomputation (`LibraryScanner`)**:
     - Implement recursive directory scanning discovering all standard audio formats (`.flac`, `.mp3`, `.wav`, `.ogg`, `.m4a`, `.aac`, `.aiff`).
     - Check cache database to skip already-analyzed tracks, profile unprofiled tracks, and precompute pairwise transition matrices across the library.
  4. **Lightweight Local Network Sync Protocol (`NodeServer` & `NodeClient`)**:
     - Implement a zero-async HTTP node server using `tiny_http` exposing REST API endpoints:
       - `GET /api/v1/info`: Node status, catalog counts, and version.
       - `GET /api/v1/tracks`: Track ID catalog.
       - `GET /api/v1/tap/:hash` & `POST /api/v1/tap`: Individual profile retrieval and ingestion.
       - `GET /api/v1/ptr/:hash_a/:hash_b` & `POST /api/v1/ptr`: Transition recipe retrieval and ingestion.
       - `GET /api/v1/export`: Streaming CachePack archive download.
       - `POST /api/v1/sync/push`: Batch push of client profiles and recipes.
       - `POST /api/v1/sync/pull`: Batch pull of missing profiles and relevant transition recipes based on client track list.
     - Implement a synchronous HTTP sync client using `ureq` allowing clients to sync entire libraries in seconds over LAN.
  5. **Cache-Integrated JIT Engine (`JitPlanner::plan_cached_or_jit`)**:
     - Transparently check SQLite cache before performing spectral analysis. Cached transition lookups complete in microsecond/sub-millisecond time.
     - Automatically persist newly planned recipes to the cache database for subsequent instant playback.
- **Consequences**:
  - Transition recipe lookup on cached tracks drops from ~1.5s to <100ms.
  - Heavy analysis can be run once on a desktop/server and transferred to mobile devices with zero mobile compute overhead.
  - Full Phase 3 milestones completed and verified.
