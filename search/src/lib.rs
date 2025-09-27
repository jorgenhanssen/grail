#![feature(portable_simd)]

mod config;
mod def;
mod negamax;
mod utils;

pub use config::EngineConfig;
pub use def::Engine;
pub use negamax::NegamaxEngine;
