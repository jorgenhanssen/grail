#![feature(portable_simd)]

mod aspiration;
mod config;
mod engine;
mod history;
mod lmr;
mod move_ordering;
mod pv;
mod result;
mod scores;
pub(crate) mod searcher;
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
