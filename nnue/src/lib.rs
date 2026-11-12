mod evaluator;
pub mod features;
mod inference;
mod kernels;
pub mod network;

pub use evaluator::Evaluator;

pub const MODEL_PATH: &str = "nnue/model.safetensors";

pub(crate) use utils::bitset;
