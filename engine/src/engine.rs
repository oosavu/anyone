use std::{collections::HashMap, f32::consts::PI};

use api::*;

use crate::audio_backend::AudioBackend;

struct RealtimeCore {
    in_bus: Bus,
    out_bus: Bus,
    modules: HashMap<String, Box<dyn Module>>,
}

impl RealtimeCore {
    fn new() -> Self {
        Self {
            in_bus: Bus::new(2, 512),
            out_bus: Bus::new(2, 512),
            modules: HashMap::new(),
        }
    }
}

pub struct Engine {
    modules: Vec<Box<dyn Module>>,
    audio_backend: Box<dyn AudioBackend>,
}

impl Engine {
    pub fn new(audio_backend: Box<dyn AudioBackend>) -> Self {
        Self {
            modules: Vec::new(),
            audio_backend,
        }
    }

    pub fn add_module(&mut self, module: Box<dyn Module>) {
        self.modules.push(module);
    }

    pub fn start(&mut self) {
        let info = self.audio_backend.info();
        let sample_rate = info.sample_rate() as f32;
        let channels = info.output_channels() as usize;
        let frequency = 440.0;

        let mut phase = 0.0f32;
        let phase_step = frequency / sample_rate;

        self.audio_backend.start(Box::new(move |data: &mut [f32]| {
            for frame in data.chunks_mut(channels) {
                let sample = (phase * 2.0 * PI).sin();
                phase = (phase + phase_step) % 1.0;
                for out in frame {
                    *out = sample;
                }
            }
        }));
    }
}
