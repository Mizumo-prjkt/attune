# 02 — DSP & Transition Algorithm Specification

This document defines the mathematical and algorithmic architecture of Attune's **Five-Layer Audio Profiler** and **Multi-Archetype Transition Planner**.

---

## 1. Cue-Zone Windowing Strategy

Rather than treating a song as a single static vector, Attune segments each track into structural **Cue Zones**:
- **Entry Zone ($Z_{\text{in}}$)**: Typically $[0\text{ s}, 45\text{ s}]$ (or up to the first chorus downbeat).
- **Exit Zone ($Z_{\text{out}}$)**: Typically $[T_{\text{end}} - 45\text{ s}, T_{\text{end}}]$ (from the final chorus/outro breakdown to the end).
- **Mid-Track Hook/Breakdown Zones ($Z_{\text{mid}}$)** *(Extracted during full background/external scans)*: Breakdown bridges or chorus transitions that allow "Quick-Mix / Power-Hour" DJ jumps.

All 5 feature layers are extracted at a unified frame grid of **$20\text{ Hz}$ ($50\text{ ms}$ hop size)** within active Cue Zones.

---

## 2. The Five-Layer Audio Profiler

### Layer 1: Micro-Pitch, Vocal Sustain & Formant Contour
Designed to power seamless melodic/vocal morphs (e.g., *Whitney Houston $\rightarrow$ Mariah Carey*):
- **Vocal-Band Isolation**: Band-pass filter ($150\text{ Hz} - 4000\text{ Hz}$) with Mid/Side center-channel emphasis ($M = \frac{L+R}{2}$, where lead vocals reside) or Tier-2 vocal stem separation on External Compute Nodes.
- **Pitch Tracking ($f_0(t)$ & Voicing Confidence $v(t) \in [0,1]$)**:
  - Extracted via **pYIN** (Tier-1 local fast-pass) or **RMVPE / CREPE-tiny** (Tier-2 background/external worker).
  - Converted to continuous MIDI pitch cents: $p(t) = 6900 + 1200 \log_2\left(\frac{f_0(t)}{440}\right)$.
- **Sustained Note / Fermata Detector**:
  - A contiguous window $[t_1, t_2]$ is flagged as a **Sustained Pitch Region** if:
    1. Duration $t_2 - t_1 \ge 800\text{ ms}$,
    2. Mean voicing confidence $\bar{v} \ge 0.75$,
    3. Pitch stability $\text{std}(p(t)) \le 45\text{ cents}$ (allowing natural vibrato of $\pm 35\text{ cents}$ at $5\text{–}7\text{ Hz}$),
    4. Percussive transient flux in the Sub and Air bands is below threshold $\tau_{\text{perc}}$.
- **Formant / Vowel Approximation ($F_1, F_2$)**:
  - Linear Predictive Coding (LPC, order $12$) or spectral envelope peak picking extracts the first two formant frequencies $F_1, F_2$ during sustained regions to match vowel openness (e.g., matching an open *"oo / oh / ah"* vowel sustain across two tracks).

### Layer 2: Local Harmonic & Chroma Progression
- **12-Bin Constant-Q Transform (Chroma CQT)** across 6 octaves ($C_1$ to $B_6$), L2-normalized per frame, smoothed with a $250\text{ ms}$ median filter.
- Captures the **exact local chord** at the exit cue and entry cue (not just the whole-song key), enabling harmonic modulation bridges (unison, relative major/minor, dominant fifth, or $\pm 1\text{–}2$ semitone pitch-shifted locks).

### Layer 3: Dynamic Rhythm, Beat & Phrase Grid
- **Onset Strength Envelope (OSE)**: Multi-band spectral flux emphasizing low-end kick ($30\text{–}120\text{ Hz}$) and high-mid snare/hat ($1.5\text{–}8\text{ kHz}$).
- **Dynamic Tempo & Beat Tracking**:
  - Dynamic programming beat tracker plus bar-level **Downbeat (1-beat) Classifier** (using harmonic change rate + bass onset periodicity).
  - Supports **non-constant tempo curves $\text{BPM}(t)$** to handle rubato ballad intros/outros before a quantized beat enters.

### Layer 4: 4-Band Spectral Energy Envelopes
RMS energy curves (in LUFS / dBFS) across four linkwitz-riley crossover bands:
1. $E_{\text{sub}}(t)$: $20\text{–}150\text{ Hz}$ (Sub-bass & Kick fundamental)
2. $E_{\text{lowmid}}(t)$: $150\text{–}600\text{ Hz}$ (Warmth, snare body, bass harmonics)
3. $E_{\text{vocal}}(t)$: $600\text{–}4000\text{ Hz}$ (Lead vocals, electric guitar bite, piano/synth lead)
4. $E_{\text{air}}(t)$: $4000\text{–}20000\text{ Hz}$ (Cymbals, breath, hi-hats, sibilance)

### Layer 5: Instrument Signature, Vibe & Genre-Aggression Profiler
Designed to understand **musical instrumentation, aesthetic vibe (e.g., Ado $\rightarrow$ Punk/J-Rock), and rock/pop energy levels**:

#### 5.1 Dominant Instrument & Vocal Articulation Vector ($\mathbf{I}(t) \in [0,1]^K$)
Combines a lightweight quantized audio tagging head (e.g., MobileNetV3-Audio / EffNet-Discogs / PANNs running on $1\text{ s}$ Mel-spectrogram patches) with targeted DSP detectors to output activation probabilities for key timbral families:
- `inst_distorted_guitar`: High odd-harmonic saturation + dense spectral flatness in $800\text{–}3500\text{ Hz}$.
- `inst_clean_acoustic_guitar_piano`: Sharp plucked/struck attack followed by clean exponential harmonic decay.
- `inst_brass_horns`: Strong harmonic series with characteristic brass formant peak ($1\text{–}2.5\text{ kHz}$).
- `inst_synth_electronic`: High sub-bass stability or saw/square wave harmonic combs.
- `vocal_clean_melodic`: High harmonic-to-noise ratio (HNR) in the mid-channel vocal band.
- `vocal_grit_fry_growl`: Captures raspy/scream/growl vocals (such as Ado's *ganari* technique or punk vocals) where sub-harmonics and high-frequency roughness coexist with a strong $f_0$ contour.

#### 5.2 The Aggression & Drive Index ($\mathcal{A}(t) \in [0, 1]$)
Differentiates **Aggressive Rock / Punk / J-Rock** ($\mathcal{A} \in [0.70, 1.0]$) from **Driving Pop / Mid-Tempo Rock** ($\mathcal{A} \in [0.35, 0.70]$) and **Slow Rock / Acoustic Ballad** ($\mathcal{A} \in [0.0, 0.35]$):

$$\mathcal{A}(t) = \sigma\Big( w_1 \cdot \text{AttackSlope}(t) + w_2 \cdot \text{Roughness}(t) + w_3 \cdot \text{OnsetDensity}_{8\text{th/16th}}(t) + w_4 \cdot \frac{E_{\text{vocal}}(t) + E_{\text{air}}(t)}{E_{\text{total}}(t)} \Big)$$

- **Why this matters**:
  - A **Slow Rock** song at $130\text{ BPM}$ (half-time feel, long snare decay, low onset density, clean guitar) has $\mathcal{A} \approx 0.28$.
  - An **Ado / Punk / J-Rock** song at $130\text{ BPM}$ (rapid 8th/16th-note drum fills, distorted guitar saturation, vocal grit, steep transient attacks) has $\mathcal{A} \approx 0.85$.
  - A **Pop / Dance** song at $130\text{ BPM}$ (four-on-the-floor kick, clean sub-bass, low mid-range distortion roughness, high beat regularity) has high groove regularity $\mathcal{G}$ with moderate roughness.

---

## 3. Multi-Archetype Transition Planner & Scoring Engine

Given outgoing Track $A$ and incoming Track $B$, the planner evaluates candidate pairs $(t_A \in Z_{\text{out}}(A), t_B \in Z_{\text{in}}(B))$ across **Five Transition Archetypes**:

### Archetype 1: Sustained-Pitch & Formant Morph ("The Whitney $\rightarrow$ Mariah Bridge")
- **Eligibility**:
  - $A$ has a Sustained Pitch Region $[t_{A1}, t_{A2}]$ with pitch $p_A$ and low percussive energy.
  - $B$ has a Sustained Pitch Region or melodic vocal/instrumental pickup $[t_{B1}, t_{B2}]$ with pitch $p_B$.
  - Pitch distance $|\Delta p| = |(p_A - p_B) \bmod 1200| \le 250\text{ cents}$ (2.5 semitones).
- **Execution Recipe**:
  1. Time-align the midpoint of $A$'s sustained note to the onset/sustain of $B$'s note.
  2. Apply **formant-preserving pitch shift** to $B$ by $+\Delta p$ cents at entry so $B$'s vocal/lead enters in **exact unison pitch** with $A$'s held note.
  3. Crossfade the $E_{\text{vocal}}$ band using an equal-power S-curve over $1.2\text{–}2.5\text{ s}$ while cutting $A$'s residual low-end.
  4. As $B$'s sustain ends and its first rhythmic downbeat approaches, smoothly glide $B$'s pitch shift from $+\Delta p \rightarrow 0\text{ cents}$ over a chord transition or breath boundary.

### Archetype 2: Instrument Riff / Drum-Fill Handoff ("The Ado $\rightarrow$ Punk / Aggressive Rock Cut")
- **Eligibility**:
  - Both $A$ and $B$ have high Aggression ($\mathcal{A} \ge 0.65$) OR share a high cosine similarity in Dominant Instrument Signature $\mathbf{I}$ (e.g., both feature `inst_distorted_guitar`, `inst_brass_horns`, or `vocal_grit_fry_growl`).
- **Why Standard Crossfades Fail Here**:
  - Blending two distorted punk guitars or two dense J-Rock drum patterns at $-3\text{ dB}$ for 8 seconds produces phase cancellation and harsh midrange mud ($1\text{–}3.5\text{ kHz}$).
- **Execution Recipe**:
  1. Locate a **Phrase Final Hit / Scream / Breakdown Stop** in $A$ and a **Pickup Drum Fill / Guitar Riff Intro / Anacrusis** in $B$ (typically $0.5$ to $2$ bars before $B$'s first downbeat).
  2. Time-warp $B$'s pickup fill to match $A$'s outgoing tempo.
  3. Punch $B$'s pickup fill through on the exact subdivision ($1/2$ or $1$ bar before $A$'s final downbeat) while applying a dynamic notch ($1\text{–}3.5\text{ kHz}$) to $A$ so $B$'s guitar/snare transient slices cleanly through the mix.
  4. On $B$'s Downbeat 1, hard-cut $A$'s dry signal and let only a $500\text{ ms}$ tempo-synced reverb tail decay.

### Archetype 3: Beat-Locked 3-Band EQ Swap ("The Pop / Dance / Slow-Rock Groove Mix")
- **Eligibility**:
  - Both tracks have stable beat grids with $|\text{BPM}_A - \text{BPM}_B| / \text{BPM}_A \le 12\%$ and low-to-moderate distortion roughness.
- **Execution Recipe**:
  1. Phase-lock beat grids and align 4- or 8-bar phrase boundaries.
  2. Ramp $B$ in over $4\text{–}8$ bars with $E_{\text{sub}}(B)$ muted ($-24\text{ dB}$ low-shelf cut) so two kick/basslines never collide.
  3. On the target phrase downbeat, execute a **1-beat Bass Crossover Swap** ($E_{\text{sub}}(A) \rightarrow -24\text{ dB}$, $E_{\text{sub}}(B) \rightarrow 0\text{ dB}$) while completing the mid/high fade.

### Archetype 4: Tail Freeze / Shimmer Wash ("The Cross-Genre / High-$\Delta\mathcal{A}$ Bridge")
- **Eligibility**:
  - Triggered when BPM gap $> 15\%$ OR when transitioning between incompatible vibes (e.g., Aggressive Punk $\mathcal{A}=0.88 \rightarrow$ Slow Acoustic Pop $\mathcal{A}=0.20$).
- **Execution Recipe**:
  1. On the final phrase downbeat or vocal phrase end of $A$, feed the last $250\text{ ms}$ of $A$'s vocal/harmonic band into a stereo tempo-synced delay + lush plate/shimmer reverb freeze ($2.5\text{–}4.0\text{ s}$ decay).
  2. Duck $A$'s dry signal immediately so its aggressive drums stop cleanly on the bar line.
  3. Drop $B$'s intro in over the decaying harmonic wash of $A$ (optionally pitch-shifting the reverb tail by $\pm 1\text{–}2$ semitones to resolve into $B$'s opening chord!).

### Archetype 5: Transient-Aligned Natural Decay Cut
- **Eligibility**:
  - $A$ already has a natural acoustic/reverb ring-out tail, and $B$ starts with a crisp transient attack.
- **Execution Recipe**:
  - Align $B$'s first transient attack to the exact timestamp where $A$'s ring-out crosses $-18\text{ dBFS}$, preserving 100% native audio fidelity with zero time/pitch DSP artifacts.
