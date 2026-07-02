pub struct AudioBackendInfo {
    input_channels: u32,
    output_channels: u32,
    sample_rate: u32,
}

impl AudioBackendInfo {
    pub fn new(input_channels: u32, output_channels: u32, sample_rate: u32) -> Self {
        Self {
            input_channels,
            output_channels,
            sample_rate,
        }
    }

    pub fn input_channels(&self) -> u32 {
        self.input_channels
    }

    pub fn output_channels(&self) -> u32 {
        self.output_channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

pub type AudioCallback = Box<dyn FnMut(&mut [f32]) + Send>;

pub trait AudioBackend {
    fn info(&self) -> &AudioBackendInfo;

    fn start(&mut self, callback: AudioCallback);

    fn stop(&mut self);
}
