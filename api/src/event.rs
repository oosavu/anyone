pub enum EventType {
    On,
    Off,
    Trig,
    Float(f32),
    Integer(i32),
    Data(String),
}

pub struct Event {
    pub kind: EventType,
    pub index: i32,
}

pub struct TimedEvent {
    pub sample: u32,
    pub event: Event,
}

pub struct EventCable {
    events: Vec<TimedEvent>,
}

impl EventCable {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, sample: u32, event: Event) {
        self.events.push(TimedEvent { sample, event });
    }

    pub fn events(&self) -> &[TimedEvent] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for EventCable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_read() {
        let mut cable = EventCable::new();
        cable.push(
            0,
            Event {
                kind: EventType::On,
                index: 3,
            },
        );
        cable.push(
            64,
            Event {
                kind: EventType::Float(0.5),
                index: 3,
            },
        );

        let events = cable.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sample, 0);
        assert_eq!(events[1].sample, 64);
    }

    #[test]
    fn clear_empties_events() {
        let mut cable = EventCable::new();
        cable.push(
            0,
            Event {
                kind: EventType::Trig,
                index: 0,
            },
        );
        cable.clear();
        assert!(cable.events().is_empty());
    }
}
