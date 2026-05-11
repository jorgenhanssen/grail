use std::sync::{Arc, atomic::AtomicBool};

use config::EngineConfig;
use search::Engine;

use crate::nnue::resolve_nnue;

pub fn create_engine(config: &EngineConfig, stop: Arc<AtomicBool>) -> Engine {
    Engine::new(config, stop, || {
        resolve_nnue().expect("Failed to load NNUE model")
    })
}
