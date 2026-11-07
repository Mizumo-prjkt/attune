use hound::{WavSpec, WavWriter};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::TrackTweaksError;

/// In-memory representation of multi-channel interleaved audio.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioData {
    pub channels: usize,
    pub sample_rate: u32,
    pub samples: Vec<f32>, // interleaved audio samples
}

impl AudioData {
    pub fn new(channels: usize, sample_rate: u32, samples: Vec<f32>) -> Self {
        Self {
            channels,
            sample_rate,
            samples,
        }
    }

    #[inline]
    pub fn num_frames(&self) -> usize {
        self.samples.len().checked_div(self.channels).unwrap_or(0)
    }

    #[inline]
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.channels == 0 {
            0
        } else {
            ((self.num_frames() as f64 / self.sample_rate as f64) * 1000.0) as u64
        }
    }
}

/// Decodes an audio file (e.g. WAV, MP3, FLAC, OGG, AAC) into interleaved f32 samples using Symphonia.
pub fn decode_audio_file<P: AsRef<Path>>(path: P) -> Result<AudioData, TrackTweaksError> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();
    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| TrackTweaksError::Decode(e.to_string()))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(TrackTweaksError::NoSupportedTrack)?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

    let dec_opts: DecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .map_err(|e| TrackTweaksError::Decode(e.to_string()))?;

    let mut samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(TrackTweaksError::Decode(e.to_string())),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                sample_buf.copy_interleaved_ref(decoded);
                samples.extend_from_slice(sample_buf.samples());
            }
            Err(SymphoniaError::DecodeError(msg)) => {
                log::warn!("Decode warning: {}", msg);
            }
            Err(e) => return Err(TrackTweaksError::Decode(e.to_string())),
        }
    }

    Ok(AudioData {
        channels,
        sample_rate,
        samples,
    })
}

/// Encodes an AudioData struct into a 16-bit PCM WAV file using Hound.
pub fn write_wav_file<P: AsRef<Path>>(path: P, audio: &AudioData) -> Result<(), TrackTweaksError> {
    if audio.channels == 0 {
        return Err(TrackTweaksError::InvalidParameter(
            "Audio channels cannot be 0".to_string(),
        ));
    }
    if audio.sample_rate == 0 {
        return Err(TrackTweaksError::InvalidParameter(
            "Sample rate cannot be 0".to_string(),
        ));
    }

    let spec = WavSpec {
        channels: audio.channels as u16,
        sample_rate: audio.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    for &sample in &audio.samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let s_i16 = (clamped * 32767.0).round() as i16;
        writer.write_sample(s_i16)?;
    }
    writer.finalize()?;
    Ok(())
}
