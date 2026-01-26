use log::error;
use wasm96_engine::PlatformAudio;
use wasm_bindgen::JsValue;
use web_sys::{AudioContext, AudioContextState};

pub struct WebAudio {
    ctx: AudioContext,
    next_start_time: f64,
}

impl WebAudio {
    pub fn new() -> Result<Self, JsValue> {
        let ctx = AudioContext::new()?;
        Ok(Self {
            ctx,
            next_start_time: 0.0,
        })
    }

    pub fn resume(&self) {
        if self.ctx.state() == AudioContextState::Suspended {
            let _ = self.ctx.resume();
        }
    }
}

impl PlatformAudio for WebAudio {
    fn audio_batch(&mut self, samples: &[i16]) {
        // Assume 44100Hz Stereo
        let sample_rate = 44100.0;
        let channel_count = 2;
        let frame_count = (samples.len() / 2) as u32;

        if frame_count == 0 {
            return;
        }

        // Convert i16 to f32
        let mut left_channel = Vec::with_capacity(frame_count as usize);
        let mut right_channel = Vec::with_capacity(frame_count as usize);

        for chunk in samples.chunks_exact(2) {
            left_channel.push(chunk[0] as f32 / 32768.0);
            right_channel.push(chunk[1] as f32 / 32768.0);
        }

        let buffer = match self
            .ctx
            .create_buffer(channel_count, frame_count, sample_rate)
        {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to create audio buffer: {:?}", e);
                return;
            }
        };

        if let Err(e) = buffer.copy_to_channel(&mut left_channel, 0) {
            error!("Failed to copy left channel: {:?}", e);
            return;
        }
        if let Err(e) = buffer.copy_to_channel(&mut right_channel, 1) {
            error!("Failed to copy right channel: {:?}", e);
            return;
        }

        let source = match self.ctx.create_buffer_source() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create buffer source: {:?}", e);
                return;
            }
        };

        source.set_buffer(Some(&buffer));

        if let Err(e) = source.connect_with_audio_node(&self.ctx.destination()) {
            error!("Failed to connect audio source: {:?}", e);
            return;
        }

        let current_time = self.ctx.current_time();
        // Schedule slightly in the future if we are ahead, or reset if we lagged
        if self.next_start_time < current_time {
            self.next_start_time = current_time + 0.02; // Small buffer
        }

        if let Err(e) = source.start_with_when(self.next_start_time) {
            error!("Failed to start audio source: {:?}", e);
            return;
        }

        self.next_start_time += buffer.duration();
    }
}
