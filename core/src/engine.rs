use crate::args::{Args, Engines};
use nnue::network::Network;
use nnue::NNUE;
pub use search::Engine;
pub use search::MinimaxEngine;

pub fn create(args: &Args) -> impl Engine {
    match args.engines {
        Engines::Minimax {} => {
            let nnue = NNUE::new();
            MinimaxEngine::new(Box::new(nnue))
        }
    }
}
