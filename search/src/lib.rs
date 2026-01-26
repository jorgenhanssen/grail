#![feature(portable_simd)]

mod config;
pub mod engine;
mod extensions;
mod history;
mod move_ordering;
mod pruning;
mod pv;
mod reductions;
mod result;
mod stack;
mod time_control;
mod transposition;
mod utils;

/// Maximum search depth supported by the engine.
pub const MAX_DEPTH: usize = 100;

pub use config::EngineConfig;
pub use engine::Engine;
pub use pv::PvLine;
pub use result::SearchResult;

pub use ::utils::{Node, NodeType};
