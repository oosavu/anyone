use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio_backend::{AudioBackend, AudioBackendInfo, AudioCallback};

pub struct CpalBackend {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    info: AudioBackendInfo,
    stream: Option<cpal::Stream>,
}

impl CpalBackend {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let config = device
            .default_output_config()
            .expect("no default output config");
        let info = AudioBackendInfo::new(0, config.channels() as u32, config.sample_rate() as u32);

        Self {
            device,
            config,
            info,
            stream: None,
        }
    }
}

impl Default for CpalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for CpalBackend {
    fn info(&self) -> &AudioBackendInfo {
        &self.info
    }

    fn start(&mut self, mut callback: AudioCallback) {
        let stream = self
            .device
            .build_output_stream(
                self.config.clone().into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    callback(data);
                },
                |err| eprintln!("stream error: {err}"),
                None,
            )
            .expect("failed to build output stream");
        stream.play().expect("failed to play stream");
        self.stream = Some(stream);
    }

    fn stop(&mut self) {
        self.stream = None;
    }
}
