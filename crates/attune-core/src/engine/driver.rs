//! Real-Time Audio Output Driver using cpal

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use super::mixer::DualDeckMixer;

#[derive(Error, Debug)]
pub enum DriverError {
    #[error("No audio output device available")]
    NoDevice,
    #[error("CPAL audio error: {0}")]
    Cpal(#[from] cpal::Error),
}

pub struct AudioOutputDriver {
    #[allow(dead_code)]
    stream: Stream,
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
    is_forced_32bit: bool,
    device_name: String,
    mixer: Arc<Mutex<DualDeckMixer>>,
}

impl AudioOutputDriver {
    /// Query the default hardware output sample rate
    pub fn default_sample_rate() -> Result<u32, DriverError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(DriverError::NoDevice)?;
        let default_config = device.default_output_config()?;
        Ok(default_config.sample_rate())
    }

    /// Initialize audio output stream connected to the DualDeckMixer (default parameters)
    pub fn new(mixer: Arc<Mutex<DualDeckMixer>>) -> Result<Self, DriverError> {
        Self::new_with_options(mixer, false)
    }

    /// Initialize audio output stream connected to the DualDeckMixer with optional forced 32-bit float output
    pub fn new_with_options(mixer: Arc<Mutex<DualDeckMixer>>, force_32bit: bool) -> Result<Self, DriverError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(DriverError::NoDevice)?;
        let device_name = format!("{}", device);

        let default_config = device.default_output_config()?;
        let default_sample_rate = default_config.sample_rate();
        let default_format = default_config.sample_format();

        let mut chosen_config = default_config.config();
        let mut chosen_format = default_format;
        let mut is_forced = false;

        if force_32bit {
            let mut found_32bit = false;
            if let Ok(supported_configs) = device.supported_output_configs() {
                let configs: Vec<_> = supported_configs.collect();

                // 1. First priority: SampleFormat::F32 (native 32-bit float PCM)
                if let Some(f32_range) = configs.iter().find(|c| c.sample_format() == SampleFormat::F32) {
                    let rate = if default_sample_rate >= f32_range.min_sample_rate() && default_sample_rate <= f32_range.max_sample_rate() {
                        default_sample_rate
                    } else if f32_range.min_sample_rate() <= 48000 && f32_range.max_sample_rate() >= 48000 {
                        48000
                    } else if f32_range.min_sample_rate() <= 44100 && f32_range.max_sample_rate() >= 44100 {
                        44100
                    } else {
                        f32_range.max_sample_rate()
                    };
                    chosen_config = f32_range.with_sample_rate(rate).config();
                    chosen_format = SampleFormat::F32;
                    is_forced = true;
                    found_32bit = true;
                }
                // 2. Second priority: SampleFormat::I32 (native 32-bit integer PCM)
                else if let Some(i32_range) = configs.iter().find(|c| c.sample_format() == SampleFormat::I32) {
                    let rate = if default_sample_rate >= i32_range.min_sample_rate() && default_sample_rate <= i32_range.max_sample_rate() {
                        default_sample_rate
                    } else if i32_range.min_sample_rate() <= 48000 && i32_range.max_sample_rate() >= 48000 {
                        48000
                    } else if i32_range.min_sample_rate() <= 44100 && i32_range.max_sample_rate() >= 44100 {
                        44100
                    } else {
                        i32_range.max_sample_rate()
                    };
                    chosen_config = i32_range.with_sample_rate(rate).config();
                    chosen_format = SampleFormat::I32;
                    is_forced = true;
                    found_32bit = true;
                }
            }

            if !found_32bit && default_format == SampleFormat::F32 {
                chosen_format = SampleFormat::F32;
                is_forced = true;
            }
        }

        // Ensure a stable hardware buffer size (e.g. 2048 frames ~45ms) to prevent ALSA XRUN buffer underruns on desktop.
        #[cfg(not(target_os = "android"))]
        if let cpal::SupportedBufferSize::Range { min, max } = default_config.buffer_size() {
            let desired = 2048.min(*max).max(*min);
            chosen_config.buffer_size = cpal::BufferSize::Fixed(desired);
        }

        let sample_rate = chosen_config.sample_rate;
        let channels = chosen_config.channels;

        // Synchronize mixer's output sample rate with the hardware device
        if let Ok(mut m) = mixer.lock() {
            m.set_output_sample_rate(sample_rate);
        }

        log::info!(
            "AudioOutputDriver: host={}, device={}, sample_rate={} Hz, channels={}, format={:?}, forced_32bit={}",
            host.id().name(),
            device_name,
            sample_rate,
            channels,
            chosen_format,
            is_forced
        );

        let mixer_cb = Arc::clone(&mixer);
        let err_fn = |err| log::warn!("Audio output stream notice: {}", err);

        let build_stream = |fmt: SampleFormat, cfg: StreamConfig| -> Result<Stream, cpal::Error> {
            match fmt {
                SampleFormat::F32 => {
                    let m_cb = Arc::clone(&mixer_cb);
                    device.build_output_stream(
                        cfg,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            if let Ok(mut mixer) = m_cb.try_lock() {
                                mixer.process_interleaved_block(data);
                            } else {
                                crate::engine::mixer::record_underrun_event();
                                let len = data.len();
                                for (i, sample) in data.iter_mut().enumerate() {
                                    let ramp = (1.0 - (i as f32 / len.min(64) as f32)).max(0.0);
                                    *sample *= ramp;
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                SampleFormat::I32 => {
                    let m_cb = Arc::clone(&mixer_cb);
                    device.build_output_stream(
                        cfg,
                        move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                            if let Ok(mut mixer) = m_cb.try_lock() {
                                let mut temp = vec![0.0f32; data.len()];
                                mixer.process_interleaved_block(&mut temp);
                                for (dest, &src) in data.iter_mut().zip(temp.iter()) {
                                    *dest = (src.clamp(-1.0, 1.0) * 2147483647.0) as i32;
                                }
                            } else {
                                crate::engine::mixer::record_underrun_event();
                                let len = data.len();
                                for (i, sample) in data.iter_mut().enumerate() {
                                    let ramp = (1.0 - (i as f32 / len.min(64) as f32)).max(0.0);
                                    *sample = (*sample as f32 * ramp) as i32;
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                SampleFormat::I16 => {
                    let m_cb = Arc::clone(&mixer_cb);
                    device.build_output_stream(
                        cfg,
                        move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                            if let Ok(mut mixer) = m_cb.try_lock() {
                                let mut temp = vec![0.0f32; data.len()];
                                mixer.process_interleaved_block(&mut temp);
                                for (dest, &src) in data.iter_mut().zip(temp.iter()) {
                                    *dest = (src.clamp(-1.0, 1.0) * 32767.0) as i16;
                                }
                            } else {
                                crate::engine::mixer::record_underrun_event();
                                let len = data.len();
                                for (i, sample) in data.iter_mut().enumerate() {
                                    let ramp = (1.0 - (i as f32 / len.min(64) as f32)).max(0.0);
                                    *sample = (*sample as f32 * ramp) as i16;
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                _ => Err(cpal::Error::from(cpal::ErrorKind::UnsupportedConfig)),
            }
        };

        let stream_res = build_stream(chosen_format, chosen_config.clone());

        let (stream, final_format, final_forced) = match stream_res {
            Ok(s) => (s, chosen_format, is_forced),
            Err(e) if force_32bit => {
                log::warn!("Failed to build 32-bit audio stream ({:?}); attempting fallback to default config", e);
                let fallback_config = default_config.config();
                let fallback_format = default_config.sample_format();
                let fallback_stream = build_stream(fallback_format, fallback_config)?;
                (fallback_stream, fallback_format, false)
            }
            Err(e) => return Err(DriverError::Cpal(e)),
        };

        stream.play()?;

        Ok(Self {
            stream,
            sample_rate,
            channels,
            sample_format: final_format,
            is_forced_32bit: final_forced,
            device_name,
            mixer,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    pub fn is_forced_32bit(&self) -> bool {
        self.is_forced_32bit
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn precision_label(&self) -> String {
        match self.sample_format {
            SampleFormat::F32 => {
                if self.is_forced_32bit {
                    "32-bit Float PCM (Forced)".to_string()
                } else {
                    "32-bit Float Native".to_string()
                }
            }
            SampleFormat::I32 => {
                if self.is_forced_32bit {
                    "32-bit Int PCM (Forced)".to_string()
                } else {
                    "32-bit Int Native".to_string()
                }
            }
            SampleFormat::I16 => "16-bit Int PCM (Standard)".to_string(),
            SampleFormat::U16 => "16-bit Unsigned PCM".to_string(),
            SampleFormat::F64 => "64-bit Float Native".to_string(),
            _ => format!("{:?} PCM", self.sample_format),
        }
    }

    pub fn mixer(&self) -> &Arc<Mutex<DualDeckMixer>> {
        &self.mixer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_sample_format_and_precision_label() {
        let dummy_mixer = Arc::new(Mutex::new(DualDeckMixer::new(44100)));
        let _ = AudioOutputDriver::default_sample_rate();
        // If an audio device is present, test construction with force_32bit
        if let Ok(driver) = AudioOutputDriver::new_with_options(dummy_mixer, true) {
            assert!(driver.sample_rate() > 0);
            assert!(driver.channels() > 0);
            let label = driver.precision_label();
            assert!(!label.is_empty());
        }
    }
}
