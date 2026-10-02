pub mod encoding;
pub mod evaluator;
pub mod kernels;
pub mod network;

pub use evaluator::Evaluator;

pub const MODEL_PATH: &str = "nnue/model.safetensors";

pub(crate) use utils::bitset;
