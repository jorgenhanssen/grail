use std::path::PathBuf;

use crate::args::{Args, Engines};
use nnue::NNUE;
pub use search::Engine;
pub use search::MinimaxEngine;

pub fn create(args: &Args) -> impl Engine {
    match args.engines {
        Engines::Minimax {} => {
            // TODO: Fix
            let path = PathBuf::from("nnue/versions/v0/model.bin");
            let nnue = NNUE::new(path);
            MinimaxEngine::new(Box::new(nnue))
        }
    }
}
