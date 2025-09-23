use crate::args::{Args, Engines};
use crate::nnue::resolve_nnue;
use evaluation::hce;
use evaluation::HCE;
pub use search::Engine;
use search::EngineConfig;
pub use search::NegamaxEngine;

pub fn create(args: &Args, config: &EngineConfig) -> impl Engine {
    match args.engines.as_ref().unwrap_or(&Engines::Negamax {}) {
        Engines::Negamax {} => {
            let nnue = resolve_nnue().expect("Failed to resolve NNUE");
            let hce: Box<dyn HCE> = Box::new(hce::Evaluator);

            NegamaxEngine::new(config, hce, nnue)
        }
    }
}
