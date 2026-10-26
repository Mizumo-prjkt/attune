# 03 — Cache Schema & Offloading Protocol Specification

This document defines the portable binary/JSON schemas for **Track Analysis Profiles (`.tap`)**, **Pairwise Transition Recipes (`.ptr`)**, and the **3-Tier Compute & External Node Sync Protocol**.

---

## 1. Content-Addressable Track Identity (`TrackHash`)

Because audio files may live at `/storage/emulated/0/Music/song.flac` on an Android phone and `/home/user/Music/song.mp3` on an External Compute Node (PC/Server), caches **must never be keyed solely by file path**.

Every track is resolved to a canonical `TrackId` via a 2-tier lookup:
1. **Fast Metadata + Duration Key**: `normalize(artist) + ":" + normalize(title) + ":" + round_sec(duration_ms)` (plus `ISRC` or `MusicBrainzID` if present).
2. **Acoustic Fingerprint Key (`audio_hash`)**: A compact Chromaprint / landmark hash computed from the first $15\text{ s}$ and last $15\text{ s}$ of decoded mono $11025\text{ Hz}$ audio. This survives transcoding (e.g., FLAC on PC $\leftrightarrow$ AAC/Opus on mobile).

---

## 2. Track Analysis Profile Schema (`.tap`)

A `.tap` record stores the extracted 5-Layer Audio Profile for a single song. When serialized with MessagePack / `rkyv` (with `zstd` compression), a complete `.tap` file is only **15 KB – 30 KB**.

```json
{
  "schema_version": 1,
  "track_id": {
    "audio_hash": "blake3_or_chromaprint_hex",
    "duration_ms": 271400,
    "canonical_slug": "whitney-houston:i-will-always-love-you:271"
  },
  "analysis_tier": "TIER_1_FAST_HEAD_TAIL | TIER_2_FULL_LOCAL | TIER_3_EXTERNAL_DEEP_ML",
  "global_summary": {
    "nominal_bpm": 67.5,
    "bpm_stability": 0.62,
    "primary_key": "A Major",
    "mean_aggression_index": 0.24,
    "macro_vibe_family": "SLOW_BALLAD_SOUL",
    "dominant_instruments": ["vocal_clean_melodic", "inst_clean_acoustic_guitar_piano", "inst_brass_horns"]
  },
  "cue_zones": [
    {
      "zone_type": "ENTRY | EXIT | BREAKDOWN",
      "start_ms": 225000,
      "end_ms": 271400,
      "hop_ms": 50,
      "beats_ms": [225120, 226010, 226900],
      "downbeats_ms": [225120, 228680, 232240],
      "phrase_boundaries_ms": [225120, 239360, 253600],
      "sustained_pitch_regions": [
        {
          "start_ms": 241200,
          "end_ms": 245800,
          "mean_midi_cents": 6900,
          "pitch_std_cents": 22.5,
          "voicing_confidence": 0.94,
          "formant_f1_hz": 480,
          "formant_f2_hz": 1150,
          "percussive_leakage_db": -34.0
        }
      ],
      "frames_20hz": {
        "f0_midi_cents_quantized": "base64_u16_array",
        "chroma_12_quantized": "base64_u8_matrix",
        "energy_4band_db_quantized": "base64_i8_matrix",
        "aggression_curve_u8": "base64_u8_array",
        "instrument_activations_u8": "base64_u8_matrix"
      }
    }
  ]
}
```

---

## 3. Pairwise Transition Recipe Schema (`.ptr`)

A `.ptr` record is the **compiled execution plan** between Track $A$ and Track $B$. Even the weakest mobile device can execute a `.ptr` at $<2\%$ CPU load because all search and optimization math is already finished.

```json
{
  "schema_version": 1,
  "track_a_hash": "hash_a",
  "track_b_hash": "hash_b",
  "archetype": "ARCHETYPE_1_SUSTAIN_BRIDGE | ARCHETYPE_2_RIFF_HANDOFF | ARCHETYPE_3_EQ_SWAP | ARCHETYPE_4_TAIL_FREEZE | ARCHETYPE_5_COLD_CUT",
  "confidence_score": 0.93,
  "timing": {
    "track_a_transition_start_ms": 241500,
    "track_a_exit_end_ms": 245200,
    "track_b_entry_start_ms": 1200,
    "track_b_native_lock_ms": 5800
  },
  "optional_stem_bridge_uri": null,
  "dsp_automation_keyframes": [
    {
      "offset_ms": 0,
      "deck_a": { "gain_db": 0.0, "eq_sub_db": 0.0, "eq_mid_db": 0.0, "eq_high_db": 0.0, "stretch_ratio": 1.0, "pitch_cents": 0, "reverb_freeze_send": 0.0 },
      "deck_b": { "gain_db": -60.0, "eq_sub_db": -24.0, "eq_mid_db": -12.0, "eq_high_db": -6.0, "stretch_ratio": 0.96, "pitch_cents": -100, "reverb_freeze_send": 0.0 }
    },
    {
      "offset_ms": 1800,
      "deck_a": { "gain_db": -4.5, "eq_sub_db": -24.0, "eq_mid_db": -3.0, "eq_high_db": -6.0, "stretch_ratio": 1.0, "pitch_cents": 0, "reverb_freeze_send": 0.2 },
      "deck_b": { "gain_db": -1.5, "eq_sub_db": -18.0, "eq_mid_db": 0.0, "eq_high_db": 0.0, "stretch_ratio": 0.98, "pitch_cents": -100, "reverb_freeze_send": 0.0 }
    },
    {
      "offset_ms": 3700,
      "deck_a": { "gain_db": -60.0, "eq_sub_db": -60.0, "eq_mid_db": -60.0, "eq_high_db": -60.0, "stretch_ratio": 1.0, "pitch_cents": 0, "reverb_freeze_send": 0.0 },
      "deck_b": { "gain_db": 0.0, "eq_sub_db": 0.0, "eq_mid_db": 0.0, "eq_high_db": 0.0, "stretch_ratio": 1.0, "pitch_cents": 0, "reverb_freeze_send": 0.0 }
    }
  ]
}
```

---

## 4. Three-Tier Compute & Offloading Workflow

### Tier 1: On-Demand JIT Fast-Pass (Playback Device)
- **Trigger**: Track $B$ is queued to follow Track $A$, and either `.tap(A)`, `.tap(B)`, or `.ptr(A, B)` is missing in the local SQLite ledger.
- **Pipeline**:
  1. Seek decoder for Track $A$ directly to $T_{\text{end}} - 45\text{ s}$; decode only the final $45\text{ s}$.
  2. Seek decoder for Track $B$ to $0\text{ s}$; decode only the first $45\text{ s}$.
  3. Run DSP Layer 1–5 extractors (using fast pYIN + spectral/harmonic heuristics) $\rightarrow$ save `TIER_1_FAST_HEAD_TAIL` `.tap` records.
  4. Run the Transition Planner $\rightarrow$ emit `.ptr(A, B)` into the real-time audio engine before Track $A$ reaches $T_{\text{end}} - 45\text{ s}$.

### Tier 2: Local Background Idle Worker (Playback Device)
- **Trigger**: Device is idle / charging and user library has tracks at `UNANALYZED` or `TIER_1`.
- **Pipeline**:
  - Scans full tracks at low thread priority, discovers mid-track breakdown/chorus cue zones, and pre-computes an **All-Pairs Compatibility Graph** (top-K best outgoing transitions per track in the library so "Smart Shuffle / Auto-DJ" can pick tracks that sound incredible together).

### Tier 3: External Compute Node ("Heavy Learning Worker")
- **Trigger**: User runs `attune-node` on a PC, Mac, Home Server, or Cloud container pointed at their music library (or accepting audio jobs over LAN).
- **Pipeline**:
  1. Executes `TIER_3_EXTERNAL_DEEP_ML` extraction:
     - Optional neural 4-stem separation (`htdemucs` / `onnx`) to cleanly isolate vocals, drums, bass, and guitar/other during Cue Zones.
     - High-precision neural pitch tracking (`RMVPE`) and deep instrument/vibe embeddings.
  2. Exports a compressed sync bundle (`attune-cache.pack` or live REST/WebSocket sync via mDNS `_attune._tcp.local.`).
  3. The mobile playback device imports these `TIER_3` `.tap` and `.ptr` records into its local SQLite ledger, getting flagship studio-grade transitions with zero mobile battery drain.
