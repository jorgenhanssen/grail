#![feature(portable_simd)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod encoding;
pub mod evaluator;
pub mod network;

pub use evaluator::Evaluator;
