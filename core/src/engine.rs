use std::sync::{Arc, atomic::AtomicBool};

use crate::nnue::resolve_nnue;
use search::Engine;
use search::EngineConfig;

pub fn create_engine(config: &EngineConfig, stop: Arc<AtomicBool>) -> Engine {
    let hce_config = config.get_hce_config();
    Engine::new(config, stop, move || {
        let hce = Box::new(hce::Evaluator::new(hce_config)) as Box<dyn evaluation::HCE>;
        let nnue =
            Some(resolve_nnue().expect("Failed to load NNUE model") as Box<dyn evaluation::NNUE>);
        (hce, nnue)
    })
}
