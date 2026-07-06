use crate::audio_backend::AudioBackend;
use api::*;
use std::collections::HashMap;
use std::mem;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub type ModuleID = Uuid;
pub type BusModuleID = Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Source {
    Module { id: BusModuleID, port: usize },
    InputBus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Sink {
    Module { id: BusModuleID, port: usize },
    OutputBus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cable {
    source: Source,
    sink: Sink,
}

/// Prebuilt argument lists for one `BusModule::process` call. All pointers
/// are resolved by `rebuild_wiring` on every graph edit, so `process` just
/// transmutes them to the reference slices the trait expects - no lookups,
/// no allocations, no per-block resolution.
///
/// `*const` is used uniformly even for the out slots; `process` transmutes
/// those to `&mut`, which is sound because every out pointer targets a bus
/// or cable no other pointer in the same call refers to.
struct BusModuleCallData {
    bus_in: Vec<*const Bus>,
    events_in: Vec<*const EventCable>,
    bus_out: Vec<*const Bus>,
    events_out: Vec<*const EventCable>,
}

struct BusModuleSlot {
    module: Box<dyn BusModule>,
    /// Pool of buses used when a port is not connected to anything: one per
    /// port, input ports first, then output ports. An unconnected output
    /// writes into its stub (and nobody reads it); an unconnected input
    /// reads permanent silence from its stub.
    stub_buses: Vec<Box<Bus>>,
    stub_event_cables: Vec<Box<EventCable>>,
    call_data: BusModuleCallData,
}

struct RealtimeCore {
    in_bus: Box<Bus>,
    out_bus: Box<Bus>,
    frames_per_buffer: usize,
    bus_modules: HashMap<BusModuleID, BusModuleSlot>,
    cables: Vec<Cable>,
    /// The buses actually carrying signal between connected modules, owned
    /// globally: one per connected source port, shared by all of its sinks
    /// (fan-out reads the same bus). Rebuilt together with the wiring; a
    /// source cabled into the engine output writes straight into `out_bus`
    /// instead, so nothing is ever copied after the graph runs.
    buses: Vec<Box<Bus>>,
    /// Topological execution order for `process`. Rebuilt after every graph edit.
    /// modules in inner vector are executed in parallel, but modules in outer vector are executed sequentially.
    execution_order: Vec<Vec<BusModuleID>>,
}

impl RealtimeCore {
    fn new(input_bus: Box<Bus>, output_bus: Box<Bus>) -> Self {
        let frames_per_buffer = input_bus.frames();
        Self {
            in_bus: input_bus,
            out_bus: output_bus,
            frames_per_buffer,
            bus_modules: HashMap::new(),
            cables: Vec::new(),
            buses: Vec::new(),
            execution_order: Vec::new(),
        }
    }

    fn add_bus_module(&mut self, module: Box<dyn BusModule>) -> BusModuleID {
        let id = Uuid::new_v4();
        let info = module.info();

        let input_count = info.input_ports.len();
        let stub_buses: Vec<Box<Bus>> = info
            .input_ports
            .iter()
            .chain(info.output_ports.iter())
            .map(|p| Box::new(Bus::new(p.max_voices, self.frames_per_buffer)))
            .collect();
        // One stub per event port: inputs first, then outputs. Separate
        // objects so the transmuted `&mut` slices never alias.
        let event_in_count = info.input_event_ports.len();
        let event_out_count = info.output_event_ports.len();
        let stub_event_cables: Vec<Box<EventCable>> = (0..event_in_count + event_out_count)
            .map(|_| Box::new(EventCable::new()))
            .collect();

        let call_data = BusModuleCallData {
            bus_in: stub_buses[..input_count]
                .iter()
                .map(|b| b.as_ref() as *const Bus)
                .collect(),
            events_in: stub_event_cables[..event_in_count]
                .iter()
                .map(|c| c.as_ref() as *const EventCable)
                .collect(),
            bus_out: stub_buses[input_count..]
                .iter()
                .map(|b| b.as_ref() as *const Bus)
                .collect(),
            events_out: stub_event_cables[event_in_count..]
                .iter()
                .map(|c| c.as_ref() as *const EventCable)
                .collect(),
        };

        self.bus_modules.insert(
            id,
            BusModuleSlot {
                module,
                stub_buses,
                stub_event_cables,
                call_data,
            },
        );
        self.rebuild_wiring()
            .expect("adding an unconnected module cannot introduce a cycle");
        id
    }

    fn remove_bus_module(&mut self, id: &BusModuleID) -> Option<Box<dyn BusModule>> {
        self.cables.retain(|cable| {
            !matches!(cable.source, Source::Module { id: src, .. } if src == *id)
                && !matches!(cable.sink, Sink::Module { id: dst, .. } if dst == *id)
        });
        let removed = self.bus_modules.remove(id).map(|slot| slot.module);
        if removed.is_some() {
            self.rebuild_wiring()
                .expect("removing a module cannot introduce a cycle");
        }
        removed
    }

    fn add_bus_cable(&mut self, source: Source, sink: Sink) -> Result<(), String> {
        self.validate_source(&source)?;
        self.validate_sink(&sink)?;
        if source == Source::InputBus && sink == Sink::OutputBus {
            return Err("cannot cable the input bus straight into the output bus: no module would write it".to_string());
        }
        if self.cables.iter().any(|c| c.sink == sink) {
            return Err("sink already has a cable connected; disconnect it first".to_string());
        }
        // A source cabled to the engine output writes into `out_bus` itself,
        // so it must stay exclusive: no fan-out on top of an output-bus
        // cable, in either order.
        if self
            .cables
            .iter()
            .any(|c| c.source == source && (c.sink == Sink::OutputBus || sink == Sink::OutputBus))
        {
            return Err("source is cabled to the output bus (or would be) and cannot fan out".to_string());
        }

        self.cables.push(Cable { source, sink });
        if let Err(err) = self.rebuild_wiring() {
            self.cables.pop();
            return Err(err);
        }
        Ok(())
    }

    fn remove_bus_cable(&mut self, source: Source, sink: Sink) {
        self.cables.retain(|c| c.source != source || c.sink != sink);
        self.rebuild_wiring()
            .expect("removing a cable cannot introduce a cycle");
    }

    fn validate_source(&self, source: &Source) -> Result<(), String> {
        match source {
            Source::Module { id, port } => {
                let slot = self
                    .bus_modules
                    .get(id)
                    .ok_or_else(|| format!("unknown module '{id}'"))?;
                if *port >= slot.call_data.bus_out.len() {
                    return Err(format!("module '{id}' has no output port {port}"));
                }
            }
            Source::InputBus => {}
        }
        Ok(())
    }

    fn validate_sink(&self, sink: &Sink) -> Result<(), String> {
        match sink {
            Sink::Module { id, port } => {
                let slot = self
                    .bus_modules
                    .get(id)
                    .ok_or_else(|| format!("unknown module '{id}'"))?;
                if *port >= slot.call_data.bus_in.len() {
                    return Err(format!("module '{id}' has no input port {port}"));
                }
            }
            Sink::OutputBus => {}
        }
        Ok(())
    }

    /// Recomputes `execution_order` (erring on cycles, before any state is
    /// touched), reallocates the shared inter-module buses, and re-resolves
    /// every port pointer in every slot's `call_data`:
    /// - unconnected ports point at their own stubs;
    /// - each connected source port gets one bus in `self.buses`, and both
    ///   the source's `bus_out` slot and every sink's `bus_in` slot point at
    ///   it - except a source cabled to the engine output, which writes
    ///   straight into `out_bus`;
    /// - inputs fed from the engine input point straight at `in_bus`.
    /// Must run after every add/remove of a module or cable.
    fn rebuild_wiring(&mut self) -> Result<(), String> {
        let order = self.topo_levels()?;

        for slot in self.bus_modules.values_mut() {
            let input_count = slot.call_data.bus_in.len();
            for (port, source) in slot.call_data.bus_in.iter_mut().enumerate() {
                *source = slot.stub_buses[port].as_ref();
            }
            for (port, source) in slot.call_data.bus_out.iter_mut().enumerate() {
                *source = slot.stub_buses[input_count + port].as_ref();
            }
        }

        // Resolve one bus per connected source port. A source feeding the
        // engine output writes into `out_bus` itself (its other sinks then
        // read `out_bus` too); anything else gets a fresh shared bus.
        let mut new_buses: Vec<Box<Bus>> = Vec::new();
        let mut source_buses: HashMap<Source, *const Bus> = HashMap::new();
        for cable in &self.cables {
            if matches!(cable.source, Source::Module { .. }) && cable.sink == Sink::OutputBus {
                source_buses.insert(cable.source, self.out_bus.as_ref() as *const Bus);
            }
        }
        for cable in &self.cables {
            source_buses.entry(cable.source).or_insert_with(|| match cable.source {
                Source::InputBus => self.in_bus.as_ref() as *const Bus,
                Source::Module { id, port } => {
                    let voices = self.bus_modules[&id].module.info().output_ports[port].max_voices;
                    new_buses.push(Box::new(Bus::new(voices, self.frames_per_buffer)));
                    new_buses.last().unwrap().as_ref() as *const Bus
                }
            });
        }

        for cable in &self.cables {
            let bus_ptr = source_buses[&cable.source];
            if let Source::Module { id, port } = cable.source {
                self.bus_modules.get_mut(&id).unwrap().call_data.bus_out[port] = bus_ptr;
            }
            if let Sink::Module { id, port } = cable.sink {
                self.bus_modules.get_mut(&id).unwrap().call_data.bus_in[port] = bus_ptr;
            }
        }

        self.buses = new_buses;
        self.execution_order = order;
        Ok(())
    }

    /// Kahn's algorithm, level by level: each level holds modules whose
    /// inputs are all satisfied by previous levels, so modules within one
    /// level are independent and may run in parallel.
    fn topo_levels(&self) -> Result<Vec<Vec<BusModuleID>>, String> {
        let mut in_degree: HashMap<BusModuleID, usize> =
            self.bus_modules.keys().map(|id| (*id, 0)).collect();
        let mut dependents: HashMap<BusModuleID, Vec<BusModuleID>> = HashMap::new();

        for cable in &self.cables {
            if let (Source::Module { id: src, .. }, Sink::Module { id: dst, .. }) =
                (cable.source, cable.sink)
            {
                dependents.entry(src).or_default().push(dst);
                *in_degree.entry(dst).or_insert(0) += 1;
            }
        }

        let mut current: Vec<BusModuleID> = in_degree
            .iter()
            .filter(|&(_, &degree)| degree == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut ordered = 0;
        let mut levels = Vec::new();
        while !current.is_empty() {
            let mut next = Vec::new();
            for id in &current {
                if let Some(dsts) = dependents.get(id) {
                    for &dst in dsts {
                        let degree = in_degree.get_mut(&dst).unwrap();
                        *degree -= 1;
                        if *degree == 0 {
                            next.push(dst);
                        }
                    }
                }
            }
            ordered += current.len();
            levels.push(mem::replace(&mut current, next));
        }

        if ordered != self.bus_modules.len() {
            return Err("module graph contains a cycle".to_string());
        }
        Ok(levels)
    }

    /// Runs one block through the module graph: every module in
    /// `execution_order`, in order, gets its prebuilt `call_data` transmuted
    /// into the slices `BusModule::process` expects. No allocations, no
    /// pointer resolution, no copying - a source cabled to the engine output
    /// has already written into `out_bus` directly by the time the graph
    /// finishes.
    fn process(&mut self) {
        for level in 0..self.execution_order.len() {
            for i in 0..self.execution_order[level].len() {
                let id = self.execution_order[level][i];
                // SAFETY: `execution_order` is rebuilt in lockstep with
                // `bus_modules` on every graph edit, so every id in it is
                // present in the map.
                let slot = unsafe { self.bus_modules.get_mut(&id).unwrap_unchecked() };
                let BusModuleSlot { module, call_data, .. } = slot;

                // SAFETY: `*const T` and `&T`/`&mut T` are layout-identical,
                // so transmuting the slices only reinterprets the element
                // type. The pointers themselves are valid: `rebuild_wiring`
                // refreshed them after the last graph edit, and every target
                // (a stub, a shared bus in `self.buses`, `in_bus`/`out_bus`)
                // is alive for as long as the wiring that references it. The
                // `&mut` slices never alias anything else in the same call:
                // each connected source port owns a distinct shared bus (or
                // `out_bus`, whose sink accepts only one cable), stubs are
                // per-port, and an out pointer can't coincide with one of
                // this module's own in pointers because `add_bus_cable`
                // rejects cycles. Writes-before-reads across modules is
                // guaranteed by the topological execution order.
                unsafe {
                    let bus_in: &[&Bus] = mem::transmute(call_data.bus_in.as_slice());
                    let events_in: &mut [&mut EventCable] =
                        mem::transmute(call_data.events_in.as_mut_slice());
                    let bus_out: &mut [&mut Bus] = mem::transmute(call_data.bus_out.as_mut_slice());
                    let events_out: &mut [&mut EventCable] =
                        mem::transmute(call_data.events_out.as_mut_slice());
                    module.process(bus_in, events_in, bus_out, events_out);
                }
            }
        }
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
