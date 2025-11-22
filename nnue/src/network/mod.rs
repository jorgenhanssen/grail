pub mod accumulator;
pub mod inference;
pub mod linear;
pub mod model;
pub mod simd;

pub use inference::NNUENetwork;
pub use linear::LinearLayer;
pub use model::Network;

pub const EMBEDDING_SIZE: usize = 1024;
pub const HIDDEN_SIZE: usize = 16;

pub const CP_MAX: i16 = 5000;
pub const CP_MIN: i16 = -5000;
pub const FV_SCALE: f32 = 400.0;

pub const QUANTIZATION_PERCENTILE: f32 = 0.999;
