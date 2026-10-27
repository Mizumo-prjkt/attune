# Attune — AI Agent Guidebook & Knowledge Ledger Protocol

Welcome, AI Agent. This repository (**Attune**) is an open-source, intelligent music player with a DJ-grade, audio-profile-driven transition and remixing engine.

To prevent context drift across AI sessions, this project uses a strict **Knowledge Ledgering System** stored in [`ledger/`](file:///home/miku/Documents/attune/ledger/).

---

## 1. Mandatory Session Startup Protocol (Read-First Rule)

Before proposing architectural changes, writing DSP code, or scaffolding UI components, **every AI session MUST read**:

1. [`ledger/00_MASTER_LEDGER.md`](file:///home/miku/Documents/attune/ledger/00_MASTER_LEDGER.md) — Current project phase, locked vs. open architectural decisions, and active next steps.
2. [`ledger/01_ADR_LOG.md`](file:///home/miku/Documents/attune/ledger/01_ADR_LOG.md) — Architecture Decision Records explaining *why* core technical decisions were made.
3. The domain specification relevant to your current task:
   - **Audio Analysis, Vibe/Genre Profiling & Mixing Math**: [`ledger/02_DSP_ALGORITHM_SPEC.md`](file:///home/miku/Documents/attune/ledger/02_DSP_ALGORITHM_SPEC.md)
   - **Cache Formats (`.tap`, `.ptr`) & Offloading/External Compute Protocol**: [`ledger/03_SCHEMA_AND_PROTOCOL.md`](file:///home/miku/Documents/attune/ledger/03_SCHEMA_AND_PROTOCOL.md)
   - **Transition Benchmark Cases & Empirical Listening Logs**: [`ledger/04_TRANSITION_LAB_LOG.md`](file:///home/miku/Documents/attune/ledger/04_TRANSITION_LAB_LOG.md)
   - **Flutter UI, "Liquid Material Glass" Theme, Typography & Shared Responsive Shell**: [`ledger/05_UI_DESIGN_SPEC.md`](file:///home/miku/Documents/attune/ledger/05_UI_DESIGN_SPEC.md)

---

## 2. Core Engineering Mandates

### Mandate 1: Algorithm First, UI Skeleton Second
- Do **not** build complex Flutter UI screens until the headless native transition engine (`attune-core` + `attune-cli`) is proven on real audio transitions.
- Every change to the mixing/transition algorithm must be testable headlessly (inputting two audio files, generating their `.tap` profiles and `.ptr` transition recipe, and rendering the transition window to an output audio file or live stream).

### Mandate 2: Multi-Dimensional Audio Profiling (Never Rely on Basic Metadata Alone)
- Basic BPM and global Key tags are insufficient.
- Attune's transition engine must evaluate **Five Audio Profile Layers** inside candidate Cue Zones:
  1. **Micro-Pitch & Sustained Contour ($f_0$, Formants $F_1/F_2$)**: For vocal-sustain morphs (e.g., *Whitney Houston — "I Will Always Love You"* held *"youuuuu"* $\rightarrow$ *Mariah Carey — "All I Want For Christmas Is You"* held *"youuuuu oh baby"*).
  2. **Local Harmonic Chroma (12-bin CQT)**: Capturing real-time chord progressions in intros/outros.
  3. **Dynamic Rhythm & Phrase Grid**: Beat/downbeat tracking, rubato tempo curves, and 4/8/16-bar phrase boundaries.
  4. **Band-Split Spectral Energy**: Sub/Bass (20–150 Hz), Low-Mid (150–600 Hz), Vocal/Presence (600–4k Hz), Air (4k–20k Hz).
  5. **Instrument Signature, Vibe & Genre-Aggression Dynamics**: Identifying dominant instruments (distorted guitar, brass, slap bass, synth, acoustic piano, vocal grit/growl like Ado's *ganari*) and computing an **Aggression & Drive Index ($\mathcal{A} \in [0, 1]$)** to distinguish **Aggressive Rock / Punk / J-Rock** from **Slow Rock** or **Pop**, adapting transition speed and cut style accordingly.

### Mandate 3: Three-Tier Compute & Portable Cache Offloading
- Heavy analysis is decoupled from lightweight real-time playback DSP:
  1. **On-Demand JIT Mode**: Analyzes only the last 45s of Track A and first 45s of Track B (<150 ms) when uncached tracks are queued.
  2. **Background Idle Mode**: Low-priority local worker scanning the full library when idle/charging.
  3. **External Compute Node (Desktop/Server/Cloud)**: Runs the exact same core engine (plus optional heavy ML like 4-stem separation and deep audio embeddings) on a capable machine and syncs compact `.tap` (Track Analysis Profile) and `.ptr` (Pairwise Transition Recipe) caches to weaker playback devices.

---

## 3. Session Handoff & Ledger Maintenance Rules

Before ending a work session or completing a milestone, the AI agent MUST:
1. **Update [`ledger/00_MASTER_LEDGER.md`](file:///home/miku/Documents/attune/ledger/00_MASTER_LEDGER.md)** with completed tasks, new blockers, and the exact next task for the following session.
2. **Record New Architectural Decisions** in [`ledger/01_ADR_LOG.md`](file:///home/miku/Documents/attune/ledger/01_ADR_LOG.md) whenever a library, crate, schema, or algorithmic approach is chosen or changed.
3. **Log All Algorithm Tuning Experiments** in [`ledger/04_TRANSITION_LAB_LOG.md`](file:///home/miku/Documents/attune/ledger/04_TRANSITION_LAB_LOG.md). Never overwrite previous test observations—append new dated entries so future AI sessions understand which parameter combinations failed and why.
4. **Provide a very detailed Git Commit after changes were sucessfully was made**. Be extremely descriptive on the git logs on what issue did you tackle, any hiccups during build or tests, and also apply attribution to the Git as the agents and the author alongside.
