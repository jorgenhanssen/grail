mod def;
mod minimax;

use crate::args::{Args, Engines};
pub use def::Engine;
pub use minimax::MinimaxEngine;

pub fn create(args: &Args) -> impl Engine {
    match args.engines {
        Engines::Minimax {} => MinimaxEngine::default(),
    }
}
