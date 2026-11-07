//! Liquid Material Glass theme & dynamic color engine for Attune.
//!
//! Complies with `ledger/05_UI_DESIGN_SPEC.md`:
//! - Primary Accent: `#2DD4BF` (Prismatic Mint / Teal)
//! - Secondary Accent: `#A855F7` (Electric Violet)
//! - Tertiary Accent: `#FB7185` (Coral Rose)
//! - Dynamic album cover art color extraction with multi-layer vibrant palette generation
//! - Strictly adheres to Mandate 4 (zero hardcoded values, graceful fallbacks).

use std::path::Path;
use crate::library::TrackEntry;

pub const PRIMARY_HEX: &str = "#2DD4BF";
pub const SECONDARY_HEX: &str = "#A855F7";
pub const TERTIARY_HEX: &str = "#FB7185";

/// Injected WebKit runtime script measuring overflowing text and enabling smooth ping-pong marquee scrolling.
pub const MARQUEE_SCRIPT: &str = r#"
<script>
(function() {
  function refreshMarquees() {
    var isMovingEnabled = document.querySelector('.text-moving-enabled') !== null;
    var elements = document.querySelectorAll('.marquee-text');
    for (var i = 0; i < elements.length; i++) {
      var el = elements[i];
      var parent = el.parentElement;
      if (!parent) continue;
      var overflow = el.scrollWidth - parent.clientWidth;
      if (isMovingEnabled && overflow > 2) {
        if (!el.classList.contains('is-overflowing')) {
          el.classList.add('is-overflowing');
        }
        el.style.setProperty('--scroll-dist', (overflow + 14) + 'px');
        var dur = Math.max(4.5, Math.min(14.0, overflow / 20.0 + 3.0));
        el.style.setProperty('--scroll-dur', dur.toFixed(2) + 's');
      } else {
        if (el.classList.contains('is-overflowing')) {
          el.classList.remove('is-overflowing');
          el.style.removeProperty('--scroll-dist');
          el.style.removeProperty('--scroll-dur');
        }
      }
    }
  }

  window.addEventListener('resize', refreshMarquees);
  if (document.readyState === 'loading') {
    window.addEventListener('DOMContentLoaded', initMarqueeObserver);
  } else {
    initMarqueeObserver();
  }

  function initMarqueeObserver() {
    refreshMarquees();
    var observer = new MutationObserver(function() {
      requestAnimationFrame(refreshMarquees);
    });
    if (document.body) {
      observer.observe(document.body, { childList: true, subtree: true, characterData: true });
    }
    setInterval(refreshMarquees, 2500);
  }
})();
</script>
"#;

/// Injected WebKit runtime script for smooth auto-centering synchronized lyrics in follow mode.
pub const LYRICS_SYNC_SCRIPT: &str = r#"
<script>
(function() {
  function centerElement(container, active) {
    if (!container || !active) return;
    var cTop = container.scrollTop;
    var cRect = container.getBoundingClientRect();
    var aRect = active.getBoundingClientRect();
    var target = cTop + (aRect.top - cRect.top) - (cRect.height / 2) + (aRect.height / 2);
    container.scrollTo({ top: target, behavior: 'smooth' });
  }

  window.attuneSyncLyrics = function(force) {
    function doSync() {
      var fsContainer = document.querySelector('.fullscreen-lyrics-scroll');
      if (fsContainer && (force || fsContainer.getAttribute('data-locked') !== 'false')) {
        var active = fsContainer.querySelector('.fullscreen-lyric-active');
        if (active) {
          centerElement(fsContainer, active);
        }
      }
      var dockContainer = document.querySelector('.dock-scroll');
      if (dockContainer && (force || dockContainer.getAttribute('data-locked') !== 'false')) {
        var activeDock = dockContainer.querySelector('.lyric-line.active');
        if (activeDock) {
          centerElement(dockContainer, activeDock);
        }
      }
    }
    doSync();
    requestAnimationFrame(doSync);
  };

  var lyricsObserver = new MutationObserver(function(mutations) {
    var shouldSync = false;
    for (var i = 0; i < mutations.length; i++) {
      var m = mutations[i];
      if (m.type === 'attributes' && m.attributeName === 'class') {
        var t = m.target;
        if (t && (t.classList.contains('fullscreen-lyric-active') || (t.classList.contains('lyric-line') && t.classList.contains('active')))) {
          shouldSync = true;
          break;
        }
      } else if (m.type === 'childList') {
        for (var j = 0; j < m.addedNodes.length; j++) {
          var node = m.addedNodes[j];
          if (node.nodeType === 1 && (node.classList.contains('fullscreen-lyric-active') || (node.classList.contains('lyric-line') && node.classList.contains('active')) || node.querySelector('.fullscreen-lyric-active') || node.querySelector('.lyric-line.active'))) {
            shouldSync = true;
            break;
          }
        }
      }
    }
    if (shouldSync) {
      window.attuneSyncLyrics(false);
    }
  });

  function initLyricsObserver() {
    if (document.body) {
      lyricsObserver.observe(document.body, { attributes: true, subtree: true, attributeFilter: ['class'], childList: true });
    }
  }

  if (document.readyState === 'loading') {
    window.addEventListener('DOMContentLoaded', initLyricsObserver);
  } else {
    initLyricsObserver();
  }
})();
</script>
"#;

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicPalette {
    pub primary: String,
    pub secondary: String,
    pub tertiary: String,
    pub primary_rgb: (u8, u8, u8),
    pub secondary_rgb: (u8, u8, u8),
    pub tertiary_rgb: (u8, u8, u8),
    pub dark_base_rgb: (u8, u8, u8),
}

impl Default for DynamicPalette {
    fn default() -> Self {
        Self {
            primary: PRIMARY_HEX.to_string(),
            secondary: SECONDARY_HEX.to_string(),
            tertiary: TERTIARY_HEX.to_string(),
            primary_rgb: (0x2D, 0xD4, 0xBF),
            secondary_rgb: (0xA8, 0x55, 0xF7),
            tertiary_rgb: (0xFB, 0x71, 0x85),
            dark_base_rgb: (0x0A, 0x0E, 0x17),
        }
    }
}

impl DynamicPalette {
    /// Extracts dominant vibrant colors from an image file on disk.
    pub fn from_image(path: &Path) -> Option<Self> {
        let reader = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?;
        let img = reader.decode().ok()?;
        Some(Self::from_dynamic_image(&img))
    }

    /// Extracts dominant vibrant colors from a base64 Data URL (e.g. embedded artwork).
    pub fn from_data_url(data_url: &str) -> Option<Self> {
        if !data_url.starts_with("data:image/") {
            return None;
        }
        let comma_idx = data_url.find(',')?;
        let base64_payload = &data_url[comma_idx + 1..];
        use base64::Engine;
        let bytes = base64::prelude::BASE64_STANDARD.decode(base64_payload.trim()).ok()?;
        let img = image::load_from_memory(&bytes).ok()?;
        Some(Self::from_dynamic_image(&img))
    }

    /// Resolves dynamic palette from a `TrackEntry` using artwork path, data URL,
    /// or deterministic harmonic hash fallback.
    pub fn from_track(track: &TrackEntry) -> Self {
        if let Some(ref p) = track.artwork_path {
            if let Some(pal) = Self::from_image(p) {
                return pal;
            }
        }
        if let Some(ref data_url) = track.artwork_data_url {
            if let Some(pal) = Self::from_data_url(data_url) {
                return pal;
            }
        }
        let seed = format!("{}-{}-{}", track.title, track.artist, track.album);
        Self::from_hash(&seed)
    }

    /// Deterministic harmonic color palette generator for tracks without artwork.
    pub fn from_hash(seed_str: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(seed_str, &mut hasher);
        let hash = std::hash::Hasher::finish(&hasher);

        let hue_deg = (hash % 360) as f32;
        let p_rgb = hsl_to_rgb(hue_deg, 0.75, 0.55);
        let s_rgb = hsl_to_rgb((hue_deg + 45.0) % 360.0, 0.70, 0.50);
        let t_rgb = hsl_to_rgb((hue_deg + 180.0) % 360.0, 0.65, 0.52);
        let dark_rgb = (
            ((p_rgb.0 as f32 * 0.10) as u8).saturating_add(8),
            ((p_rgb.1 as f32 * 0.10) as u8).saturating_add(10),
            ((p_rgb.2 as f32 * 0.10) as u8).saturating_add(14),
        );

        Self {
            primary: format!("rgb({}, {}, {})", p_rgb.0, p_rgb.1, p_rgb.2),
            secondary: format!("rgb({}, {}, {})", s_rgb.0, s_rgb.1, s_rgb.2),
            tertiary: format!("rgb({}, {}, {})", t_rgb.0, t_rgb.1, t_rgb.2),
            primary_rgb: p_rgb,
            secondary_rgb: s_rgb,
            tertiary_rgb: t_rgb,
            dark_base_rgb: dark_rgb,
        }
    }

    /// Analyzes an image and extracts a multi-color vibrant harmonic palette.
    pub fn from_dynamic_image(img: &image::DynamicImage) -> Self {
        let thumb = img.thumbnail(48, 48);
        let rgb_img = thumb.to_rgb8();

        // 12 hue sectors (0..360 in 30-degree buckets)
        struct HueBucket {
            count: usize,
            total_sat: f32,
            max_sat: f32,
            best_rgb: (u8, u8, u8),
            avg_hue: f32,
        }

        let mut buckets: Vec<HueBucket> = (0..12)
            .map(|_| HueBucket {
                count: 0,
                total_sat: 0.0,
                max_sat: 0.0,
                best_rgb: (0, 0, 0),
                avg_hue: 0.0,
            })
            .collect();

        for p in rgb_img.pixels() {
            let r = p[0] as f32 / 255.0;
            let g = p[1] as f32 / 255.0;
            let b = p[2] as f32 / 255.0;

            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let delta = max - min;
            let brightness = (r + g + b) / 3.0;

            // Filter out muddy blacks, washed out whites, and uncolored grays
            if brightness < 0.12 || brightness > 0.92 || delta < 0.12 {
                continue;
            }

            let sat = if max > 0.0 { delta / max } else { 0.0 };

            let hue = if delta < 0.0001 {
                0.0
            } else if (max - r).abs() < 0.0001 {
                60.0 * (((g - b) / delta) % 6.0)
            } else if (max - g).abs() < 0.0001 {
                60.0 * (((b - r) / delta) + 2.0)
            } else {
                60.0 * (((r - g) / delta) + 4.0)
            };
            let norm_hue = (hue + 360.0) % 360.0;
            let bucket_idx = ((norm_hue / 30.0) as usize).min(11);

            let b_entry = &mut buckets[bucket_idx];
            b_entry.count += 1;
            b_entry.total_sat += sat;
            b_entry.avg_hue = norm_hue;
            if sat > b_entry.max_sat {
                b_entry.max_sat = sat;
                b_entry.best_rgb = (p[0], p[1], p[2]);
            }
        }

        // Score buckets by count * avg_saturation
        let mut scored_buckets: Vec<(usize, f32, (u8, u8, u8), f32)> = buckets
            .into_iter()
            .enumerate()
            .filter(|(_, b)| b.count > 0)
            .map(|(idx, b)| {
                let avg_sat = b.total_sat / b.count as f32;
                let score = b.count as f32 * avg_sat * b.max_sat;
                (idx, score, b.best_rgb, b.avg_hue)
            })
            .collect();

        scored_buckets.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if scored_buckets.is_empty() {
            return Self::default();
        }

        let primary_rgb = scored_buckets[0].2;
        let primary_hue = scored_buckets[0].3;

        // Find secondary bucket with distinct hue distance (>= 45 degrees)
        let secondary_rgb = scored_buckets
            .iter()
            .skip(1)
            .find(|(_, _, _, h)| {
                let diff = (h - primary_hue).abs();
                let circ_diff = diff.min(360.0 - diff);
                circ_diff >= 45.0
            })
            .map(|(_, _, rgb, _)| *rgb)
            .unwrap_or_else(|| {
                // If the album is monochromatic (e.g. solid red with white/black), generate analog hue shift
                hsl_to_rgb((primary_hue + 35.0) % 360.0, 0.70, 0.52)
            });

        // Find tertiary bucket or complementary hue
        let tertiary_rgb = scored_buckets
            .iter()
            .skip(1)
            .find(|(_, _, rgb, h)| {
                let diff = (h - primary_hue).abs();
                let circ_diff = diff.min(360.0 - diff);
                circ_diff >= 85.0 && *rgb != secondary_rgb
            })
            .map(|(_, _, rgb, _)| *rgb)
            .unwrap_or_else(|| {
                hsl_to_rgb((primary_hue + 160.0) % 360.0, 0.65, 0.50)
            });

        let dark_base_rgb = (
            ((primary_rgb.0 as f32 * 0.12) as u8).saturating_add(8),
            ((primary_rgb.1 as f32 * 0.12) as u8).saturating_add(10),
            ((primary_rgb.2 as f32 * 0.12) as u8).saturating_add(14),
        );

        Self {
            primary: format!("rgb({}, {}, {})", primary_rgb.0, primary_rgb.1, primary_rgb.2),
            secondary: format!("rgb({}, {}, {})", secondary_rgb.0, secondary_rgb.1, secondary_rgb.2),
            tertiary: format!("rgb({}, {}, {})", tertiary_rgb.0, tertiary_rgb.1, tertiary_rgb.2),
            primary_rgb,
            secondary_rgb,
            tertiary_rgb,
            dark_base_rgb,
        }
    }
}

/// Helper converting HSL values (hue: 0..360, sat: 0..1, lit: 0..1) to RGB (u8, u8, u8).
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (r_prime, g_prime, b_prime) = if (0.0..60.0).contains(&h) {
        (c, x, 0.0)
    } else if (60.0..120.0).contains(&h) {
        (x, c, 0.0)
    } else if (120.0..180.0).contains(&h) {
        (0.0, c, x)
    } else if (180.0..240.0).contains(&h) {
        (0.0, x, c)
    } else if (240.0..300.0).contains(&h) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        (((r_prime + m) * 255.0).clamp(0.0, 255.0)) as u8,
        (((g_prime + m) * 255.0).clamp(0.0, 255.0)) as u8,
        (((b_prime + m) * 255.0).clamp(0.0, 255.0)) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_palette_default() {
        let def = DynamicPalette::default();
        assert_eq!(def.primary, PRIMARY_HEX);
        assert_eq!(def.primary_rgb, (0x2D, 0xD4, 0xBF));
    }

    #[test]
    fn test_dynamic_palette_from_hash() {
        let pal1 = DynamicPalette::from_hash("Nayutan Seijin - Dance Robot Dance");
        let pal2 = DynamicPalette::from_hash("Nayutan Seijin - Dance Robot Dance");
        assert_eq!(pal1, pal2);
        assert!(!pal1.primary.is_empty());
        assert!(!pal1.secondary.is_empty());
    }

    #[test]
    fn test_dynamic_palette_from_solid_color_image() {
        use image::{ImageBuffer, Rgb};
        // Create 32x32 image with solid coral red (similar to Nayutan cover)
        let img = ImageBuffer::from_fn(32, 32, |_, _| Rgb([230u8, 100u8, 100u8]));
        let dyn_img = image::DynamicImage::ImageRgb8(img);
        let pal = DynamicPalette::from_dynamic_image(&dyn_img);

        assert!((pal.primary_rgb.0 as i16 - 230).abs() <= 2);
        assert!((pal.primary_rgb.1 as i16 - 100).abs() <= 2);
        assert!((pal.primary_rgb.2 as i16 - 100).abs() <= 2);
        assert!(pal.primary.contains("230, "));
        // Secondary and tertiary generated harmoniously
        assert_ne!(pal.primary_rgb, pal.secondary_rgb);
    }
}
