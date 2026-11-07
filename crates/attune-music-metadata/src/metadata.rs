use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey, Tag, Value};
use symphonia::core::probe::Hint;

use crate::error::TrackTweaksError;

/// Comprehensive song metadata and non-destructive track tweaks.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    #[serde(default)]
    pub additional_artists: Vec<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_number: Option<u32>,
    pub disc_total: Option<u32>,
    pub composer: Option<String>,
    #[serde(default)]
    pub writer: Option<String>,
    #[serde(default)]
    pub vocalist: Option<String>,
    #[serde(default)]
    pub producer: Option<String>,
    pub comment: Option<String>,
    pub lyrics: Option<String>,
    pub bpm: Option<f32>,
    pub musical_key: Option<String>,

    // Catalog & Online Identifiers
    pub record_label: Option<String>,
    pub catalog_number: Option<String>,
    pub album_type: Option<String>,
    pub release_date: Option<String>,
    pub language: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub musicbrainz_track_id: Option<String>,
    pub online_query_url: Option<String>,

    // Non-destructive track tweaks stored in song metadata:
    pub replaygain_track_gain_db: Option<f32>,
    pub replaygain_track_peak: Option<f32>,
    pub start_trim_ms: Option<u64>,
    pub end_trim_ms: Option<u64>,
    pub speed_factor: Option<f32>,
}

impl TrackMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets or updates a tag by key name (case-insensitive).
    pub fn set_tag(&mut self, key: &str, value: &str) {
        let key_upper = key.trim().to_uppercase();
        let val_trimmed = value.trim_matches(|c: char| c.is_whitespace() || c == '\0');

        match key_upper.as_str() {
            "TITLE" | "TRACKTITLE" | "TRACK_TITLE" | "INAM" | "TIT2" => {
                self.title = Some(val_trimmed.to_string())
            }
            "ARTIST" | "IART" | "TPE1" => {
                // If contains comma, consider comma as a separation for multiple artists
                if val_trimmed.contains(',') {
                    let parts: Vec<&str> = val_trimmed
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !parts.is_empty() {
                        self.artist = Some(parts[0].to_string());
                        for extra in &parts[1..] {
                            if !self.additional_artists.iter().any(|a| a.eq_ignore_ascii_case(extra)) {
                                self.additional_artists.push(extra.to_string());
                            }
                        }
                    }
                } else {
                    self.artist = Some(val_trimmed.to_string());
                }
            }
            "ARTISTS" | "ADDITIONAL_ARTISTS" | "FEATURED_ARTISTS" | "FEAT" => {
                let parts: Vec<&str> = val_trimmed
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                for part in parts {
                    if !self.additional_artists.iter().any(|a| a.eq_ignore_ascii_case(part)) {
                        self.additional_artists.push(part.to_string());
                    }
                }
            }
            "ALBUM" | "IPRD" | "TALB" => self.album = Some(val_trimmed.to_string()),
            "ALBUM_ARTIST" | "ALBUMARTIST" | "TPE2" => {
                self.album_artist = Some(val_trimmed.to_string())
            }
            "GENRE" | "IGNR" | "TCON" => self.genre = Some(val_trimmed.to_string()),
            "YEAR" | "DATE" | "ICRD" | "TDRC" | "TYER" => {
                if let Ok(y) = val_trimmed.parse::<u32>() {
                    self.year = Some(y);
                } else if val_trimmed.len() >= 4 {
                    if let Ok(y) = val_trimmed[..4].parse::<u32>() {
                        self.year = Some(y);
                    }
                }
            }
            "TRACK" | "TRACKNUMBER" | "TRACK_NUMBER" | "ITRK" | "TRCK" => {
                let parts: Vec<&str> = val_trimmed.split('/').collect();
                if let Ok(n) = parts[0].trim().parse::<u32>() {
                    self.track_number = Some(n);
                }
                if parts.len() > 1 {
                    if let Ok(tot) = parts[1].trim().parse::<u32>() {
                        self.track_total = Some(tot);
                    }
                }
            }
            "DISC" | "DISCNUMBER" | "DISC_NUMBER" | "TPOS" => {
                let parts: Vec<&str> = val_trimmed.split('/').collect();
                if let Ok(n) = parts[0].trim().parse::<u32>() {
                    self.disc_number = Some(n);
                }
                if parts.len() > 1 {
                    if let Ok(tot) = parts[1].trim().parse::<u32>() {
                        self.disc_total = Some(tot);
                    }
                }
            }
            "COMPOSER" | "TCOM" => self.composer = Some(val_trimmed.to_string()),
            "WRITER" | "LYRICIST" | "TEXT" | "AUTHOR" => self.writer = Some(val_trimmed.to_string()),
            "VOCAL" | "VOCALS" | "PERFORMER" | "SINGER" => self.vocalist = Some(val_trimmed.to_string()),
            "PRODUCER" | "ARRANGER" | "MIXER" => self.producer = Some(val_trimmed.to_string()),
            "COMMENT" | "ICMT" | "COMM" => self.comment = Some(val_trimmed.to_string()),
            "LYRICS" | "USLT" => self.lyrics = Some(val_trimmed.to_string()),
            "BPM" | "TBPM" => {
                if let Ok(b) = val_trimmed.replace(',', ".").parse::<f32>() {
                    self.bpm = Some(b);
                }
            }
            "KEY" | "MUSICAL_KEY" | "TKEY" => self.musical_key = Some(val_trimmed.to_string()),
            "LABEL" | "ORGANIZATION" | "PUBLISHER" | "TPUB" => {
                self.record_label = Some(val_trimmed.to_string());
            }
            "CATALOGNUMBER" | "CATALOG_NUMBER" | "BARCODE" | "ISRC" | "TSRC" => {
                self.catalog_number = Some(val_trimmed.to_string());
            }
            "RELEASETYPE" | "ALBUMTYPE" | "MUSICBRAINZ_ALBUMTYPE" => {
                self.album_type = Some(val_trimmed.to_string());
            }
            "RELEASEDATE" | "RELEASE_DATE" => {
                self.release_date = Some(val_trimmed.to_string());
            }
            "LANGUAGE" | "TLAN" => {
                self.language = Some(val_trimmed.to_string());
            }
            "MUSICBRAINZ_ALBUMID" | "MUSICBRAINZ_RELEASEID" => {
                self.musicbrainz_id = Some(val_trimmed.to_string());
            }
            "MUSICBRAINZ_TRACKID" => {
                self.musicbrainz_track_id = Some(val_trimmed.to_string());
            }
            "ONLINE_QUERY_URL" | "WIKI_URL" | "URL" => {
                self.online_query_url = Some(val_trimmed.to_string());
            }
            "REPLAYGAIN_TRACK_GAIN" | "REPLAYGAIN_TRACK_GAIN_DB" => {
                let clean = val_trimmed.replace("dB", "").trim().to_string();
                if let Ok(g) = clean.parse::<f32>() {
                    self.replaygain_track_gain_db = Some(g);
                }
            }
            "REPLAYGAIN_TRACK_PEAK" => {
                if let Ok(p) = val_trimmed.parse::<f32>() {
                    self.replaygain_track_peak = Some(p);
                }
            }
            "START_TRIM_MS" => {
                if let Ok(s) = val_trimmed.parse::<u64>() {
                    self.start_trim_ms = Some(s);
                }
            }
            "END_TRIM_MS" => {
                if let Ok(e) = val_trimmed.parse::<u64>() {
                    self.end_trim_ms = Some(e);
                }
            }
            "SPEED_FACTOR" => {
                if let Ok(sf) = val_trimmed.parse::<f32>() {
                    self.speed_factor = Some(sf);
                }
            }
            _ => {
                log::info!("Unhandled metadata tag key: {}", key);
            }
        }
    }
}

/// Reads track metadata from an audio file using Symphonia container probing.
/// Also reads from sidecar `<path>.tweaks.json` if available to merge non-destructive tweaks.
pub fn read_track_metadata<P: AsRef<Path>>(path: P) -> Result<TrackMetadata, TrackTweaksError> {
    let p = path.as_ref();
    let file = File::open(p)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();
    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();

    let mut meta = TrackMetadata::default();

    if let Ok(mut probed) =
        symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)
    {
        // Check probed metadata (prepended tags like ID3v2)
        if let Some(metadata) = probed.metadata.get() {
            if let Some(rev) = metadata.current() {
                for tag in rev.tags() {
                    apply_symphonia_tag(&mut meta, tag);
                }
            }
        }

        // Check format metadata
        if let Some(rev) = probed.format.metadata().current() {
            for tag in rev.tags() {
                apply_symphonia_tag(&mut meta, tag);
            }
        }
    }

    // Merge companion sidecar if present
    let sidecar_path = get_sidecar_path(p);
    if sidecar_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&sidecar_path) {
            if let Ok(sidecar_meta) = serde_json::from_str::<TrackMetadata>(&content) {
                merge_metadata(&mut meta, sidecar_meta);
            }
        }
    }

    Ok(meta)
}

fn apply_symphonia_tag(meta: &mut TrackMetadata, tag: &Tag) {
    if let Some(std_key) = tag.std_key {
        match std_key {
            StandardTagKey::TrackTitle => {
                if meta.title.is_none() {
                    meta.title = Some(tag_value_to_string(&tag.value));
                }
            }
            StandardTagKey::Artist => {
                if meta.artist.is_none() {
                    meta.artist = Some(tag_value_to_string(&tag.value));
                }
            }
            StandardTagKey::Album => {
                if meta.album.is_none() {
                    meta.album = Some(tag_value_to_string(&tag.value));
                }
            }
            StandardTagKey::AlbumArtist => {
                if meta.album_artist.is_none() {
                    meta.album_artist = Some(tag_value_to_string(&tag.value));
                }
            }
            StandardTagKey::Genre => {
                if meta.genre.is_none() {
                    meta.genre = Some(tag_value_to_string(&tag.value));
                }
            }
            StandardTagKey::Date => {
                if meta.year.is_none() {
                    let s = tag_value_to_string(&tag.value);
                    if let Ok(y) = s.parse::<u32>() {
                        meta.year = Some(y);
                    } else if s.len() >= 4 {
                        if let Ok(y) = s[..4].parse::<u32>() {
                            meta.year = Some(y);
                        }
                    }
                }
            }
            StandardTagKey::TrackNumber => {
                if meta.track_number.is_none() {
                    let s = tag_value_to_string(&tag.value);
                    let parts: Vec<&str> = s.split('/').collect();
                    if let Ok(n) = parts[0].trim().parse::<u32>() {
                        meta.track_number = Some(n);
                    }
                    if parts.len() > 1 {
                        if let Ok(tot) = parts[1].trim().parse::<u32>() {
                            meta.track_total = Some(tot);
                        }
                    }
                }
            }
            StandardTagKey::DiscNumber => {
                if meta.disc_number.is_none() {
                    let s = tag_value_to_string(&tag.value);
                    let parts: Vec<&str> = s.split('/').collect();
                    if let Ok(n) = parts[0].trim().parse::<u32>() {
                        meta.disc_number = Some(n);
                    }
                }
            }
            StandardTagKey::Composer => {
                if meta.composer.is_none() {
                    meta.composer = Some(tag_value_to_string(&tag.value));
                }
            }
            StandardTagKey::Comment => {
                if meta.comment.is_none() {
                    meta.comment = Some(tag_value_to_string(&tag.value));
                }
            }
            StandardTagKey::Lyrics => {
                if meta.lyrics.is_none() {
                    meta.lyrics = Some(tag_value_to_string(&tag.value));
                }
            }
            StandardTagKey::Bpm if meta.bpm.is_none() => {
                let s = tag_value_to_string(&tag.value);
                if let Ok(b) = s.replace(',', ".").parse::<f32>() {
                    meta.bpm = Some(b);
                }
            }
            _ => {}
        }
    } else {
        // Custom or unmapped tag key
        let key = tag.key.as_str();
        let val = tag_value_to_string(&tag.value);
        meta.set_tag(key, &val);
    }
}

fn tag_value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.trim_end_matches('\0').to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Flag => "true".to_string(),
        Value::UnsignedInt(u) => u.to_string(),
        Value::SignedInt(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Binary(bytes) => String::from_utf8_lossy(bytes)
            .trim_end_matches('\0')
            .to_string(),
    }
}

fn merge_metadata(target: &mut TrackMetadata, source: TrackMetadata) {
    if source.title.is_some() {
        target.title = source.title;
    }
    if source.artist.is_some() {
        target.artist = source.artist;
    }
    if source.album.is_some() {
        target.album = source.album;
    }
    if source.album_artist.is_some() {
        target.album_artist = source.album_artist;
    }
    if source.genre.is_some() {
        target.genre = source.genre;
    }
    if source.year.is_some() {
        target.year = source.year;
    }
    if source.track_number.is_some() {
        target.track_number = source.track_number;
    }
    if source.track_total.is_some() {
        target.track_total = source.track_total;
    }
    if source.disc_number.is_some() {
        target.disc_number = source.disc_number;
    }
    if source.disc_total.is_some() {
        target.disc_total = source.disc_total;
    }
    if source.composer.is_some() {
        target.composer = source.composer;
    }
    if source.writer.is_some() {
        target.writer = source.writer;
    }
    if source.vocalist.is_some() {
        target.vocalist = source.vocalist;
    }
    if source.producer.is_some() {
        target.producer = source.producer;
    }
    if source.comment.is_some() {
        target.comment = source.comment;
    }
    if source.lyrics.is_some() {
        target.lyrics = source.lyrics;
    }
    if source.bpm.is_some() {
        target.bpm = source.bpm;
    }
    if source.musical_key.is_some() {
        target.musical_key = source.musical_key;
    }
    if source.replaygain_track_gain_db.is_some() {
        target.replaygain_track_gain_db = source.replaygain_track_gain_db;
    }
    if source.replaygain_track_peak.is_some() {
        target.replaygain_track_peak = source.replaygain_track_peak;
    }
    if source.start_trim_ms.is_some() {
        target.start_trim_ms = source.start_trim_ms;
    }
    if source.end_trim_ms.is_some() {
        target.end_trim_ms = source.end_trim_ms;
    }
    if source.speed_factor.is_some() {
        target.speed_factor = source.speed_factor;
    }
}

pub fn get_sidecar_path(path: &Path) -> PathBuf {
    let mut p = path.to_path_buf();
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    p.set_extension(format!("{}.tweaks.json", ext));
    p
}

/// Writes song metadata to an audio file WITHOUT TOUCHING or re-encoding the audio stream.
/// Supports FLAC (Vorbis comment block), MP3 (ID3v2.4 tag), WAV (RIFF INFO chunk),
/// and companion `.tweaks.json` sidecar for any format.
pub fn write_track_metadata<P: AsRef<Path>>(
    path: P,
    metadata: &TrackMetadata,
) -> Result<(), TrackTweaksError> {
    let p = path.as_ref();
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "flac" => write_flac_metadata(p, metadata),
        "mp3" => write_mp3_metadata(p, metadata),
        "wav" => write_wav_metadata(p, metadata),
        _ => {
            // Write companion sidecar file so metadata tweaks are stored losslessly without touching file
            let sidecar_path = get_sidecar_path(p);
            let json = serde_json::to_string_pretty(metadata)?;
            std::fs::write(sidecar_path, json)?;
            Ok(())
        }
    }
}

// =========================================================================
// FLAC Metadata Writer (Preserves Audio Frames 100% Bit-for-Bit)
// =========================================================================

fn write_flac_metadata(path: &Path, meta: &TrackMetadata) -> Result<(), TrackTweaksError> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    if data.len() < 4 || &data[0..4] != b"fLaC" {
        return Err(TrackTweaksError::Metadata(
            "Invalid FLAC magic marker".to_string(),
        ));
    }

    let mut cursor = 4;
    let mut blocks: Vec<(bool, u8, Vec<u8>)> = Vec::new();
    let mut audio_start = 0;

    while cursor + 4 <= data.len() {
        let header_byte = data[cursor];
        let is_last = (header_byte & 0x80) != 0;
        let block_type = header_byte & 0x7F;
        let length = ((data[cursor + 1] as usize) << 16)
            | ((data[cursor + 2] as usize) << 8)
            | (data[cursor + 3] as usize);

        cursor += 4;
        if cursor + length > data.len() {
            return Err(TrackTweaksError::Metadata(
                "Truncated FLAC metadata block".to_string(),
            ));
        }

        let block_data = data[cursor..cursor + length].to_vec();
        cursor += length;

        // Skip existing VORBIS_COMMENT block (block_type 4)
        if block_type != 4 {
            blocks.push((is_last, block_type, block_data));
        }

        if is_last {
            audio_start = cursor;
            break;
        }
    }

    if audio_start == 0 || audio_start > data.len() {
        audio_start = cursor;
    }

    // Build updated Vorbis Comment block payload
    let vorbis_block = build_vorbis_comment_block(meta);
    // Insert Vorbis Comment block right after STREAMINFO (index 1 if STREAMINFO exists)
    let insert_pos = if !blocks.is_empty() && blocks[0].1 == 0 {
        1
    } else {
        0
    };
    blocks.insert(insert_pos, (false, 4, vorbis_block));

    // Reconstruct FLAC file: "fLaC" + updated blocks + original audio stream
    let mut out = Vec::with_capacity(data.len() + 1024);
    out.extend_from_slice(b"fLaC");

    let num_blocks = blocks.len();
    for (idx, (_, block_type, block_data)) in blocks.into_iter().enumerate() {
        let is_last_block = idx == num_blocks - 1;
        let header_byte = (if is_last_block { 0x80 } else { 0x00 }) | (block_type & 0x7F);
        out.push(header_byte);
        let len = block_data.len();
        out.push(((len >> 16) & 0xFF) as u8);
        out.push(((len >> 8) & 0xFF) as u8);
        out.push((len & 0xFF) as u8);
        out.extend_from_slice(&block_data);
    }

    // Append untouched audio frames bit-for-bit
    out.extend_from_slice(&data[audio_start..]);

    // Atomic write
    atomic_write(path, &out)
}

fn build_vorbis_comment_block(meta: &TrackMetadata) -> Vec<u8> {
    let mut out = Vec::new();
    let vendor = "Attune Music Metadata Changer";
    out.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    out.extend_from_slice(vendor.as_bytes());

    let mut comments: Vec<String> = Vec::new();
    if let Some(ref t) = meta.title {
        comments.push(format!("TITLE={}", t));
    }
    if let Some(ref a) = meta.artist {
        comments.push(format!("ARTIST={}", a));
    }
    if !meta.additional_artists.is_empty() {
        comments.push(format!("ARTISTS={}", meta.additional_artists.join(", ")));
        for extra in &meta.additional_artists {
            comments.push(format!("FEAT={}", extra));
        }
    }
    if let Some(ref alb) = meta.album {
        comments.push(format!("ALBUM={}", alb));
    }
    if let Some(ref aa) = meta.album_artist {
        comments.push(format!("ALBUMARTIST={}", aa));
    }
    if let Some(ref g) = meta.genre {
        comments.push(format!("GENRE={}", g));
    }
    if let Some(y) = meta.year {
        comments.push(format!("DATE={}", y));
    }
    if let Some(tr) = meta.track_number {
        if let Some(tot) = meta.track_total {
            comments.push(format!("TRACKNUMBER={}/{}", tr, tot));
        } else {
            comments.push(format!("TRACKNUMBER={}", tr));
        }
    }
    if let Some(dn) = meta.disc_number {
        comments.push(format!("DISCNUMBER={}", dn));
    }
    if let Some(ref c) = meta.composer {
        comments.push(format!("COMPOSER={}", c));
    }
    if let Some(ref cm) = meta.comment {
        comments.push(format!("COMMENT={}", cm));
    }
    if let Some(ref ly) = meta.lyrics {
        comments.push(format!("LYRICS={}", ly));
    }
    if let Some(bpm) = meta.bpm {
        comments.push(format!("BPM={:.1}", bpm));
    }
    if let Some(ref k) = meta.musical_key {
        comments.push(format!("KEY={}", k));
    }
    if let Some(ref l) = meta.record_label {
        comments.push(format!("LABEL={}", l));
    }
    if let Some(ref cat) = meta.catalog_number {
        comments.push(format!("CATALOGNUMBER={}", cat));
    }
    if let Some(ref at) = meta.album_type {
        comments.push(format!("ALBUMTYPE={}", at));
    }
    if let Some(ref rd) = meta.release_date {
        comments.push(format!("RELEASEDATE={}", rd));
    }
    if let Some(ref lang) = meta.language {
        comments.push(format!("LANGUAGE={}", lang));
    }
    if let Some(ref mbid) = meta.musicbrainz_id {
        comments.push(format!("MUSICBRAINZ_ALBUMID={}", mbid));
    }
    if let Some(ref mbtid) = meta.musicbrainz_track_id {
        comments.push(format!("MUSICBRAINZ_TRACKID={}", mbtid));
    }
    if let Some(ref u) = meta.online_query_url {
        comments.push(format!("ONLINE_QUERY_URL={}", u));
    }
    if let Some(rg) = meta.replaygain_track_gain_db {
        comments.push(format!("REPLAYGAIN_TRACK_GAIN={:+.2} dB", rg));
    }
    if let Some(rp) = meta.replaygain_track_peak {
        comments.push(format!("REPLAYGAIN_TRACK_PEAK={:.6}", rp));
    }
    if let Some(st) = meta.start_trim_ms {
        comments.push(format!("START_TRIM_MS={}", st));
    }
    if let Some(et) = meta.end_trim_ms {
        comments.push(format!("END_TRIM_MS={}", et));
    }
    if let Some(sf) = meta.speed_factor {
        comments.push(format!("SPEED_FACTOR={:.4}", sf));
    }

    out.extend_from_slice(&(comments.len() as u32).to_le_bytes());
    for c in comments {
        let bytes = c.as_bytes();
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(bytes);
    }

    out
}

// =========================================================================
// MP3 Metadata Writer (Preserves Audio Frames 100% Bit-for-Bit)
// =========================================================================

fn write_mp3_metadata(path: &Path, meta: &TrackMetadata) -> Result<(), TrackTweaksError> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let mut audio_start = 0;
    if data.len() >= 10 && &data[0..3] == b"ID3" {
        let tag_size = decode_synchsafe([data[6], data[7], data[8], data[9]]) as usize;
        audio_start = 10 + tag_size;
    }

    if audio_start > data.len() {
        audio_start = 0;
    }

    let id3v2_tag = build_id3v24_tag(meta);

    let mut out = Vec::with_capacity(id3v2_tag.len() + data.len() - audio_start);
    out.extend_from_slice(&id3v2_tag);
    out.extend_from_slice(&data[audio_start..]);

    atomic_write(path, &out)
}

fn encode_synchsafe(mut val: u32) -> [u8; 4] {
    let mut out = [0u8; 4];
    out[3] = (val & 0x7F) as u8;
    val >>= 7;
    out[2] = (val & 0x7F) as u8;
    val >>= 7;
    out[1] = (val & 0x7F) as u8;
    val >>= 7;
    out[0] = (val & 0x7F) as u8;
    out
}

fn decode_synchsafe(bytes: [u8; 4]) -> u32 {
    ((bytes[0] as u32 & 0x7F) << 21)
        | ((bytes[1] as u32 & 0x7F) << 14)
        | ((bytes[2] as u32 & 0x7F) << 7)
        | (bytes[3] as u32 & 0x7F)
}

fn build_id3v24_tag(meta: &TrackMetadata) -> Vec<u8> {
    let mut frames_data = Vec::new();

    if let Some(ref t) = meta.title {
        append_id3_text_frame(&mut frames_data, b"TIT2", t);
    }
    if let Some(ref a) = meta.artist {
        append_id3_text_frame(&mut frames_data, b"TPE1", a);
    }
    if let Some(ref alb) = meta.album {
        append_id3_text_frame(&mut frames_data, b"TALB", alb);
    }
    if let Some(ref aa) = meta.album_artist {
        append_id3_text_frame(&mut frames_data, b"TPE2", aa);
    }
    if let Some(ref g) = meta.genre {
        append_id3_text_frame(&mut frames_data, b"TCON", g);
    }
    if let Some(y) = meta.year {
        append_id3_text_frame(&mut frames_data, b"TDRC", &y.to_string());
    }
    if let Some(tr) = meta.track_number {
        let val = if let Some(tot) = meta.track_total {
            format!("{}/{}", tr, tot)
        } else {
            tr.to_string()
        };
        append_id3_text_frame(&mut frames_data, b"TRCK", &val);
    }
    if let Some(dn) = meta.disc_number {
        append_id3_text_frame(&mut frames_data, b"TPOS", &dn.to_string());
    }
    if let Some(ref c) = meta.composer {
        append_id3_text_frame(&mut frames_data, b"TCOM", c);
    }
    if let Some(bpm) = meta.bpm {
        append_id3_text_frame(&mut frames_data, b"TBPM", &format!("{:.1}", bpm));
    }
    if let Some(ref k) = meta.musical_key {
        append_id3_text_frame(&mut frames_data, b"TKEY", k);
    }
    if let Some(ref l) = meta.record_label {
        append_id3_text_frame(&mut frames_data, b"TPUB", l);
    }
    if let Some(ref lang) = meta.language {
        append_id3_text_frame(&mut frames_data, b"TLAN", lang);
    }
    if let Some(ref cat) = meta.catalog_number {
        append_id3_text_frame(&mut frames_data, b"TSRC", cat);
    }
    if let Some(ref rd) = meta.release_date {
        append_id3_text_frame(&mut frames_data, b"TDRC", rd);
    }
    if !meta.additional_artists.is_empty() {
        append_id3_txxx_frame(
            &mut frames_data,
            "ARTISTS",
            &meta.additional_artists.join(", "),
        );
    }
    if let Some(ref at) = meta.album_type {
        append_id3_txxx_frame(&mut frames_data, "ALBUMTYPE", at);
    }
    if let Some(ref mbid) = meta.musicbrainz_id {
        append_id3_txxx_frame(&mut frames_data, "MusicBrainz Album Id", mbid);
    }
    if let Some(ref mbtid) = meta.musicbrainz_track_id {
        append_id3_txxx_frame(&mut frames_data, "MusicBrainz Track Id", mbtid);
    }
    if let Some(ref u) = meta.online_query_url {
        append_id3_txxx_frame(&mut frames_data, "ONLINE_QUERY_URL", u);
    }

    // Comment frame COMM
    if let Some(ref cm) = meta.comment {
        let mut payload = Vec::new();
        payload.push(0x03); // UTF-8
        payload.extend_from_slice(b"eng"); // ISO language
        payload.push(0x00); // Empty short description terminated by \0
        payload.extend_from_slice(cm.as_bytes());
        append_id3_raw_frame(&mut frames_data, b"COMM", &payload);
    }

    // Unsynchronized lyrics USLT
    if let Some(ref ly) = meta.lyrics {
        let mut payload = Vec::new();
        payload.push(0x03); // UTF-8
        payload.extend_from_slice(b"eng");
        payload.push(0x00);
        payload.extend_from_slice(ly.as_bytes());
        append_id3_raw_frame(&mut frames_data, b"USLT", &payload);
    }

    // Custom TXXX frames for non-destructive tweaks
    if let Some(rg) = meta.replaygain_track_gain_db {
        append_id3_txxx_frame(
            &mut frames_data,
            "REPLAYGAIN_TRACK_GAIN",
            &format!("{:+.2} dB", rg),
        );
    }
    if let Some(rp) = meta.replaygain_track_peak {
        append_id3_txxx_frame(
            &mut frames_data,
            "REPLAYGAIN_TRACK_PEAK",
            &format!("{:.6}", rp),
        );
    }
    if let Some(st) = meta.start_trim_ms {
        append_id3_txxx_frame(&mut frames_data, "START_TRIM_MS", &st.to_string());
    }
    if let Some(et) = meta.end_trim_ms {
        append_id3_txxx_frame(&mut frames_data, "END_TRIM_MS", &et.to_string());
    }
    if let Some(sf) = meta.speed_factor {
        append_id3_txxx_frame(&mut frames_data, "SPEED_FACTOR", &format!("{:.4}", sf));
    }

    // ID3v2.4 Header (10 bytes)
    let mut header = Vec::with_capacity(10 + frames_data.len());
    header.extend_from_slice(b"ID3");
    header.push(0x04); // ID3v2.4
    header.push(0x00); // revision
    header.push(0x00); // flags
    let synch_size = encode_synchsafe(frames_data.len() as u32);
    header.extend_from_slice(&synch_size);
    header.extend_from_slice(&frames_data);

    header
}

fn append_id3_text_frame(dest: &mut Vec<u8>, frame_id: &[u8; 4], text: &str) {
    let mut payload = Vec::with_capacity(1 + text.len());
    payload.push(0x03); // UTF-8 encoding
    payload.extend_from_slice(text.as_bytes());
    append_id3_raw_frame(dest, frame_id, &payload);
}

fn append_id3_txxx_frame(dest: &mut Vec<u8>, desc: &str, val: &str) {
    let mut payload = Vec::new();
    payload.push(0x03); // UTF-8
    payload.extend_from_slice(desc.as_bytes());
    payload.push(0x00); // null separator
    payload.extend_from_slice(val.as_bytes());
    append_id3_raw_frame(dest, b"TXXX", &payload);
}

fn append_id3_raw_frame(dest: &mut Vec<u8>, frame_id: &[u8; 4], payload: &[u8]) {
    dest.extend_from_slice(frame_id);
    let size_bytes = encode_synchsafe(payload.len() as u32);
    dest.extend_from_slice(&size_bytes);
    dest.extend_from_slice(&[0x00, 0x00]); // 2 bytes flags
    dest.extend_from_slice(payload);
}

// =========================================================================
// WAV RIFF INFO Metadata Writer (Preserves Audio Samples 100% Bit-for-Bit)
// =========================================================================

fn write_wav_metadata(path: &Path, meta: &TrackMetadata) -> Result<(), TrackTweaksError> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err(TrackTweaksError::Metadata(
            "Invalid WAV/RIFF header".to_string(),
        ));
    }

    let mut cursor = 12;
    let mut preserved_chunks: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

    while cursor + 8 <= data.len() {
        let chunk_id = &data[cursor..cursor + 4];
        let chunk_size = u32::from_le_bytes([
            data[cursor + 4],
            data[cursor + 5],
            data[cursor + 6],
            data[cursor + 7],
        ]) as usize;

        cursor += 8;
        if cursor + chunk_size > data.len() {
            break;
        }

        let chunk_data = &data[cursor..cursor + chunk_size];
        let padded_size = chunk_size + (chunk_size % 2);
        cursor += padded_size;

        // If it's a LIST INFO chunk, we will replace it with our updated metadata
        if chunk_id == b"LIST" && chunk_data.len() >= 4 && &chunk_data[0..4] == b"INFO" {
            continue;
        }

        preserved_chunks.push((chunk_id.to_vec(), chunk_data.to_vec()));
    }

    let list_info_chunk = build_wav_info_chunk(meta);

    // Calculate total RIFF file size (36 + all chunks)
    let mut total_body_size = 4; // "WAVE"
    for (_id, d) in &preserved_chunks {
        let padded = d.len() + (d.len() % 2);
        total_body_size += 8 + padded;
    }
    let list_padded = list_info_chunk.len() + (list_info_chunk.len() % 2);
    total_body_size += 8 + list_padded;

    let mut out = Vec::with_capacity(total_body_size + 8);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(total_body_size as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");

    // Write fmt chunk and other chunks, then LIST INFO, then data chunk (or maintain order)
    let mut wrote_info = false;
    for (id, d) in preserved_chunks {
        if id == b"data" && !wrote_info {
            // Place LIST INFO right before data chunk
            write_single_chunk(&mut out, b"LIST", &list_info_chunk);
            wrote_info = true;
        }
        write_single_chunk(&mut out, &id, &d);
    }
    if !wrote_info {
        write_single_chunk(&mut out, b"LIST", &list_info_chunk);
    }

    atomic_write(path, &out)
}

fn write_single_chunk(dest: &mut Vec<u8>, id: &[u8], data: &[u8]) {
    dest.extend_from_slice(id);
    dest.extend_from_slice(&(data.len() as u32).to_le_bytes());
    dest.extend_from_slice(data);
    if !data.len().is_multiple_of(2) {
        dest.push(0x00);
    }
}

fn build_wav_info_chunk(meta: &TrackMetadata) -> Vec<u8> {
    let mut info = Vec::new();
    info.extend_from_slice(b"INFO");

    if let Some(ref t) = meta.title {
        append_info_subchunk(&mut info, b"INAM", t);
    }
    if let Some(ref a) = meta.artist {
        append_info_subchunk(&mut info, b"IART", a);
    }
    if let Some(ref alb) = meta.album {
        append_info_subchunk(&mut info, b"IPRD", alb);
    }
    if let Some(ref g) = meta.genre {
        append_info_subchunk(&mut info, b"IGNR", g);
    }
    if let Some(y) = meta.year {
        append_info_subchunk(&mut info, b"ICRD", &y.to_string());
    }
    if let Some(tr) = meta.track_number {
        append_info_subchunk(&mut info, b"ITRK", &tr.to_string());
    }
    if let Some(ref c) = meta.comment {
        append_info_subchunk(&mut info, b"ICMT", c);
    }

    info
}

fn append_info_subchunk(dest: &mut Vec<u8>, id: &[u8; 4], text: &str) {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0x00); // null terminated string
    dest.extend_from_slice(id);
    dest.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    dest.extend_from_slice(&bytes);
    if !bytes.len().is_multiple_of(2) {
        dest.push(0x00); // word aligned
    }
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), TrackTweaksError> {
    let tmp_path = path.with_extension("tmp_tweak");
    {
        let mut tmp = File::create(&tmp_path)?;
        tmp.write_all(content)?;
        tmp.flush()?;
    }
    std::fs::rename(tmp_path, path)?;
    Ok(())
}
