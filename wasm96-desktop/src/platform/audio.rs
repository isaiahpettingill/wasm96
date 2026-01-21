use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Split},
    wrap::caching::Caching,
};
use std::sync::Arc;

/// Concrete type for the audio producer to avoid dyn compatibility issues
pub type AudioProducer = Caching<Arc<HeapRb<i16>>, true, false>;
pub type AudioConsumer = Caching<Arc<HeapRb<i16>>, false, true>;

pub struct AudioStream {
    _stream: Box<dyn StreamTrait>,
}

impl AudioStream {
    pub fn new(mut cons: AudioConsumer) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No audio output device found")?;
        let config = device.default_output_config()?;

        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| {
                for sample in data.iter_mut() {
                    if let Some(s) = cons.try_pop() {
                        *sample = s as f32 / 32768.0;
                    } else {
                        *sample = 0.0;
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;
        stream.play()?;

        Ok(Self {
            _stream: Box::new(stream),
        })
    }
}

pub fn init_audio() -> Result<(AudioProducer, AudioStream)> {
    const AUDIO_BUFFER_SIZE: usize = 4096;
    let rb = Arc::new(HeapRb::<i16>::new(AUDIO_BUFFER_SIZE * 2));
    let (prod, cons) = rb.split();

    // In ringbuf 0.4, we don't clone the producer/consumer directly.
    // Split() gives us the base wrapper.
    let stream = AudioStream::new(cons)?;

    Ok((prod, stream))
}
