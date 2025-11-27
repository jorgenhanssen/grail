#![feature(portable_simd)]

mod config;
pub mod engine;
mod history;
mod move_ordering;
mod pruning;
mod stack;
mod time_control;
mod transposition;
mod utils;

/// Maximum search depth supported by the engine.
pub const MAX_DEPTH: usize = 100;

pub use config::EngineConfig;
pub use engine::Engine;
