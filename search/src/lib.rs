#![feature(portable_simd)]

mod config;
mod engine;
mod history;
mod move_ordering;
mod pruning;
mod stack;
mod time_control;
mod transposition;
mod utils;

pub use config::EngineConfig;
pub use engine::Engine;
