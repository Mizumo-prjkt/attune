# Attune — Lyrics Engine, Formatting & Custom Styling Manual

Welcome to the definitive user and developer guide for **Attune's Synchronized Lyrics Engine**.

Attune incorporates a high-fidelity, GPU-composited lyrics rendering system inspired by modern lyrics engines like Better Lyrics, Apple Music, and AMLL (Apple Music Like Lyrics). It supports standard line-by-line LRC files as well as syllable-level, word-by-word timed lyrics (**TTML** and **Enhanced LRC / ELRC**), dynamic cover art atmospheric gradients, tactile spring physics, and custom CSS theme injection.

---

## 1. Supported Formats & File Specifications

Attune automatically resolves and pairs lyric companion files located either in the same directory as the audio track, or inside the user cache (`~/.cache/attune-desktop/lyrics/`).

| Format | Extension | Sync Resolution | Syntax Example |
| :--- | :--- | :--- | :--- |
| **Standard LRC** | `.lrc` | Line-by-Line | `[00:15.20] Step onto the floor, the frequency is clear` |
| **Enhanced LRC** | `.elrc`, `.yrc` | Syllable / Word-by-Word | `[00:15.20] <00:15.20>Step <00:15.50>onto <00:15.85>the <00:16.05>floor` |
| **NetEase YRC** | `.yrc` | Syllable-by-Syllable | `[00:15.20](0,300,0)Step (300,350,0)onto (650,200,0)the` |
| **Apple Music TTML** | `.ttml`, `.xml` | Syllable / Word XML Tree | `<p begin="00:15.20" end="00:17.90"><span begin="00:15.20" end="00:15.50">Step </span><span begin="00:15.50" end="00:15.85">onto </span></p>` |
| **Unsynchronized Text** | `.txt` | Static Lines | `Plain text lyrics displayed as clean readable paragraphs` |

### Companion File Naming Convention
To ensure automatic association, place companion files alongside the song file with identical file stems:
```text
/home/user/Music/
├── DECO*27 - (Not) A Devil.flac
├── DECO*27 - (Not) A Devil.elrc       <-- Primary Syllable Sync
└── DECO*27 - (Not) A Devil.lrc        <-- Fallback Line Sync
```

### Priority Resolution Order
When searching for lyrics, Attune follows the configured priority in **Settings > Lyrics > Default Lyrics Priority**:
1. **Embedded Audio Tags**: Vorbis Comment `LYRICS` / `UNSYNCEDLYRICS` (FLAC/OGG), ID3v2 `USLT` / `SYLT` (MP3).
2. **Local Companion Files**: `.elrc` $\rightarrow$ `.ttml` $\rightarrow$ `.yrc` $\rightarrow$ `.lrc` $\rightarrow$ `.txt`.
3. **LRCLIB Public Database**: Online open API for synchronized and unsynchronized text.
4. **BiniLyrics**: Syllable-level enhanced and Apple Music TTML streams.

---

## 2. Settings & Visual Customization

All lyrics visual settings can be accessed through the **Settings (`⚙`) > Lyrics** tab.

### Typography & Font Family Dropdown
Attune features a grouped font selector dropdown under **Settings > Lyrics > Lyrics Font Family**:
* **Default & Attune Fonts (`<optgroup>`)**:
  - `System Default (Adaptive)`: Attune's adaptive system UI font stack with native CJK and Latin fallbacks.
  - `Plus Jakarta Sans`: Attune's signature body and lyrics typeface.
  - `Sora`: Attune's signature geometric display typeface.
  - `Inter`: High-legibility modern sans-serif.
  - `Roboto`: Neutral neo-grotesque sans-serif.
* **System Installed Fonts (`<optgroup>`)**:
  - Discovered cross-platform on Linux via `fontconfig` (`fc-list`) and on Windows via system registry and font directories.
  - **Deduplication Guard**: Any of Attune's default fonts (`Plus Jakarta Sans`, `Sora`, `Inter`, `Roboto`) that happen to be installed on the user's host machine are automatically excluded from the "System Fonts" group to prevent duplicate entries.
* **Live Typography Preview**: An interactive preview card displays sample English and Japanese text immediately in the chosen font face with CSS variable metrics (`--lyrics-font-family`).

### Fullscreen Layout Placement
Controls which side of the edge-to-edge fullscreen window hosts album artwork and player controls versus the lyrics column:
* **Controls Left, Lyrics Right** (Default): Standard player layout with cover art on the left and scrolling lyrics on the right.
* **Controls Right, Lyrics Left**: Reverses the layout, placing album art and transport controls on the right and lyrics on the left.

### Text Alignment Controls
* **Sidebar Lyrics Alignment**: Governs the right-docked player drawer (`Left`, `Center`, or `Right`).
* **Fullscreen Lyrics Alignment**: Governs the edge-to-edge immersive stage (`Left`, `Center`, or `Right`).
  > [!IMPORTANT]
  > When **Custom Fullscreen Lyrics CSS Animation** is enabled, declarative fullscreen alignment is automatically disabled. Text alignment is governed exclusively by your custom CSS rules (`text-align` inside `.fullscreen-lyric-line`). The docked sidebar lyrics remain governed by the sidebar alignment setting.

### Visual Effects & Atmospheric Meshes
* **Active Lyric Glowing Shadow**: Applies a high-visibility cream-white drop-shadow glow to the currently singing line.
* **Album Art Atmospheric Gradient**: Generates a dynamic multi-point background gradient extracted in real time from the active album cover art palette.
* **Visualizer Palette Pulse Effects**: Renders ambient breathing color orbs behind the lyrics that pulsate with the music.

---

## 3. Syllable & Word-by-Word Synchronization (TTML & ELRC)

For enhanced syllable-timed lyrics (`.elrc` and `.ttml`), Attune segments the active singing line into individual span tokens (`.fullscreen-lyric-word`).

### Word State Lifecycle
As time advances within an active line, each word transitions through three distinct CSS state classes:

```
[ Line Becomes Active (.fullscreen-lyric-active) ]
                         │
                         ▼
        ┌───────────────────────────────────┐
        │ 1. .word-upcoming (Pre-singing)   │ --> Dimmed, subtle opacity (0.40)
        └─────────────────┬─────────────────┘
                          │ (Timestamp Reached)
                          ▼
        ┌───────────────────────────────────┐
        │ 2. .word-active   (Being Sung)    │ --> Full scale pop, bright glow, vivid color
        └─────────────────┬─────────────────┘
                          │ (Duration Expired)
                          ▼
        ┌───────────────────────────────────┐
        │ 3. .word-sung     (Already Sung)  │ --> Crisp solid text (0.95 opacity)
        └───────────────────────────────────┘
```

---

## 4. Custom CSS Keyframe & Typography Guide

Attune provides a built-in monospace CSS editor in **Settings > Lyrics > Custom Fullscreen Lyrics CSS Animation**. Custom CSS rules are scoped exclusively to the Fullscreen Lyrics stage and are saved directly to `~/.config/attune/config.json`.

### Class Selector Reference

| Selector | Element Description | Recommended Properties |
| :--- | :--- | :--- |
| `.fullscreen-lyric-line` | Base container for each line | `line-height`, `text-align`, `transform-origin`, `transition` |
| `.fullscreen-lyric-active` | Currently singing line | `color`, `font-weight`, `text-shadow`, `animation` |
| `.fullscreen-lyric-inactive` | Past & upcoming lines | `color`, `opacity`, `filter: blur(...)`, `transform: scale(...)` |
| `.fullscreen-lyric-word` | Syllable/word span container | `display: inline-block`, `transition`, `margin-right` |
| `.fullscreen-lyric-word.word-active` | Active syllable currently sung | `color`, `text-shadow`, `animation: springBounce`, `transform` |
| `.fullscreen-lyric-word.word-sung` | Already sung syllable in line | `opacity: 1`, `color` |
| `.fullscreen-lyric-word.word-upcoming`| Upcoming syllable in line | `opacity: 0.40` |
| `.fullscreen-lyrics-scroll` | Scroll viewport container | `padding`, `mask-image`, `scroll-behavior` |
| `[data-line-idx="N"]` | Target specific line by zero-based index | Targeted color or emphasis |

> [!TIP]
> **Performance Best Practice**: To ensure 60/120 FPS buttery-smooth scrolling, always animate GPU-composited properties (`transform: scale(...)`, `transform: translateY(...)`, `opacity`, `filter`) rather than reflow-heavy properties (`font-size`, `margin`, `padding`, `top`).

---

## 5. Built-in Animation Presets

Attune comes with four pre-tuned presets that can be applied with one click in the CSS editor toolbar:

### Preset 1: Spotlight Depth Blur
Blurs upcoming and past lines with a focal depth-of-field effect, spotlighting the singing line.
```css
/* Spotlight Depth Blur (Better Lyrics Inspired) */
.fullscreen-lyric-inactive {
  filter: blur(2.5px);
  opacity: 0.35;
  transform: scale(0.96);
  transition: all 0.4s cubic-bezier(0.22, 1, 0.36, 1);
}

.fullscreen-lyric-active {
  filter: blur(0px);
  opacity: 1;
  font-weight: 700;
  transform: scale(1.05);
  transition: all 0.35s cubic-bezier(0.22, 1, 0.36, 1);
}
```

### Preset 2: Apple Music Spring Pop & Float
Tactile spring physics with dynamic expansion and upward momentum.
```css
/* Apple Music Spring Pop & Float */
.fullscreen-lyric-active {
  animation: appleSpringPop 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275) both;
}

@keyframes appleSpringPop {
  0% { transform: scale(0.96) translateY(4px); }
  70% { transform: scale(1.055) translateY(-2px); }
  100% { transform: scale(1.04) translateY(0); }
}
```

### Preset 3: Cyberpunk Neon Pulse Glow
Atmospheric cyan-magenta dual neon glow with subtle breathing flicker.
```css
/* Cyberpunk Neon Pulse Glow */
.fullscreen-lyric-active {
  color: #00f5d4;
  text-shadow: 0 0 10px #00f5d4, 0 0 25px rgba(0, 245, 212, 0.6), 0 0 50px rgba(255, 0, 127, 0.4);
  font-weight: 700;
  animation: neonFlicker 3s infinite alternate ease-in-out;
}

@keyframes neonFlicker {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.92; }
}
```

### Preset 4: Karaoke Golden Bounce
Vivid golden `#FFDF00` illumination with individual syllable pops.
```css
/* Karaoke Golden Glow & Tactile Bounce */
.fullscreen-lyric-active {
  color: #FFDF00;
  font-weight: 800;
  text-shadow: 0 0 12px rgba(255, 223, 0, 0.95), 0 0 28px rgba(255, 170, 0, 0.65);
  animation: karaokeBounce 0.45s cubic-bezier(0.175, 0.885, 0.32, 1.275) both;
}

.fullscreen-lyric-word.word-active {
  color: #FFF275;
  text-shadow: 0 0 18px #FFDF00, 0 0 35px #FFA726;
  transform: scale(1.15) translateY(-2px);
}

@keyframes karaokeBounce {
  0% { transform: scale(0.96) translateY(3px); }
  65% { transform: scale(1.08) translateY(-2px); }
  100% { transform: scale(1.06) translateY(0); }
}
```

---

## 6. Standalone Live Lyrics Animation Preview Window

To inspect lyrics styles, alignments, and syllable animations without playing music, open the **Live Lyrics Preview Window**:

1. Open **Settings (`⚙`) > Lyrics**.
2. Click **`[Preview Animations & Looks]`** in either the Fullscreen card or Custom CSS card.
3. A dedicated secondary OS window titled **`Attune — Lyrics Live Animation Preview`** will open.

### Preview Window Features
* **Live Two-Way Synchronization**: Polls settings every 500ms. Typing in the custom CSS editor or clicking presets immediately updates the preview with zero restart.
* **Format Switcher**:
  * **Line Sync (LRC)**: Tests whole-line animation transitions.
  * **Word Sync (TTML/ELRC)**: Simulates millisecond syllable progression (`.word-upcoming` $\rightarrow$ `.word-active` $\rightarrow$ `.word-sung`).
* **Viewport Switcher**: Toggle between **Fullscreen View** (edge-to-edge split) and **Docked Sidebar** (360px drawer).
* **Sample Songs**: Switch between Japanese Vocaloid J-Rock (`(Not) A Devil`), English Electronic Dance (`Ready`), and Melodic Pop (`YUMEYUME`).
* **Interactive Controls**:
  * Step to next / previous lines.
  * Adjust playback simulation speed (`0.5x`, `1.0x`, `2.0x`).
  * Click any line or syllable word to jump the simulation immediately to that exact timestamp.

---

## 7. Troubleshooting & FAQ

#### Why are the Fullscreen Lyrics Alignment buttons disabled?
When **Custom Fullscreen Lyrics CSS Animation** is active, declarative alignment is disabled to prevent CSS rule collisions. You can set alignment directly in your custom CSS:
```css
.fullscreen-lyric-line {
  text-align: center; /* or left, right */
}
```

#### Why are non-English characters (Japanese / Chinese / Korean) not rendering properly?
Ensure companion `.lrc` and `.elrc` files are saved with **UTF-8 encoding**. Legacy Shift-JIS or GBK encoded files can be converted to UTF-8 using any text editor or `iconv`.

#### What is the 40ms Lookahead?
Attune offsets lyric timestamps by 40ms before hardware DAC rendering. This compensates for ALSA audio buffer latencies and human visual perception delay, ensuring singing highlights align with audio transients.
