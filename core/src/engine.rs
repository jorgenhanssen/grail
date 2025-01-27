use crate::args::{Args, Engines};
pub use search::Engine;
pub use search::MinimaxEngine;

pub fn create(args: &Args) -> impl Engine {
    match args.engines {
        Engines::Minimax {} => MinimaxEngine::default(),
    }
}
