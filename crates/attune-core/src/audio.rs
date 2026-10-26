use std::path::Path;
use symphonia::core::audio::AudioBufferRef;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Audio decode error: {0}")]
    Decode(String),
    #[error("No supported audio tracks found")]
    NoSupportedTrack,
}

#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Planar channel audio samples (normalized -1.0 to 1.0)
    pub channels: Vec<Vec<f32>>,
    pub sample_rate: u32,
}

impl AudioBuffer {
    pub fn new(channels: usize, sample_rate: u32) -> Self {
        Self {
            channels: vec![Vec::new(); channels],
            sample_rate,
        }
    }

    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }

    pub fn num_frames(&self) -> usize {
        self.channels.first().map(|c| c.len()).unwrap_or(0)
    }

    pub fn duration_seconds(&self) -> f32 {
        self.num_frames() as f32 / self.sample_rate as f32
    }

    pub fn duration_ms(&self) -> u64 {
        (self.duration_seconds() * 1000.0) as u64
    }

    /// Extract mono mix (average of all channels)
    pub fn to_mono(&self) -> Vec<f32> {
        let frames = self.num_frames();
        let num_ch = self.num_channels();
        if num_ch == 0 || frames == 0 {
            return Vec::new();
        }
        if num_ch == 1 {
            return self.channels[0].clone();
        }

        let mut mono = vec![0.0f32; frames];
        let factor = 1.0 / num_ch as f32;
        for ch in &self.channels {
            for (i, &s) in ch.iter().enumerate() {
                mono[i] += s * factor;
            }
        }
        mono
    }

    /// Slice a sub-window of audio from start_ms to end_ms
    pub fn slice_ms(&self, start_ms: u64, end_ms: u64) -> Self {
        let sr = self.sample_rate as f64;
        let start_frame = ((start_ms as f64 / 1000.0) * sr) as usize;
        let end_frame = ((end_ms as f64 / 1000.0) * sr) as usize;
        let total_frames = self.num_frames();

        let s = start_frame.min(total_frames);
        let e = end_frame.min(total_frames).max(s);

        let mut new_channels = Vec::with_capacity(self.num_channels());
        for ch in &self.channels {
            new_channels.push(ch[s..e].to_vec());
        }

        Self {
            channels: new_channels,
            sample_rate: self.sample_rate,
        }
    }

    /// Find the first timestamp (in ms) where audio energy exceeds threshold_db
    pub fn find_audible_start_ms(&self, threshold_db: f32) -> u64 {
        let num_frames = self.num_frames();
        if num_frames == 0 || self.channels.is_empty() {
            return 0;
        }
        let sr = self.sample_rate as usize;
        let window = (sr / 50).max(1); // 20ms window
        let thresh_linear = 10.0f32.powf(threshold_db / 20.0);
        let max_scan_frames = (sr * 30).min(num_frames); // scan at most first 30s

        let num_channels = self.channels.len();
        let inv_ch = 1.0f32 / num_channels as f32;

        for chunk_start in (0..max_scan_frames).step_by(window) {
            let chunk_end = (chunk_start + window).min(num_frames);
            let chunk_len = chunk_end - chunk_start;
            let mut sum_sq = 0.0f32;
            for ch in &self.channels {
                for &s in &ch[chunk_start..chunk_end] {
                    sum_sq += s * s;
                }
            }
            let rms = (sum_sq * inv_ch / chunk_len as f32).sqrt();
            if rms >= thresh_linear {
                return ((chunk_start as f64 / self.sample_rate as f64) * 1000.0) as u64;
            }
        }
        0
    }

    /// Find the last timestamp (in ms) where audio energy exceeds threshold_db
    pub fn find_audible_end_ms(&self, threshold_db: f32) -> u64 {
        let num_frames = self.num_frames();
        if num_frames == 0 || self.channels.is_empty() {
            return self.duration_ms();
        }
        let sr = self.sample_rate as usize;
        let window = (sr / 50).max(1); // 20ms window
        let thresh_linear = 10.0f32.powf(threshold_db / 20.0);
        let min_scan_frame = num_frames.saturating_sub(sr * 45); // scan at most last 45s

        let num_channels = self.channels.len();
        let inv_ch = 1.0f32 / num_channels as f32;

        let last_chunk_start = (num_frames / window) * window;
        let mut chunk_start = last_chunk_start;

        while chunk_start >= min_scan_frame {
            let chunk_end = (chunk_start + window).min(num_frames);
            let chunk_len = chunk_end - chunk_start;
            if chunk_len > 0 {
                let mut sum_sq = 0.0f32;
                for ch in &self.channels {
                    for &s in &ch[chunk_start..chunk_end] {
                        sum_sq += s * s;
                    }
                }
                let rms = (sum_sq * inv_ch / chunk_len as f32).sqrt();
                if rms >= thresh_linear {
                    return ((chunk_end as f64 / self.sample_rate as f64) * 1000.0) as u64;
                }
            }
            if chunk_start < window {
                break;
            }
            chunk_start -= window;
        }
        self.duration_ms()
    }

    /// Compute a fast content-addressable acoustic hash
    pub fn compute_hash(&self) -> String {
        let mono = self.to_mono();
        let mut hasher = blake3::Hasher::new();
        // Sample every 4th sample to be fast while robust
        for chunk in mono.chunks(4) {
            if let Some(&val) = chunk.first() {
                hasher.update(&val.to_le_bytes());
            }
        }
        hasher.update(&self.duration_ms().to_le_bytes());
        hasher.finalize().to_hex().to_string()
    }
}

pub struct AudioDecoder;

impl AudioDecoder {
    /// Load an entire audio file into memory as an AudioBuffer
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<AudioBuffer, AudioError> {
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let hint = Hint::new();
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| AudioError::Decode(e.to_string()))?;

        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(AudioError::NoSupportedTrack)?;

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

        let dec_opts: DecoderOptions = Default::default();
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .map_err(|e| AudioError::Decode(e.to_string()))?;

        let mut audio_buf = AudioBuffer::new(channels, sample_rate);

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(AudioError::Decode(e.to_string())),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    Self::copy_decoded_samples(decoded, &mut audio_buf);
                }
                Err(SymphoniaError::DecodeError(msg)) => {
                    log::warn!("Decode warning: {}", msg);
                }
                Err(e) => return Err(AudioError::Decode(e.to_string())),
            }
        }

        Ok(audio_buf)
    }

    /// Load only the Cue Zones (first 45s and last 45s) for On-Demand JIT Mode 1
    pub fn load_head_tail<P: AsRef<Path>>(
        path: P,
        head_seconds: f32,
        tail_seconds: f32,
    ) -> Result<(AudioBuffer, AudioBuffer, u64), AudioError> {
        let full = Self::load_file(path)?;
        let dur_ms = full.duration_ms();
        let head_ms = (head_seconds * 1000.0) as u64;
        let tail_ms = (tail_seconds * 1000.0) as u64;

        let head = full.slice_ms(0, head_ms.min(dur_ms));
        let tail_start = dur_ms.saturating_sub(tail_ms);
        let tail = full.slice_ms(tail_start, dur_ms);

        Ok((head, tail, dur_ms))
    }

    fn copy_decoded_samples(decoded: AudioBufferRef, dest: &mut AudioBuffer) {
        use symphonia::core::audio::SampleBuffer;

        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let samples = sample_buf.samples();
        let num_ch = dest.num_channels();
        if num_ch == 0 {
            return;
        }

        for (i, &sample) in samples.iter().enumerate() {
            let ch = i % num_ch;
            dest.channels[ch].push(sample);
        }
    }
}
