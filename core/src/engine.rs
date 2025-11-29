use std::sync::{atomic::AtomicBool, Arc};

use crate::nnue::resolve_nnue;
use search::Engine;
use search::EngineConfig;

pub fn create_engine(config: &EngineConfig, stop: Arc<AtomicBool>) -> Engine {
    let hce = Box::new(hce::Evaluator::new(
        config.get_piece_values(),
        config.get_hce_config(),
    ));
    let nnue = Some(resolve_nnue().expect("Failed to load NNUE model"));

    Engine::new(config, hce, nnue, stop)
}
