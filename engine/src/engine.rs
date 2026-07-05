use std::{collections::HashMap, f32::consts::PI, ops::Range};

use api::*;

use crate::audio_backend::AudioBackend;

/// Number of samples per audio callback. Only sizes `in_bus`/`out_bus`; the
/// module graph itself is ticked one sample at a time (see
/// `RealtimeCore::process`).
const FRAMES_PER_BLOCK: usize = 512;

/// Where a cable's signal comes from: an output port of a module, or a
/// channel of the engine's input bus (e.g. audio coming in from the sound
/// card).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Source {
    Module { id: ModuleID, port: usize },
    InputBus { channel: usize },
}

/// Where a cable's signal goes to: an input port of a module, or a channel of
/// the engine's output bus (e.g. audio going out to the sound card).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Sink {
    Module { id: ModuleID, port: usize },
    OutputBus { channel: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Cable {
    source: Source,
    sink: Sink,
}

/// A resolved [`Source`]: points directly at a slot in `RealtimeCore::ports`
/// (or a bus channel) instead of a `(ModuleID, port)` pair, so `process`
/// never has to look modules up by id.
#[derive(Clone, Copy, Debug)]
enum ResolvedSource {
    Port(usize),
    InputBus(usize),
}

/// The fast-path cable cache: for every port, and every output-bus channel,
/// the (at most one) resolved source feeding it. A sink can only ever have
/// one cable plugged into it, so there is nothing to sum - `process` just
/// copies. Rebuilt any time the module graph or its cables change.
#[derive(Default)]
struct Wiring {
    port_sources: Vec<Option<ResolvedSource>>,
    output_bus_sources: Vec<Option<ResolvedSource>>,
}

struct ModuleSlot {
    module: Box<dyn Module>,
    /// Range in `RealtimeCore::ports` holding this module's ports: the first
    /// `input_count` entries are its input ports, the rest are its outputs.
    port_range: Range<usize>,
    input_count: usize,
    events_out: Vec<EventCable>,
}

impl ModuleSlot {
    fn input_port(&self, local: usize) -> usize {
        self.port_range.start + local
    }

    fn output_port(&self, local: usize) -> usize {
        self.port_range.start + self.input_count + local
    }

    fn output_count(&self) -> usize {
        self.port_range.len() - self.input_count
    }
}

struct RealtimeCore {
    in_bus: Bus,
    out_bus: Bus,
    modules: HashMap<ModuleID, ModuleSlot>,
    cables: Vec<Cable>,
    /// Backing storage for every declared input/output port of every module.
    ports: Vec<Port>,
    wiring: Wiring,
}

impl RealtimeCore {
    fn new() -> Self {
        Self {
            in_bus: Bus::new(2, FRAMES_PER_BLOCK),
            out_bus: Bus::new(2, FRAMES_PER_BLOCK),
            modules: HashMap::new(),
            cables: Vec::new(),
            ports: Vec::new(),
            wiring: Wiring::default(),
        }
    }

    fn add_module(&mut self, id: ModuleID, module: Box<dyn Module>) -> Result<(), String> {
        if self.modules.contains_key(&id) {
            return Err(format!("module '{id}' already exists"));
        }

        let events_out = module
            .info()
            .output_event_ports
            .iter()
            .map(|_| EventCable::new())
            .collect();

        self.modules.insert(
            id,
            ModuleSlot {
                module,
                port_range: 0..0,
                input_count: 0,
                events_out,
            },
        );
        self.rebuild_ports();
        self.rebuild_wiring();
        Ok(())
    }

    fn remove_module(&mut self, id: &str) -> Option<Box<dyn Module>> {
        self.cables.retain(|cable| {
            !matches!(&cable.source, Source::Module { id: src, .. } if src == id)
                && !matches!(&cable.sink, Sink::Module { id: dst, .. } if dst == id)
        });
        let removed = self.modules.remove(id).map(|slot| slot.module);
        if removed.is_some() {
            self.rebuild_ports();
            self.rebuild_wiring();
        }
        removed
    }

    fn connect(&mut self, source: Source, sink: Sink) -> Result<(), String> {
        self.validate_source(&source)?;
        self.validate_sink(&sink)?;
        if self.cables.iter().any(|c| c.sink == sink) {
            return Err("sink already has a cable connected; disconnect it first".to_string());
        }
        self.cables.push(Cable { source, sink });
        self.rebuild_wiring();
        Ok(())
    }

    fn disconnect(&mut self, source: &Source, sink: &Sink) {
        self.cables.retain(|c| &c.source != source || &c.sink != sink);
        self.rebuild_wiring();
    }

    fn validate_source(&self, source: &Source) -> Result<(), String> {
        match source {
            Source::Module { id, port } => {
                let slot = self
                    .modules
                    .get(id)
                    .ok_or_else(|| format!("unknown module '{id}'"))?;
                if *port >= slot.output_count() {
                    return Err(format!("module '{id}' has no output port {port}"));
                }
            }
            Source::InputBus { channel } => {
                if *channel >= self.in_bus.channels() {
                    return Err(format!("input bus has no channel {channel}"));
                }
            }
        }
        Ok(())
    }

    fn validate_sink(&self, sink: &Sink) -> Result<(), String> {
        match sink {
            Sink::Module { id, port } => {
                let slot = self
                    .modules
                    .get(id)
                    .ok_or_else(|| format!("unknown module '{id}'"))?;
                if *port >= slot.input_count {
                    return Err(format!("module '{id}' has no input port {port}"));
                }
            }
            Sink::OutputBus { channel } => {
                if *channel >= self.out_bus.channels() {
                    return Err(format!("output bus has no channel {channel}"));
                }
            }
        }
        Ok(())
    }

    /// Reassigns every module a fresh, contiguous range in `ports`: its input
    /// ports first, then its output ports. Must run before `rebuild_wiring`
    /// whenever the set of modules (or their port counts) changes.
    fn rebuild_ports(&mut self) {
        self.ports.clear();
        for slot in self.modules.values_mut() {
            let info = slot.module.info();
            let input_voices: Vec<usize> = info.input_ports.iter().map(|p| p.max_voices).collect();
            let output_voices: Vec<usize> = info.output_ports.iter().map(|p| p.max_voices).collect();
            let input_count = input_voices.len();

            let start = self.ports.len();
            for voices in input_voices {
                self.ports.push(Port::new(voices));
            }
            for voices in output_voices {
                self.ports.push(Port::new(voices));
            }
            slot.port_range = start..self.ports.len();
            slot.input_count = input_count;
        }
    }

    /// Resolves every cable's module/port pair into a direct `ports` index
    /// (or bus channel), indexed by destination for O(1) lookup in `process`.
    /// `connect` guarantees each sink appears in `cables` at most once.
    fn rebuild_wiring(&mut self) {
        let mut port_sources = vec![None; self.ports.len()];
        let mut output_bus_sources: Vec<Option<ResolvedSource>> =
            (0..self.out_bus.channels()).map(|_| None).collect();

        for cable in &self.cables {
            let source = match &cable.source {
                Source::Module { id, port } => ResolvedSource::Port(self.modules[id].output_port(*port)),
                Source::InputBus { channel } => ResolvedSource::InputBus(*channel),
            };
            match &cable.sink {
                Sink::Module { id, port } => {
                    port_sources[self.modules[id].input_port(*port)] = Some(source);
                }
                Sink::OutputBus { channel } => {
                    output_bus_sources[*channel] = Some(source);
                }
            }
        }

        self.wiring = Wiring {
            port_sources,
            output_bus_sources,
        };
    }

    /// Runs `in_bus`/`out_bus` through the module graph one sample at a time.
    /// Every module-to-module cable carries a fixed one-sample delay: a
    /// module's inputs for this tick are copied from other modules' ports as
    /// they stood at the end of the *previous* tick, since nothing writes to
    /// a port until its owning module's `process` call runs, later in this
    /// same tick. That means modules can run in any order and feedback
    /// cycles are simply valid graphs - no topological sort needed.
    fn process(&mut self) {
        let module_order: Vec<ModuleID> = self.modules.keys().cloned().collect();

        for sample in 0..self.out_bus.frames() {
            for idx in 0..self.ports.len() {
                self.ports[idx].as_mut_slice().fill(0.0);
                match self.wiring.port_sources[idx] {
                    Some(ResolvedSource::Port(src)) => copy_port(&mut self.ports, src, idx),
                    Some(ResolvedSource::InputBus(channel)) => {
                        let value = self.in_bus[(channel, sample)];
                        for v in self.ports[idx].as_mut_slice() {
                            *v = value;
                        }
                    }
                    None => {}
                }
            }

            for id in &module_order {
                let slot = self.modules.get_mut(id).unwrap();
                for cable in &mut slot.events_out {
                    cable.clear();
                }
                let range = slot.port_range.clone();
                let input_count = slot.input_count;
                let (inputs, outputs) = self.ports[range].split_at_mut(input_count);
                let input_slices: Vec<&[f32]> = inputs.iter().map(|p| p.as_slice()).collect();
                let output_slices: Vec<&mut [f32]> =
                    outputs.iter_mut().map(|p| p.as_mut_slice()).collect();
                slot.module
                    .process(&input_slices, &[], &output_slices, &mut slot.events_out);
            }

            for channel in 0..self.out_bus.channels() {
                self.out_bus[(channel, sample)] = match self.wiring.output_bus_sources[channel] {
                    Some(ResolvedSource::Port(idx)) => self.ports[idx].as_slice().iter().sum::<f32>(),
                    Some(ResolvedSource::InputBus(in_channel)) => self.in_bus[(in_channel, sample)],
                    None => 0.0,
                };
            }
        }
    }
}

/// Copies `ports[src]`'s samples into `ports[dst]`, in place and without
/// allocating. `src` and `dst` are always disjoint: one is always an output
/// port and the other always an input port, and every module's ports occupy
/// their own private range.
fn copy_port(ports: &mut [Port], src: usize, dst: usize) {
    let (a, b) = if src < dst {
        let (left, right) = ports.split_at_mut(dst);
        (&left[src], &mut right[0])
    } else {
        let (left, right) = ports.split_at_mut(src);
        (&right[0], &mut left[dst])
    };
    for (d, s) in b.as_mut_slice().iter_mut().zip(a.as_slice().iter()) {
        *d = *s;
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
