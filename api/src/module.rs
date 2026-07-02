use crate::{EventCable, Port};

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

pub trait PortModule {
    fn info(&self) -> &ModuleInfo;

    fn process(
        &mut self,
        ports_in: &[Port],
        events_in: &[EventCable],
        ports_out: &mut [Port],
        events_out: &mut [EventCable],
    );
}
