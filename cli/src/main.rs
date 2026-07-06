use std::thread;
use std::time::Duration;

use engine::{CpalBackend, Engine};

fn main() {
    // let mut engine = Engine::new(Box::new(CpalBackend::new()));
    // engine.start();
    thread::sleep(Duration::from_secs(3));
}
