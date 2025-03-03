#![feature(portable_simd)]

pub mod encoding;
pub mod evaluator;
pub mod network;
pub mod samples;
pub mod version;

pub use evaluator::NNUE;

#[cfg(test)]
mod tests {
    mod network_tests;
}
