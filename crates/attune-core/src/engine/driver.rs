//! Real-Time Audio Output Driver using cpal

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
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

    /// Initialize audio output stream connected to the DualDeckMixer
    pub fn new(mixer: Arc<Mutex<DualDeckMixer>>) -> Result<Self, DriverError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(DriverError::NoDevice)?;

        let default_config = device.default_output_config()?;
        let sample_rate = default_config.sample_rate();
        let channels = default_config.channels();
        let sample_format = default_config.sample_format();
        let mut config = default_config.config();

        // Ensure a stable hardware buffer size (e.g. 2048 frames ~45ms) to prevent ALSA XRUN buffer underruns
        if let cpal::SupportedBufferSize::Range { min, max } = default_config.buffer_size() {
            let desired = 2048.min(*max).max(*min);
            config.buffer_size = cpal::BufferSize::Fixed(desired);
        }

        // Synchronize mixer's output sample rate with the hardware device
        if let Ok(mut m) = mixer.lock() {
            m.set_output_sample_rate(sample_rate);
        }

        let mixer_cb = Arc::clone(&mixer);
        let err_fn = |err| log::warn!("Audio output stream notice: {}", err);

        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut mixer) = mixer_cb.try_lock() {
                        mixer.process_interleaved_block(data);
                    } else {
                        data.fill(0.0);
                    }
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => device.build_output_stream(
                config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut mixer) = mixer_cb.try_lock() {
                        let mut temp = vec![0.0f32; data.len()];
                        mixer.process_interleaved_block(&mut temp);
                        for (dest, &src) in data.iter_mut().zip(temp.iter()) {
                            *dest = (src * 32767.0).clamp(-32768.0, 32767.0) as i16;
                        }
                    } else {
                        data.fill(0);
                    }
                },
                err_fn,
                None,
            )?,
            _ => return Err(DriverError::NoDevice),
        };

        stream.play()?;

        Ok(Self {
            stream,
            sample_rate,
            channels,
            mixer,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn mixer(&self) -> &Arc<Mutex<DualDeckMixer>> {
        &self.mixer
    }
}
