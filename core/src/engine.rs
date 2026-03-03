use std::sync::{Arc, atomic::AtomicBool};

use crate::nnue::resolve_nnue;
use search::Engine;
use search::EngineConfig;

pub fn create_engine(config: &EngineConfig, stop: Arc<AtomicBool>) -> Engine {
    Engine::new(config, stop, || {
        resolve_nnue().expect("Failed to load NNUE model")
    })
}
