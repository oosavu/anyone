use crate::bus::Bus;
use crate::port::Port;
use crate::{EventCable, TimedEvent};

pub struct PortInfo {
    pub name: String,
    pub max_voices: usize,
}

pub struct EventPortInfo {
    pub name: String,
}

pub struct ModuleInfo {
    pub name: String,
    pub input_ports: Vec<PortInfo>,
    pub output_ports: Vec<PortInfo>,
    pub input_event_ports: Vec<EventPortInfo>,
    pub output_event_ports: Vec<EventPortInfo>,
}

pub trait Module {
    fn info(&self) -> &ModuleInfo;

    fn process(
        &mut self,
        ports_in: &[&Port],
        events_in: &[TimedEvent],
        ports_out: &mut [&mut Port],
        events_out: &mut [&mut EventCable],
    );
}

pub struct BusInfo {
    pub name: String,
    pub channels: usize,
    pub frames: usize,
}

pub struct BusModuleInfo {
    pub name: String,
    pub input_ports: Vec<PortInfo>,
    pub output_ports: Vec<PortInfo>,
    pub input_event_ports: Vec<EventPortInfo>,
    pub output_event_ports: Vec<EventPortInfo>,
}

pub trait BusModule {
    fn info(&self) -> &BusModuleInfo;

    fn process(
        &mut self,
        bus_in: &[&Bus],
        events_in: &mut [&mut EventCable],
        bus_out: &mut [&mut Bus],
        events_out: &mut [&mut EventCable],
    );
}
