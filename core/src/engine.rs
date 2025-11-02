use crate::nnue::resolve_nnue;
use evaluation::hce;
use search::EngineConfig;
use search::Engine;

pub fn create_engine(config: &EngineConfig) -> Engine {
    let hce = Box::new(hce::Evaluator::new(
        config.get_piece_values(),
        config.get_hce_config(),
    ));
    let nnue = resolve_nnue().expect("Failed to resolve NNUE");

    Engine::new(config, hce, nnue)
}
