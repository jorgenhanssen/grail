use crate::args::{Args, Engines};
use crate::nnue::resolve_nnue;
use evaluation::hce;
pub use search::Engine;
use search::EngineConfig;
pub use search::NegamaxEngine;

pub fn create(args: &Args, config: &EngineConfig) -> impl Engine {
    match args.engines.as_ref().unwrap_or(&Engines::Negamax {}) {
        Engines::Negamax {} => {
            let hce = Box::new(hce::Evaluator::new(
                config.get_piece_values(),
                config.get_hce_config(),
            ));
            let nnue = resolve_nnue().expect("Failed to resolve NNUE");

            NegamaxEngine::new(config, hce, nnue)
        }
    }
}
