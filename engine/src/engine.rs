use crate::audio_backend::AudioBackend;
use api::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub type ModuleID = Uuid;
pub type BusModuleID = Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Module { id: BusModuleID, port: usize },
    InputBus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sink {
    Module { id: BusModuleID, port: usize },
    OutputBus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cable {
    source: Source,
    sink: Sink,
}

struct BusModuleCallData {
    bus_in: Vec<*const Bus>,
    events_in: Vec<*const EventCable>,
    bus_out: Vec<*const Bus>,
    events_out: Vec<*const EventCable>,
}

struct BusModuleSlot {
    module: Box<dyn BusModule>,
    //pool of buses that used if port is not connected to anything
    stub_buses: Vec<Box<Bus>>,
    call_data: BusModuleCallData,
}

struct RealtimeCore {
    in_bus: Box<Bus>,
    out_bus: Box<Bus>,
    frames_per_buffer: usize,
    bus_modules: HashMap<BusModuleID, BusModuleSlot>,
    cables: Vec<Cable>,
    /// Topological execution order for `process`. Rebuilt after every graph edit.
    /// modules in inner vector are executed in parallel, but modules in outer vector are executed sequentially.
    execution_order: Vec<Vec<BusModuleID>>,
}

impl RealtimeCore {}

fn copy_bus(src: &Bus, dst: &mut Bus) {
    let channels = src.channels().min(dst.channels());
    let frames = src.frames().min(dst.frames());
    for channel in 0..channels {
        dst.channel_mut(channel)[..frames].copy_from_slice(&src.channel(channel)[..frames]);
        dst.set_silent(channel, src.is_silent(channel));
    }
}

pub struct Engine {
    core: Arc<Mutex<RealtimeCore>>,
    audio_backend: Box<dyn AudioBackend>,
}

impl Engine {
    // pub fn new(audio_backend: Box<dyn AudioBackend>) -> Self {
    //     Self {
    //         core: Arc::new(Mutex::new(RealtimeCore::new(0, 0, 0))),
    //         audio_backend,
    //     }
    // }

    // pub fn add_module(&mut self, module: Box<dyn Module>) {
    //     let mut core = self.core.lock().unwrap();
    //     core.modules.insert(module.id(), module);
    // }

    // pub fn start(&mut self) {
    //     let info = self.audio_backend.info();
    //     let sample_rate = info.sample_rate() as f32;
    //     let channels = info.output_channels() as usize;
    //     let frequency = 440.0;

    //     let mut phase = 0.0f32;
    //     let phase_step = frequency / sample_rate;

    //     self.audio_backend.start(Box::new(move |data: &mut [f32]| {
    //         for frame in data.chunks_mut(channels) {
    //             let sample = (phase * 2.0 * PI).sin();
    //             phase = (phase + phase_step) % 1.0;
    //             for out in frame {
    //                 *out = sample;
    //             }
    //         }
    //     }));
    // }
}
