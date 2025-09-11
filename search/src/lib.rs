#![feature(portable_simd)]

mod def;
mod negamax;
mod utils;

pub use def::Engine;
pub use negamax::NegamaxEngine;
