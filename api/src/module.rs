use crate::{EventCable, Port};

pub struct PortInfo {
    max_voices: i32,
    name: String,
}

pub struct EventPortInfo {
    name: String,
}

pub struct ModuleInfo {
    name: String,
    input_ports: Vec<PortInfo>,
    input_event_ports: Vec<EventPortInfo>,
    output_ports: Vec<PortInfo>,
    output_event_ports: Vec<EventPortInfo>,
}

pub struct GlobalCommand {
    pub command: String,
}

pub trait Module {
    fn process(
        &mut self,
        ports_in: &[Port],
        events_in: &[EventCable],
        ports_out: &mut [Port],
        events_out: &[EventCable],
    );

    fn info(&self) -> &ModuleInfo;

    fn handle_command(&mut self, command: GlobalCommand);
}
