use crate::{EventCable, TimedEvent};

pub struct PortInfo {
    pub name: String,
    pub max_voices: usize,
}

pub struct EventPortInfo {
    pub name: String,
}

pub type ModuleID = String;

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
        ports_in: &[&[f32]],
        events_in: &[TimedEvent],
        ports_out: &[&mut [f32]],
        events_out: &mut [EventCable],
    );
}
