use dioxus::prelude::*;
use std::cmp::Ordering;

// ============================================================================
// 1. IN-APP GITHUB RELEASES UPDATE CHECKER & SEMVER 2.0.0 ENGINE
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    UpToDate,
    UpdateAvailable,
    NoReleasesFound,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub size_bytes: u64,
    pub is_debug: bool,
    pub platform_category: String, // "linux", "windows", "android", "other"
}

impl ReleaseAsset {
    pub fn from_json(val: &serde_json::Value) -> Self {
        let name = val
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let download_url = val
            .get("browser_download_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let size_bytes = val.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

        let lower = name.to_ascii_lowercase();
        let is_debug = lower.contains("debug");

        let platform_category = if lower.contains("appimage")
            || lower.contains("linux")
            || lower.ends_with(".tar.gz")
        {
            "linux".to_string()
        } else if lower.contains("setup")
            || lower.contains("windows")
            || lower.ends_with(".exe")
            || (lower.ends_with(".zip") && lower.contains("win"))
        {
            "windows".to_string()
        } else if lower.ends_with(".apk") || lower.contains("android") {
            "android".to_string()
        } else {
            "other".to_string()
        };

        Self {
            name,
            download_url,
            size_bytes,
            is_debug,
            platform_category,
        }
    }

    pub fn formatted_size(&self) -> String {
        if self.size_bytes == 0 {
            return String::new();
        }
        if self.size_bytes >= 1024 * 1024 {
            format!("{:.1} MB", (self.size_bytes as f64) / (1024.0 * 1024.0))
        } else {
            format!("{:.1} KB", (self.size_bytes as f64) / 1024.0)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateInfo {
    pub tag_name: String,
    pub title: String,
    pub release_notes: String,
    pub html_url: String,
    pub is_prerelease: bool,
    pub is_newer: bool,
    pub assets: Vec<ReleaseAsset>,
}

impl UpdateInfo {
    pub fn from_json(val: &serde_json::Value, is_newer: bool) -> Self {
        let tag_name = val
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("v0.0.0")
            .to_string();
        let title = val
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&tag_name)
            .to_string();
        let release_notes = val
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let html_url = val
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://github.com/Mizumo-prjkt/attune/releases")
            .to_string();
        let is_prerelease = val
            .get("prerelease")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let assets = val
            .get("assets")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(ReleaseAsset::from_json).collect())
            .unwrap_or_default();

        Self {
            tag_name,
            title,
            release_notes,
            html_url,
            is_prerelease,
            is_newer,
            assets,
        }
    }

    pub fn host_platform_assets(&self) -> Vec<ReleaseAsset> {
        let current_platform = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            "other"
        };

        let matched: Vec<ReleaseAsset> = self
            .assets
            .iter()
            .filter(|a| a.platform_category == current_platform)
            .cloned()
            .collect();

        if matched.is_empty() {
            self.assets.clone()
        } else {
            matched
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateCheckResult {
    pub status: UpdateStatus,
    pub update_info: Option<UpdateInfo>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedVersion {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<String>,
}

impl ParsedVersion {
    fn parse(raw: &str) -> Self {
        let mut cleaned = raw.trim();
        if let Some(rest) = cleaned.strip_prefix('v').or_else(|| cleaned.strip_prefix('V')) {
            cleaned = rest.trim();
        }
        // Strip build metadata (+...) per SemVer 2.0.0
        if let Some((before_plus, _)) = cleaned.split_once('+') {
            cleaned = before_plus;
        }

        let (core_part, pre_part) = match cleaned.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (cleaned, None),
        };

        let mut nums = core_part
            .split('.')
            .map(|seg| {
                let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse::<u64>().unwrap_or(0)
            });

        let major = nums.next().unwrap_or(0);
        let minor = nums.next().unwrap_or(0);
        let patch = nums.next().unwrap_or(0);

        let prerelease = match pre_part {
            Some(p) if !p.trim().is_empty() => Self::tokenize_prerelease(p.trim()),
            _ => Vec::new(),
        };

        Self {
            major,
            minor,
            patch,
            prerelease,
        }
    }

    fn tokenize_prerelease(pre: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        for dot_part in pre.split(&['.', '-', '_'][..]) {
            if dot_part.is_empty() {
                continue;
            }
            // Split transitions between letters and digits so "rc4" < "rc10"
            let mut current = String::new();
            let mut last_was_digit: Option<bool> = None;
            for ch in dot_part.chars() {
                let is_digit = ch.is_ascii_digit();
                if let Some(prev_digit) = last_was_digit {
                    if prev_digit != is_digit && !current.is_empty() {
                        tokens.push(current.to_ascii_lowercase());
                        current = String::new();
                    }
                }
                current.push(ch);
                last_was_digit = Some(is_digit);
            }
            if !current.is_empty() {
                tokens.push(current.to_ascii_lowercase());
            }
        }
        tokens
    }
}

/// Compares semantic version `current` against `remote` according to SemVer 2.0.0
/// with natural pre-release token precedence (`rc3 < rc4 < rc10`, `0.1.1-rc1 < 0.1.1`).
pub fn compare_semantic_versions(current: &str, remote: &str) -> Ordering {
    let v_curr = ParsedVersion::parse(current);
    let v_rem = ParsedVersion::parse(remote);

    match v_curr.major.cmp(&v_rem.major) {
        Ordering::Equal => {}
        ord => return ord,
    }
    match v_curr.minor.cmp(&v_rem.minor) {
        Ordering::Equal => {}
        ord => return ord,
    }
    match v_curr.patch.cmp(&v_rem.patch) {
        Ordering::Equal => {}
        ord => return ord,
    }

    // Stable release (empty prerelease) has higher precedence than a prerelease
    match (v_curr.prerelease.is_empty(), v_rem.prerelease.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }

    let max_len = v_curr.prerelease.len().max(v_rem.prerelease.len());
    for i in 0..max_len {
        let a = match v_curr.prerelease.get(i) {
            Some(s) => s,
            None => return Ordering::Less,
        };
        let b = match v_rem.prerelease.get(i) {
            Some(s) => s,
            None => return Ordering::Greater,
        };
        if a == b {
            continue;
        }
        match (a.parse::<u64>(), b.parse::<u64>()) {
            (Ok(na), Ok(nb)) => return na.cmp(&nb),
            (Ok(_), Err(_)) => return Ordering::Less,
            (Err(_), Ok(_)) => return Ordering::Greater,
            (Err(_), Err(_)) => return a.cmp(b),
        }
    }

    Ordering::Equal
}

/// Queries the GitHub Releases API for `Mizumo-prjkt/attune` and compares against `current_version`.
pub fn check_for_updates(current_version: &str) -> UpdateCheckResult {
    let effective = current_version.trim();
    if effective.is_empty() {
        return UpdateCheckResult {
            status: UpdateStatus::Error,
            update_info: None,
            message: "Unable to determine current application version.".to_string(),
        };
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(8))
        .timeout_read(std::time::Duration::from_secs(8))
        .build();

    let latest_url = "https://api.github.com/repos/Mizumo-prjkt/attune/releases/latest";
    let all_releases_url = "https://api.github.com/repos/Mizumo-prjkt/attune/releases?per_page=5";

    let release_json: serde_json::Value = match agent
        .get(latest_url)
        .set("User-Agent", &format!("Attune-Desktop/{effective}"))
        .set("Accept", "application/vnd.github.v3+json")
        .call()
    {
        Ok(resp) => match resp.into_string().ok().and_then(|s| serde_json::from_str(&s).ok()) {
            Some(v) => v,
            None => {
                return UpdateCheckResult {
                    status: UpdateStatus::Error,
                    update_info: None,
                    message: "Invalid JSON response from GitHub Releases API.".to_string(),
                };
            }
        },
        Err(ureq::Error::Status(404, _)) => {
            // If all published releases are pre-releases (e.g. -rc1), /releases/latest returns 404.
            // Fall back to /releases list.
            match agent
                .get(all_releases_url)
                .set("User-Agent", &format!("Attune-Desktop/{effective}"))
                .set("Accept", "application/vnd.github.v3+json")
                .call()
            {
                Ok(resp) => {
                    let parsed: Option<serde_json::Value> =
                        resp.into_string().ok().and_then(|s| serde_json::from_str(&s).ok());
                    match parsed.and_then(|v| v.as_array().and_then(|arr| arr.first().cloned())) {
                        Some(first_rel) => first_rel,
                        None => {
                            return UpdateCheckResult {
                                status: UpdateStatus::NoReleasesFound,
                                update_info: None,
                                message: "No published releases found on the GitHub repository yet."
                                    .to_string(),
                            };
                        }
                    }
                }
                Err(e) => {
                    return UpdateCheckResult {
                        status: UpdateStatus::Error,
                        update_info: None,
                        message: format!("Failed to query GitHub releases: {e}"),
                    };
                }
            }
        }
        Err(ureq::Error::Status(code, _)) => {
            return UpdateCheckResult {
                status: UpdateStatus::Error,
                update_info: None,
                message: format!("GitHub API returned HTTP {code}."),
            };
        }
        Err(e) => {
            return UpdateCheckResult {
                status: UpdateStatus::Error,
                update_info: None,
                message: format!("Failed to connect to GitHub API: {e}"),
            };
        }
    };

    let remote_tag = release_json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if remote_tag.is_empty() {
        return UpdateCheckResult {
            status: UpdateStatus::NoReleasesFound,
            update_info: None,
            message: "No release tags found on GitHub.".to_string(),
        };
    }

    let cmp = compare_semantic_versions(effective, remote_tag);
    let is_newer = cmp == Ordering::Less;
    let info = UpdateInfo::from_json(&release_json, is_newer);

    if is_newer {
        UpdateCheckResult {
            status: UpdateStatus::UpdateAvailable,
            message: format!("New release available: {}", info.tag_name),
            update_info: Some(info),
        }
    } else {
        let msg = if cmp == Ordering::Greater {
            format!(
                "You are running a newer build (v{effective}) than the latest published release ({}).",
                info.tag_name
            )
        } else {
            format!("Attune is up to date ({}).", info.tag_name)
        };
        UpdateCheckResult {
            status: UpdateStatus::UpToDate,
            message: msg,
            update_info: Some(info),
        }
    }
}

// ============================================================================
// 2. CONTRIBUTORS & AI ASSISTANTS REGISTRY (WITH SORTING & FILTERING)
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContributorKind {
    HumanContributor,
    AiAssistant,
}

impl ContributorKind {
    pub fn badge_label(&self) -> &'static str {
        match self {
            ContributorKind::HumanContributor => "Contributor",
            ContributorKind::AiAssistant => "AI Assistant",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContributorSortOrder {
    DefaultRole,
    NameAsc,
    NameDesc,
    ContributorsFirst,
    AiAssistantsFirst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContributorFilter {
    All,
    ContributorsOnly,
    AiAssistantsOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContributorEntry {
    pub default_order: usize,
    pub name: &'static str,
    pub role: &'static str,
    pub kind: ContributorKind,
    pub icon: &'static str,
    pub attribution: &'static str,
    pub description: &'static str,
    pub url: Option<&'static str>,
}

pub const CONTRIBUTORS_AND_AI_ASSISTANTS: &[ContributorEntry] = &[
    ContributorEntry {
        default_order: 1,
        name: "Mizumo-prjkt",
        role: "Author, Lead Architect & Project Creator",
        kind: ContributorKind::HumanContributor,
        icon: "person",
        attribution: "bluepearl394@gmail.com",
        description: "Core product vision, DJ transition & audio quality criteria, Liquid Material Glass UI direction, empirical transition listening verification, and multi-platform release engineering.",
        url: Some("https://github.com/Mizumo-prjkt"),
    },
    ContributorEntry {
        default_order: 2,
        name: "Gemini (Google DeepMind Antigravity)",
        role: "Autonomous Coding, Mathematical Derivation & DSP Design",
        kind: ContributorKind::AiAssistant,
        icon: "auto_awesome",
        attribution: "Gemini Code Assist <gemini-code-assist@google.com>",
        description: "Pair-programmed native Rust & Dioxus/Flutter architecture, 5-layer CQT audio profiling math, DualDeckMixer & Studio Master FX DSP rack, C-ABI FFI bridges, and Knowledge Ledger ADRs.",
        url: None,
    },
    ContributorEntry {
        default_order: 3,
        name: "Claude (Anthropic)",
        role: "AI Pair Programming & Algorithmic Collaboration",
        kind: ContributorKind::AiAssistant,
        icon: "psychology",
        attribution: "Claude <noreply@anthropic.com>",
        description: "Collaborative architectural review, transition recipe heuristics, and multi-session pair programming under the Knowledge Ledger protocol.",
        url: None,
    },
];

pub fn sorted_and_filtered_contributors(
    sort_order: ContributorSortOrder,
    filter: ContributorFilter,
) -> Vec<ContributorEntry> {
    let mut list: Vec<ContributorEntry> = CONTRIBUTORS_AND_AI_ASSISTANTS
        .iter()
        .filter(|entry| match filter {
            ContributorFilter::All => true,
            ContributorFilter::ContributorsOnly => entry.kind == ContributorKind::HumanContributor,
            ContributorFilter::AiAssistantsOnly => entry.kind == ContributorKind::AiAssistant,
        })
        .cloned()
        .collect();

    match sort_order {
        ContributorSortOrder::DefaultRole => {
            list.sort_by_key(|e| e.default_order);
        }
        ContributorSortOrder::NameAsc => {
            list.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
        }
        ContributorSortOrder::NameDesc => {
            list.sort_by(|a, b| b.name.to_ascii_lowercase().cmp(&a.name.to_ascii_lowercase()));
        }
        ContributorSortOrder::ContributorsFirst => {
            list.sort_by(|a, b| {
                let rank_a = if a.kind == ContributorKind::HumanContributor { 0 } else { 1 };
                let rank_b = if b.kind == ContributorKind::HumanContributor { 0 } else { 1 };
                rank_a.cmp(&rank_b).then_with(|| a.default_order.cmp(&b.default_order))
            });
        }
        ContributorSortOrder::AiAssistantsFirst => {
            list.sort_by(|a, b| {
                let rank_a = if a.kind == ContributorKind::AiAssistant { 0 } else { 1 };
                let rank_b = if b.kind == ContributorKind::AiAssistant { 0 } else { 1 };
                rank_a.cmp(&rank_b).then_with(|| a.default_order.cmp(&b.default_order))
            });
        }
    }

    list
}

// ============================================================================
// 3. THIRD-PARTY OPEN-SOURCE LICENSES REGISTRY
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThirdPartyLicenseItem {
    pub name: &'static str,
    pub version_or_scope: &'static str,
    pub license: &'static str,
    pub url: &'static str,
    pub usage: &'static str,
    pub notice: &'static str,
}

pub const THIRD_PARTY_LICENSES: &[ThirdPartyLicenseItem] = &[
    ThirdPartyLicenseItem {
        name: "Dioxus (dioxus / dioxus-desktop / tao / wry)",
        version_or_scope: "0.6.x",
        license: "MIT OR Apache-2.0",
        url: "https://github.com/DioxusLabs/dioxus",
        usage: "Reactive cross-platform Rust UI framework and native WebKitGTK / WebView2 desktop windowing.",
        notice: "Copyright (c) 2020-2026 Jonathan Kelley and Dioxus Labs contributors. Licensed under the MIT License or Apache License, Version 2.0.",
    },
    ThirdPartyLicenseItem {
        name: "Better Lyrics (better-lyrics/better-lyrics)",
        version_or_scope: "Styling & Animation Architecture",
        license: "GPL-3.0 / Open Source Attribution",
        url: "https://github.com/better-lyrics/better-lyrics",
        usage: "Inspiration and reference for fluid cubic-bezier lyrics animations, spring physics, and custom CSS styling guides.",
        notice: "Copyright (c) Better Lyrics contributors (https://github.com/better-lyrics/better-lyrics).",
    },
    ThirdPartyLicenseItem {
        name: "BiniLyrics (<am-lyrics> by Binimum)",
        version_or_scope: "TTML & Syllable Sync Provider",
        license: "MPL-2.0",
        url: "https://github.com/binimum/am-lyrics",
        usage: "Apple Music TTML syllable-level lyrics synchronization specifications and upstream catalog integration.",
        notice: "Copyright (c) Binimum and contributors. Subject to the terms of the Mozilla Public License, v. 2.0.",
    },
    ThirdPartyLicenseItem {
        name: "LRCLIB",
        version_or_scope: "REST API v1",
        license: "MIT",
        url: "https://lrclib.net",
        usage: "Community-driven synchronized LRC and plain-text lyrics database.",
        notice: "Copyright (c) Tran Xuan Thanh (tranxuanthang) and LRCLIB contributors.",
    },
    ThirdPartyLicenseItem {
        name: "Symphonia",
        version_or_scope: "0.5.5",
        license: "MPL-2.0",
        url: "https://github.com/pdeljanov/Symphonia",
        usage: "Pure Rust multimedia container demuxing and audio decoding (FLAC, MP3, WAV, AAC/M4A, OGG, Vorbis).",
        notice: "Copyright (c) Philip Deljanov and Symphonia contributors. Licensed under the Mozilla Public License, v. 2.0.",
    },
    ThirdPartyLicenseItem {
        name: "CPAL (Cross-Platform Audio Library)",
        version_or_scope: "0.15.x",
        license: "Apache-2.0",
        url: "https://github.com/RustAudio/cpal",
        usage: "Low-latency native hardware audio output streaming on ALSA, PulseAudio/PipeWire, WASAPI, and AAudio.",
        notice: "Copyright (c) RustAudio Developers. Licensed under the Apache License, Version 2.0.",
    },
    ThirdPartyLicenseItem {
        name: "RealFFT & RustFFT",
        version_or_scope: "3.5.x / 6.x",
        license: "MIT OR Apache-2.0",
        url: "https://github.com/HEnquist/realfft",
        usage: "Fast Fourier Transform spectral frame extraction for 12-bin CQT chroma, f0 pitch contour, and 4-band energy profiling.",
        notice: "Copyright (c) Henrik Enquist and Elliot Mahler. Licensed under MIT or Apache-2.0.",
    },
    ThirdPartyLicenseItem {
        name: "Rubato",
        version_or_scope: "0.16.x",
        license: "MIT",
        url: "https://github.com/HEnquist/rubato",
        usage: "Asynchronous sample rate conversion and polyphase sinc resampling.",
        notice: "Copyright (c) Henrik Enquist. Licensed under the MIT License.",
    },
    ThirdPartyLicenseItem {
        name: "Signalsmith Stretch",
        version_or_scope: "C++ DSP Reference",
        license: "MIT",
        url: "https://github.com/Signalsmith-Audio/signalsmith-stretch",
        usage: "Phase-vocoder time-stretching and formant-preserving pitch shifting foundations.",
        notice: "Copyright (c) Signalsmith Audio Ltd. Licensed under the MIT License.",
    },
    ThirdPartyLicenseItem {
        name: "Hound",
        version_or_scope: "3.5.x",
        license: "Apache-2.0 OR MIT",
        url: "https://github.com/ruuda/hound",
        usage: "WAV audio encoding and decoding for headless transition rendering.",
        notice: "Copyright (c) Ruud van Asseldonk. Licensed under Apache-2.0 or MIT.",
    },
    ThirdPartyLicenseItem {
        name: "Rusqlite & SQLite",
        version_or_scope: "0.34.x (bundled SQLite 3.47+)",
        license: "MIT / Public Domain",
        url: "https://github.com/rusqlite/rusqlite",
        usage: "Embedded SQLite database engine for portable .tap and .ptr audio profile caching.",
        notice: "Copyright (c) The Rusqlite Developers (MIT). SQLite is in the Public Domain.",
    },
    ThirdPartyLicenseItem {
        name: "Tokio & Rayon",
        version_or_scope: "1.x",
        license: "MIT OR Apache-2.0",
        url: "https://github.com/tokio-rs/tokio",
        usage: "Asynchronous runtime scheduler and data-parallel multi-core audio profiling work-stealing thread pool.",
        notice: "Copyright (c) Tokio Contributors and Josh Stone / Niko Matsakis. Licensed under MIT or Apache-2.0.",
    },
    ThirdPartyLicenseItem {
        name: "Serde & Serde JSON",
        version_or_scope: "1.0.x",
        license: "MIT OR Apache-2.0",
        url: "https://github.com/serde-rs/serde",
        usage: "Zero-copy serialization and deserialization for configuration, cache packs, and REST APIs.",
        notice: "Copyright (c) Erick Tryzelaar and David Tolnay. Licensed under MIT or Apache-2.0.",
    },
    ThirdPartyLicenseItem {
        name: "BLAKE3 & SHA-2 (RustCrypto)",
        version_or_scope: "1.8.x / 0.10.x",
        license: "CC0-1.0 OR Apache-2.0 OR MIT",
        url: "https://github.com/BLAKE3-team/BLAKE3",
        usage: "Cryptographic acoustic content hashing and anti-tamper dynamic library (.dll/.so) integrity verification.",
        notice: "Copyright (c) Jack O'Connor, Samuel Neves, Jean-Philippe Aumasson, Zooko Wilcox-O'Hearn, and RustCrypto Developers.",
    },
    ThirdPartyLicenseItem {
        name: "id3 & metaflac",
        version_or_scope: "1.16.x / 0.2.x",
        license: "MIT",
        url: "https://github.com/polyfloyd/rust-id3",
        usage: "Non-destructive reading and writing of MP3 ID3v2.4 frames (APIC, USLT, TXXX) and FLAC Vorbis / PICTURE blocks.",
        notice: "Copyright (c) Polyfloyd and James Hurst. Licensed under the MIT License.",
    },
    ThirdPartyLicenseItem {
        name: "Ureq",
        version_or_scope: "2.12.x",
        license: "MIT OR Apache-2.0",
        url: "https://github.com/algesten/ureq",
        usage: "Minimal synchronous HTTP/TLS client for lyrics, MusicBrainz, Wikipedia, Innertube, and GitHub release checks.",
        notice: "Copyright (c) Martin Algesten and ureq contributors. Licensed under MIT or Apache-2.0.",
    },
    ThirdPartyLicenseItem {
        name: "ZBus (Linux MPRIS v2)",
        version_or_scope: "4.x",
        license: "MIT",
        url: "https://github.com/dbus2/zbus",
        usage: "Pure Rust D-Bus IPC engine powering desktop media keys, timeline scrubbing, and lockscreen controls.",
        notice: "Copyright (c) Zeeshan Ali Khan and zbus contributors. Licensed under the MIT License.",
    },
    ThirdPartyLicenseItem {
        name: "RFD (Rusty File Dialog) & Image",
        version_or_scope: "0.15.x / 0.25.x",
        license: "MIT OR Apache-2.0",
        url: "https://github.com/PolyMeilex/rfd",
        usage: "Native OS folder/file pickers and raster artwork decoding/resizing.",
        notice: "Copyright (c) PolyMeilex and Piston Developers. Licensed under MIT or Apache-2.0.",
    },
];

// ============================================================================
// 4. ABOUT ATTUNE COMPONENT
// ============================================================================

#[component]
pub fn About() -> Element {
    let strings = crate::i18n::current_strings();
    let pkg_version = crate::config::app_version();
    let git_commit = option_env!("ATTUNE_GIT_COMMIT").unwrap_or("dev");
    let target_os = if cfg!(target_os = "windows") {
        "Windows (WebView2)"
    } else {
        "Linux (WebKitGTK)"
    };

    // Update checker state
    let mut is_checking_update = use_signal(|| false);
    let mut update_result = use_signal(|| None::<UpdateCheckResult>);

    // Contributors & AI Assistants sort/filter state
    let mut contrib_sort = use_signal(|| ContributorSortOrder::DefaultRole);
    let mut contrib_filter = use_signal(|| ContributorFilter::All);

    // Third-Party Licenses modal state
    let mut show_licenses_modal = use_signal(|| false);
    let mut license_search = use_signal(String::new);
    let mut expanded_license_idx = use_signal(|| None::<usize>);

    let visible_contributors =
        sorted_and_filtered_contributors(*contrib_sort.read(), *contrib_filter.read());

    let query_lower = license_search.read().trim().to_ascii_lowercase();
    let filtered_licenses: Vec<(usize, &ThirdPartyLicenseItem)> = THIRD_PARTY_LICENSES
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            if query_lower.is_empty() {
                true
            } else {
                item.name.to_ascii_lowercase().contains(&query_lower)
                    || item.license.to_ascii_lowercase().contains(&query_lower)
                    || item.usage.to_ascii_lowercase().contains(&query_lower)
            }
        })
        .collect();

    rsx! {
        div { class: "about-stage",
            style: "max-width: 760px; margin: 0 auto; padding: 10px 0 42px 0;",

            // =================================================================
            // 1. HERO HEADER BANNER
            // =================================================================
            div { style: "display: flex; align-items: center; gap: 20px; background: var(--bg-card); padding: 24px; border-radius: 18px; border: 1px solid var(--border-glass); margin-bottom: 20px;",
                img {
                    src: "{crate::app_icon::app_icon_tile_data_url()}",
                    style: "width: 72px; height: 72px; border-radius: 16px; object-fit: cover; flex-shrink: 0; box-shadow: 0 8px 24px rgba(0,0,0,0.4), 0 0 20px rgba(45,212,191,0.2);",
                    alt: "Attune Official Logo",
                }
                div { style: "flex: 1;",
                    h1 { style: "font-size: 26px; font-weight: 700; margin: 0;", "{strings.about_attune}" }
                    p { style: "color: var(--accent-mint); font-weight: 600; font-size: 14px; margin: 3px 0 0 0;",
                        "Version {pkg_version} • {target_os}"
                    }
                    p { style: "color: var(--text-secondary); font-size: 13px; margin: 5px 0 10px 0;",
                        "Intelligent DJ-grade music player with audio-profile-driven transition & remixing engine"
                    }
                    div { style: "display: flex; flex-wrap: wrap; gap: 8px;",
                        span {
                            style: "display: inline-flex; align-items: center; gap: 5px; padding: 3px 10px; border-radius: 10px; font-size: 11px; font-weight: 700; background: rgba(45,212,191,0.14); color: var(--accent-mint); border: 1px solid rgba(45,212,191,0.35);",
                            span { class: "material-icons md-14", "sell" }
                            "v{pkg_version}"
                        }
                        span {
                            style: "display: inline-flex; align-items: center; gap: 5px; padding: 3px 10px; border-radius: 10px; font-size: 11px; font-weight: 700; background: rgba(168,85,247,0.14); color: #c084fc; border: 1px solid rgba(168,85,247,0.35);",
                            span { class: "material-icons md-14", "memory" }
                            "Native Rust Core"
                        }
                        span {
                            style: "display: inline-flex; align-items: center; gap: 5px; padding: 3px 10px; border-radius: 10px; font-size: 11px; font-weight: 700; background: rgba(255,255,255,0.06); color: var(--text-secondary); border: 1px solid var(--border-glass); font-family: monospace;",
                            span { class: "material-icons md-14", "commit" }
                            "Git #{git_commit}"
                        }
                    }
                }
            }

            // =================================================================
            // 2. GITHUB RELEASES & IN-APP UPDATE CHECKER
            // =================================================================
            div { style: "background: var(--bg-card); padding: 22px 24px; border-radius: 16px; border: 1px solid var(--border-glass); margin-bottom: 20px;",
                div { style: "display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap;",
                    div { style: "display: flex; align-items: center; gap: 12px;",
                        div {
                            style: "width: 40px; height: 40px; border-radius: 50%; background: rgba(45,212,191,0.15); border: 1px solid rgba(45,212,191,0.35); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                            span { class: "material-icons md-20", style: "color: var(--accent-mint);", "cloud_sync" }
                        }
                        div {
                            h2 { style: "font-size: 16px; font-weight: 700; margin: 0;", "App Updates & GitHub Releases" }
                            p { style: "color: var(--text-secondary); font-size: 12.5px; margin: 2px 0 0 0;",
                                "Currently running v{pkg_version} • Query GitHub Releases for newer desktop & mobile packages"
                            }
                        }
                    }
                    button {
                        class: "btn-pill active",
                        style: "display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; font-size: 12.5px; font-weight: 600;",
                        disabled: *is_checking_update.read(),
                        onclick: move |_| {
                            let ver = pkg_version.to_string();
                            is_checking_update.set(true);
                            dioxus::prelude::spawn(async move {
                                let res = tokio::task::spawn_blocking(move || check_for_updates(&ver))
                                    .await
                                    .unwrap_or_else(|e| UpdateCheckResult {
                                        status: UpdateStatus::Error,
                                        update_info: None,
                                        message: format!("Update check task failed: {e}"),
                                    });
                                is_checking_update.set(false);
                                update_result.set(Some(res));
                            });
                        },
                        if *is_checking_update.read() {
                            span { class: "material-icons md-16 spinning", "sync" }
                            "Checking..."
                        } else {
                            span { class: "material-icons md-16", "refresh" }
                            "Check for Updates"
                        }
                    }
                }

                if let Some(res) = update_result.read().as_ref() {
                    div { style: "margin-top: 16px; padding-top: 14px; border-top: 1px solid rgba(255,255,255,0.08);",
                        if res.status == UpdateStatus::UpdateAvailable {
                            if let Some(info) = res.update_info.as_ref() {
                                div { style: "background: rgba(45,212,191,0.10); border: 1px solid rgba(45,212,191,0.35); border-radius: 12px; padding: 14px 16px;",
                                    div { style: "display: flex; align-items: center; justify-content: space-between; gap: 10px; flex-wrap: wrap;",
                                        div { style: "display: flex; align-items: center; gap: 8px; color: var(--accent-mint); font-weight: 700; font-size: 14px;",
                                            span { class: "material-icons md-18", "new_releases" }
                                            "New Release Available: {info.tag_name}"
                                        }
                                        a {
                                            class: "btn-pill",
                                            style: "font-size: 11.5px;",
                                            href: "{info.html_url}",
                                            target: "_blank",
                                            "Open Release on GitHub ↗"
                                        }
                                    }
                                    if !info.release_notes.is_empty() {
                                        pre {
                                            style: "margin: 10px 0 0 0; font-family: inherit; font-size: 12px; color: var(--text-secondary); white-space: pre-wrap; max-height: 120px; overflow-y: auto; background: rgba(0,0,0,0.25); padding: 10px 12px; border-radius: 8px;",
                                            "{info.release_notes}"
                                        }
                                    }
                                    {
                                        let host_assets = info.host_platform_assets();
                                        rsx! {
                                            if !host_assets.is_empty() {
                                                div { style: "margin-top: 12px;",
                                                    div { style: "font-size: 11.5px; color: var(--text-muted); margin-bottom: 6px; font-weight: 600;",
                                                        "Available Downloads for your OS (Release & Debug for Contributors):"
                                                    }
                                                    div { style: "display: flex; flex-wrap: wrap; gap: 8px;",
                                                        for asset in host_assets.iter() {
                                                            {
                                                                let size_str = asset.formatted_size();
                                                                let badge_style = if asset.is_debug {
                                                                    "display: inline-flex; align-items: center; gap: 6px; padding: 6px 12px; border-radius: 8px; font-size: 11.5px; background: rgba(245,158,11,0.12); color: #fbbf24; border: 1px solid rgba(245,158,11,0.35); text-decoration: none;"
                                                                } else {
                                                                    "display: inline-flex; align-items: center; gap: 6px; padding: 6px 12px; border-radius: 8px; font-size: 11.5px; background: rgba(45,212,191,0.12); color: #fff; border: 1px solid rgba(45,212,191,0.35); text-decoration: none;"
                                                                };
                                                                rsx! {
                                                                    a {
                                                                        key: "{asset.name}",
                                                                        style: "{badge_style}",
                                                                        href: "{asset.download_url}",
                                                                        target: "_blank",
                                                                        span {
                                                                            class: "material-icons md-14",
                                                                            if asset.is_debug { "bug_report" } else { "download" }
                                                                        }
                                                                        if size_str.is_empty() {
                                                                            "{asset.name}"
                                                                        } else {
                                                                            "{asset.name} ({size_str})"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else if res.status == UpdateStatus::UpToDate {
                            div { style: "display: flex; align-items: center; gap: 8px; color: var(--accent-mint); font-size: 13px; font-weight: 600;",
                                span { class: "material-icons md-18", "check_circle" }
                                "{res.message}"
                            }
                        } else if res.status == UpdateStatus::NoReleasesFound {
                            div { style: "display: flex; align-items: center; gap: 8px; color: var(--text-secondary); font-size: 13px;",
                                span { class: "material-icons md-18", "info_outline" }
                                "{res.message}"
                            }
                        } else if res.status == UpdateStatus::Error {
                            div { style: "display: flex; align-items: center; gap: 8px; color: #fb7185; font-size: 13px;",
                                span { class: "material-icons md-18", "error_outline" }
                                "{res.message}"
                            }
                        }
                    }
                }
            }

            // =================================================================
            // 3. CONTRIBUTORS & AI ASSISTANTS INVOLVEMENT (SORTABLE)
            // =================================================================
            div { style: "background: var(--bg-card); padding: 24px; border-radius: 16px; border: 1px solid var(--border-glass); margin-bottom: 20px;",
                div { style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 14px; flex-wrap: wrap; margin-bottom: 14px;",
                    div {
                        h2 { style: "font-size: 18px; font-weight: 700; margin: 0;", "Contributors & AI Assistants" }
                        p { style: "color: var(--text-secondary); font-size: 12.5px; margin: 4px 0 0 0;",
                            "Project authors, contributors, and AI pair-programming assistants involved in building Attune."
                        }
                    }

                    // Sort & Filter Controls
                    div { style: "display: flex; gap: 8px; align-items: center; flex-wrap: wrap;",
                        div { style: "display: flex; gap: 4px; background: rgba(255,255,255,0.04); padding: 3px; border-radius: 10px; border: 1px solid var(--border-glass);",
                            button {
                                class: if *contrib_filter.read() == ContributorFilter::All { "btn-pill active" } else { "btn-pill" },
                                style: "font-size: 11px; padding: 4px 10px; border: none;",
                                onclick: move |_| contrib_filter.set(ContributorFilter::All),
                                "All (3)"
                            }
                            button {
                                class: if *contrib_filter.read() == ContributorFilter::ContributorsOnly { "btn-pill active" } else { "btn-pill" },
                                style: "font-size: 11px; padding: 4px 10px; border: none;",
                                onclick: move |_| contrib_filter.set(ContributorFilter::ContributorsOnly),
                                "Contributors"
                            }
                            button {
                                class: if *contrib_filter.read() == ContributorFilter::AiAssistantsOnly { "btn-pill active" } else { "btn-pill" },
                                style: "font-size: 11px; padding: 4px 10px; border: none;",
                                onclick: move |_| contrib_filter.set(ContributorFilter::AiAssistantsOnly),
                                "AI Assistants"
                            }
                        }

                        select {
                            class: "form-select",
                            style: "font-size: 11.5px; padding: 5px 10px; border-radius: 10px; width: auto;",
                            onchange: move |e| {
                                let order = match e.value().as_str() {
                                    "name_asc" => ContributorSortOrder::NameAsc,
                                    "name_desc" => ContributorSortOrder::NameDesc,
                                    "human_first" => ContributorSortOrder::ContributorsFirst,
                                    "ai_first" => ContributorSortOrder::AiAssistantsFirst,
                                    _ => ContributorSortOrder::DefaultRole,
                                };
                                contrib_sort.set(order);
                            },
                            option { value: "default", "Sort: Role / Default" }
                            option { value: "name_asc", "Sort: Name (A → Z)" }
                            option { value: "name_desc", "Sort: Name (Z → A)" }
                            option { value: "human_first", "Sort: Contributors First" }
                            option { value: "ai_first", "Sort: AI Assistants First" }
                        }
                    }
                }

                div { style: "display: flex; flex-direction: column; gap: 10px;",
                    for entry in visible_contributors.iter() {
                        {
                            let is_ai = entry.kind == ContributorKind::AiAssistant;
                            let badge_style = if is_ai {
                                "padding: 2px 8px; border-radius: 8px; font-size: 10.5px; font-weight: 700; background: rgba(168,85,247,0.16); color: #c084fc; border: 1px solid rgba(168,85,247,0.35);"
                            } else {
                                "padding: 2px 8px; border-radius: 8px; font-size: 10.5px; font-weight: 700; background: rgba(45,212,191,0.16); color: var(--accent-mint); border: 1px solid rgba(45,212,191,0.35);"
                            };
                            let icon_color = if is_ai { "#c084fc" } else { "var(--accent-mint)" };
                            rsx! {
                                div {
                                    key: "{entry.name}",
                                    style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 12px; padding: 14px 16px; display: flex; justify-content: space-between; align-items: flex-start; gap: 12px;",
                                    div { style: "flex: 1;",
                                        div { style: "display: flex; align-items: center; gap: 8px; flex-wrap: wrap;",
                                            span { class: "material-icons md-18", style: "color: {icon_color};", "{entry.icon}" }
                                            span { style: "font-weight: 700; font-size: 14.5px; color: #fff;", "{entry.name}" }
                                            span { style: "{badge_style}", "{entry.kind.badge_label()}" }
                                        }
                                        div { style: "font-size: 12px; font-weight: 600; color: var(--accent-mint); margin-top: 3px;",
                                            "{entry.role}"
                                        }
                                        p { style: "font-size: 12.5px; color: var(--text-secondary); margin: 6px 0 0 0; line-height: 1.45;",
                                            "{entry.description}"
                                        }
                                    }
                                    if let Some(link) = entry.url {
                                        a {
                                            class: "btn-pill",
                                            style: "font-size: 11.5px; flex-shrink: 0;",
                                            href: "{link}",
                                            target: "_blank",
                                            "GitHub Profile ↗"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Knowledge Ledger Protocol Disclosure (matching Flutter AboutStage)
                div {
                    style: "margin-top: 12px; padding: 12px 16px; border-radius: 10px; background: rgba(0,0,0,0.22); border: 1px solid rgba(255,255,255,0.06); font-size: 12px; color: var(--text-secondary); line-height: 1.5;",
                    div { style: "font-weight: 700; color: var(--text-primary); margin-bottom: 4px;",
                        "Knowledge Ledger Protocol & Pair Programming Disclosure"
                    }
                    "• Knowledge Ledgering: Every architectural decision is recorded in Architecture Decision Records (ledger/01_ADR_LOG.md)."
                    br {}
                    "• Zero Hardcoding Mandate: Algorithmic parameters, DSP lookahead gates, and hardware bounds derive from real audio analysis and live OS measurements rather than arbitrary constant tables."
                    br {}
                    "• Empirical Transition Lab: Benchmark song pairs and tuning observations are logged continuously in ledger/04_TRANSITION_LAB_LOG.md."
                }
            }

            // =================================================================
            // 4. SUPPORT & DONATIONS CARD (EXACT COPY FROM FLUTTER VARIANT)
            // =================================================================
            div { style: "background: var(--bg-card); padding: 22px 24px; border-radius: 16px; border: 1px solid var(--border-glass); margin-bottom: 20px;",
                div { style: "font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--accent-mint); margin-bottom: 10px;",
                    "SUPPORT & DONATIONS"
                }
                div { style: "display: flex; align-items: flex-start; gap: 14px;",
                    div {
                        style: "width: 42px; height: 42px; border-radius: 50%; background: rgba(244, 63, 94, 0.18); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                        span { class: "material-icons md-22", style: "color: #fb7185;", "favorite" }
                    }
                    div { style: "flex: 1;",
                        div { style: "display: flex; align-items: center; gap: 8px; flex-wrap: wrap;",
                            span { style: "font-size: 15px; font-weight: 700; color: #fff;", "Community Sponsorship" }
                            span {
                                style: "padding: 2px 8px; border-radius: 10px; font-size: 9.5px; font-weight: 700; background: rgba(245, 158, 11, 0.2); color: #fcd34d; border: 1px solid rgba(245, 158, 11, 0.5);",
                                "COMING SOON"
                            }
                        }
                        p { style: "color: var(--text-secondary); font-size: 13px; margin: 6px 0 0 0; line-height: 1.5;",
                            "Attune is free and open-source software. Community sponsorship avenues (GitHub Sponsors, Ko-fi, and Liberapay) are coming soon to support further development of the project. Thank you for your support!"
                        }
                    }
                }
            }

            // =================================================================
            // 5. SPECIAL THANKS & OPEN SOURCE CREDITS (INCLUDING DIOXUS)
            // =================================================================
            div { style: "background: var(--bg-card); padding: 24px; border-radius: 16px; border: 1px solid var(--border-glass); margin-bottom: 20px;",
                h2 { style: "font-size: 18px; font-weight: 700; margin-bottom: 12px;", "{strings.special_thanks_and_credits}" }
                p { style: "color: var(--text-secondary); font-size: 13px; margin-bottom: 16px; line-height: 1.5;",
                    "Attune is crafted on the shoulders of remarkable open-source projects, community innovators, and audio pioneers:"
                }
                div { style: "display: flex; flex-direction: column; gap: 10px;",
                    // Dioxus
                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 10px; padding: 12px 16px; display: flex; justify-content: space-between; align-items: center;",
                        div {
                            div { style: "font-weight: 600; font-size: 14px; color: #fff; display: flex; align-items: center; gap: 6px;",
                                span { class: "material-icons md-16", style: "color: var(--accent-mint);", "widgets" }
                                "Dioxus"
                            }
                            div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                "Reactive cross-platform Rust UI framework powering Attune Desktop with native WebKitGTK and WebView2 rendering."
                            }
                        }
                        a {
                            class: "btn-pill",
                            style: "font-size: 11.5px; flex-shrink: 0; margin-left: 12px;",
                            href: "https://github.com/DioxusLabs/dioxus",
                            target: "_blank",
                            "DioxusLabs/dioxus ↗"
                        }
                    }

                    // Better Lyrics
                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 10px; padding: 12px 16px; display: flex; justify-content: space-between; align-items: center;",
                        div {
                            div { style: "font-weight: 600; font-size: 14px; color: #fff; display: flex; align-items: center; gap: 6px;",
                                span { class: "material-icons md-16", style: "color: var(--accent-mint);", "stars" }
                                "Better Lyrics"
                            }
                            div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                "Lyrics animation techniques, fluid cubic-bezier easing curves, and custom CSS theme architecture."
                            }
                        }
                        a {
                            class: "btn-pill",
                            style: "font-size: 11.5px; flex-shrink: 0; margin-left: 12px;",
                            href: "https://github.com/better-lyrics/better-lyrics",
                            target: "_blank",
                            "better-lyrics/better-lyrics ↗"
                        }
                    }

                    // BiniLyrics (<am-lyrics>)
                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 10px; padding: 12px 16px; display: flex; justify-content: space-between; align-items: center;",
                        div {
                            div { style: "font-weight: 600; font-size: 14px; color: #fff; display: flex; align-items: center; gap: 6px;",
                                span { class: "material-icons md-16", style: "color: var(--accent-mint);", "subtitles" }
                                "BiniLyrics (<am-lyrics>)"
                            }
                            div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                "Apple Music TTML syllable parser & rich sync components under MPL-2.0."
                            }
                        }
                        a {
                            class: "btn-pill",
                            style: "font-size: 11.5px; flex-shrink: 0; margin-left: 12px;",
                            href: "https://github.com/binimum/am-lyrics",
                            target: "_blank",
                            "am-lyrics ↗"
                        }
                    }

                    // LRCLIB
                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 10px; padding: 12px 16px; display: flex; justify-content: space-between; align-items: center;",
                        div {
                            div { style: "font-weight: 600; font-size: 14px; color: #fff; display: flex; align-items: center; gap: 6px;",
                                span { class: "material-icons md-16", style: "color: var(--accent-mint);", "cloud_queue" }
                                "LRCLIB"
                            }
                            div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                "Community-driven synchronized lyrics database and open REST API."
                            }
                        }
                        a {
                            class: "btn-pill",
                            style: "font-size: 11.5px; flex-shrink: 0; margin-left: 12px;",
                            href: "https://lrclib.net",
                            target: "_blank",
                            "lrclib.net ↗"
                        }
                    }

                    // Open-Source DSP & Audio Foundations
                    div { style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 10px; padding: 12px 16px; display: flex; justify-content: space-between; align-items: center;",
                        div {
                            div { style: "font-weight: 600; font-size: 14px; color: #fff; display: flex; align-items: center; gap: 6px;",
                                span { class: "material-icons md-16", style: "color: var(--accent-mint);", "graphic_eq" }
                                "Signalsmith, Symphonia, CPAL, RealFFT & Rubato"
                            }
                            div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 2px;",
                                "Pure Rust multimedia decoding, low-latency hardware audio I/O, FFT spectral analysis, and fractional resampling."
                            }
                        }
                        button {
                            class: "btn-pill",
                            style: "font-size: 11.5px; flex-shrink: 0; margin-left: 12px;",
                            onclick: move |_| show_licenses_modal.set(true),
                            "View Crates ↗"
                        }
                    }
                }
            }

            // =================================================================
            // 6. LICENSE & THIRD-PARTY LEGAL NOTICES (FLUTTER PARITY)
            // =================================================================
            div { style: "background: var(--bg-card); padding: 24px; border-radius: 16px; border: 1px solid var(--border-glass); margin-bottom: 20px;",
                div { style: "display: flex; align-items: center; gap: 10px; margin-bottom: 10px;",
                    span { class: "material-icons md-20", style: "color: var(--accent-mint);", "gavel" }
                    h2 { style: "font-size: 17px; font-weight: 700; margin: 0;", "MIT License (Attune Player & Core) & Third-Party Notices" }
                }
                div { style: "color: var(--text-secondary); font-size: 12.5px; line-height: 1.55; margin-bottom: 16px;",
                    p { style: "margin: 0 0 8px 0;",
                        "Attune is licensed under the MIT License: Copyright (c) 2026 Mizumo-prjkt and Attune contributors."
                    }
                    p { style: "margin: 0 0 10px 0;",
                        "Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files, to deal in the Software without restriction."
                    }
                    div {
                        style: "padding: 10px 14px; border-radius: 10px; background: rgba(0,0,0,0.22); border: 1px solid rgba(255,255,255,0.06);",
                        div { style: "font-weight: 700; color: var(--text-primary); font-size: 11.5px; text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 4px;",
                            "Note on Third-Party Libraries"
                        }
                        "Attune bundles and links against third-party open-source components and DSP libraries which carry their own respective licenses, including BSD-2-Clause, BSD-3-Clause, Apache-2.0, MIT, MPL-2.0, LGPL, and GPL. All copyrights, patents, and trademarks belong to their respective holders."
                    }
                }
                div { style: "display: flex; gap: 12px; flex-wrap: wrap;",
                    button {
                        class: "btn-pill active",
                        style: "display: inline-flex; align-items: center; gap: 8px; padding: 9px 18px; font-size: 13px; font-weight: 600;",
                        onclick: move |_| show_licenses_modal.set(true),
                        span { class: "material-icons md-18", "description" }
                        "View Complete Open-Source Licenses ({THIRD_PARTY_LICENSES.len()})"
                    }
                    a {
                        class: "btn-pill",
                        style: "display: inline-flex; align-items: center; gap: 6px; padding: 9px 16px; font-size: 13px;",
                        href: "https://github.com/Mizumo-prjkt/attune",
                        target: "_blank",
                        "GitHub Repository ↗"
                    }
                    a {
                        class: "btn-pill",
                        style: "display: inline-flex; align-items: center; gap: 6px; padding: 9px 16px; font-size: 13px;",
                        href: "https://github.com/Mizumo-prjkt/attune/releases",
                        target: "_blank",
                        "Obtainium / Releases ↗"
                    }
                }
            }

            // =================================================================
            // 7. COMPLETE OPEN-SOURCE THIRD-PARTY LICENSES MODAL
            // =================================================================
            if *show_licenses_modal.read() {
                div {
                    style: "position: fixed; inset: 0; z-index: 9500; background: rgba(6, 9, 15, 0.82); backdrop-filter: blur(18px); display: flex; align-items: center; justify-content: center; padding: 24px;",
                    onclick: move |_| show_licenses_modal.set(false),

                    div {
                        style: "background: #101622; border: 1px solid var(--border-glass); border-radius: 22px; width: 100%; max-width: 780px; max-height: 84vh; display: flex; flex-direction: column; box-shadow: 0 28px 64px rgba(0,0,0,0.65); overflow: hidden;",
                        onclick: move |e| e.stop_propagation(),

                        // Modal Header
                        div {
                            style: "padding: 20px 24px; border-bottom: 1px solid var(--border-glass); display: flex; justify-content: space-between; align-items: center; gap: 14px;",
                            div { style: "display: flex; align-items: center; gap: 12px;",
                                img {
                                    src: "{crate::app_icon::app_icon_tile_data_url()}",
                                    style: "width: 38px; height: 38px; border-radius: 10px;",
                                    alt: "Attune",
                                }
                                div {
                                    h3 { style: "margin: 0; font-size: 17px; font-weight: 700;", "Third-Party Open-Source Licenses" }
                                    p { style: "margin: 2px 0 0 0; font-size: 12px; color: var(--text-secondary);",
                                        "Attune v{pkg_version} • Distributed under the MIT License • {THIRD_PARTY_LICENSES.len()} Bundled Components"
                                    }
                                }
                            }
                            button {
                                class: "btn-pill",
                                style: "padding: 6px 12px; font-size: 12px;",
                                onclick: move |_| show_licenses_modal.set(false),
                                "Close"
                            }
                        }

                        // Filter Bar
                        div {
                            style: "padding: 12px 24px; border-bottom: 1px solid rgba(255,255,255,0.06); background: rgba(0,0,0,0.2);",
                            input {
                                class: "search-bar",
                                style: "width: 100%; padding: 9px 14px; border-radius: 10px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-glass); color: #fff; font-size: 13px; box-sizing: border-box;",
                                placeholder: "Filter components by crate name, license (MIT, MPL-2.0, Apache-2.0), or role...",
                                value: "{license_search.read()}",
                                oninput: move |e| license_search.set(e.value()),
                            }
                        }

                        // Scrollable Component License List
                        div {
                            style: "padding: 16px 24px; overflow-y: auto; display: flex; flex-direction: column; gap: 10px; flex: 1;",
                            for (idx, item) in filtered_licenses.iter() {
                                {
                                    let is_open = *expanded_license_idx.read() == Some(*idx);
                                    let row_idx = *idx;
                                    rsx! {
                                        div {
                                            key: "{item.name}",
                                            style: "background: rgba(255,255,255,0.03); border: 1px solid var(--border-glass); border-radius: 12px; padding: 12px 16px;",
                                            div {
                                                style: "display: flex; justify-content: space-between; align-items: center; gap: 12px; cursor: pointer;",
                                                onclick: move |_| {
                                                    if *expanded_license_idx.read() == Some(row_idx) {
                                                        expanded_license_idx.set(None);
                                                    } else {
                                                        expanded_license_idx.set(Some(row_idx));
                                                    }
                                                },
                                                div { style: "flex: 1;",
                                                    div { style: "display: flex; align-items: center; gap: 8px; flex-wrap: wrap;",
                                                        span { style: "font-weight: 700; font-size: 14px; color: #fff;", "{item.name}" }
                                                        span {
                                                            style: "padding: 2px 8px; border-radius: 6px; font-size: 10.5px; font-weight: 700; font-family: monospace; background: rgba(45,212,191,0.14); color: var(--accent-mint); border: 1px solid rgba(45,212,191,0.3);",
                                                            "{item.license}"
                                                        }
                                                        span {
                                                            style: "font-size: 11px; color: var(--text-muted); font-family: monospace;",
                                                            "{item.version_or_scope}"
                                                        }
                                                    }
                                                    div { style: "font-size: 12px; color: var(--text-secondary); margin-top: 4px;",
                                                        "{item.usage}"
                                                    }
                                                }
                                                span {
                                                    class: "material-icons md-18",
                                                    style: "color: var(--text-muted);",
                                                    if is_open { "expand_less" } else { "expand_more" }
                                                }
                                            }

                                            if is_open {
                                                div {
                                                    style: "margin-top: 10px; padding-top: 10px; border-top: 1px solid rgba(255,255,255,0.06); font-size: 12px; color: var(--text-secondary);",
                                                    div {
                                                        style: "font-family: monospace; background: rgba(0,0,0,0.30); padding: 10px 12px; border-radius: 8px; line-height: 1.45; margin-bottom: 8px;",
                                                        "{item.notice}"
                                                    }
                                                    a {
                                                        class: "btn-pill",
                                                        style: "font-size: 11px; display: inline-flex; align-items: center; gap: 4px;",
                                                        href: "{item.url}",
                                                        target: "_blank",
                                                        "Open Upstream Repository ({item.url}) ↗"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credits_contain_better_lyrics_and_dioxus() {
        let has_better_lyrics = THIRD_PARTY_LICENSES
            .iter()
            .any(|i| i.url.contains("better-lyrics/better-lyrics"));
        let has_dioxus = THIRD_PARTY_LICENSES
            .iter()
            .any(|i| i.url.contains("DioxusLabs/dioxus"));
        assert!(has_better_lyrics);
        assert!(has_dioxus);
    }

    #[test]
    fn test_semver_comparison_handles_prereleases_and_stable() {
        assert_eq!(compare_semantic_versions("0.1.0-rc3", "0.1.0-rc4"), Ordering::Less);
        assert_eq!(compare_semantic_versions("0.1.0-rc4", "0.1.0-rc10"), Ordering::Less);
        assert_eq!(compare_semantic_versions("0.1.0-rc4", "0.1.0"), Ordering::Less);
        assert_eq!(compare_semantic_versions("v0.1.1-rc1", "0.1.1-rc1"), Ordering::Equal);
        assert_eq!(compare_semantic_versions("0.1.2", "0.1.1-rc9"), Ordering::Greater);
    }

    #[test]
    fn test_contributors_sorting_and_filtering() {
        let all = sorted_and_filtered_contributors(
            ContributorSortOrder::DefaultRole,
            ContributorFilter::All,
        );
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].name, "Mizumo-prjkt");

        let asc = sorted_and_filtered_contributors(
            ContributorSortOrder::NameAsc,
            ContributorFilter::All,
        );
        assert_eq!(asc[0].name, "Claude (Anthropic)");
        assert_eq!(asc[1].name, "Gemini (Google DeepMind Antigravity)");
        assert_eq!(asc[2].name, "Mizumo-prjkt");

        let ai_only = sorted_and_filtered_contributors(
            ContributorSortOrder::DefaultRole,
            ContributorFilter::AiAssistantsOnly,
        );
        assert_eq!(ai_only.len(), 2);
        assert!(ai_only.iter().all(|c| c.kind == ContributorKind::AiAssistant));
    }
}
