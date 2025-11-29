pub mod accumulator;
pub mod inference;
pub mod linear;
pub mod model;
pub mod simd;

pub use inference::NNUENetwork;
pub use linear::LinearLayer;
pub use model::Network;

/// Size of the accumulator that input features are embedded into.
pub const EMBEDDING_SIZE: usize = 1024;

/// Size of the hidden layers after the embedding.
pub const HIDDEN_SIZE: usize = 16;

// Evaluation clipping bounds (centipawns)
// TODO: Consider only using a single value, since they are the same.
pub const CP_MAX: i16 = 5000;
pub const CP_MIN: i16 = -5000;

/// Scale factor for network I/O.
/// Training targets are divided by this, inference output is multiplied back.
/// Keeps network values in a small range for stable gradients during training.
pub const FV_SCALE: f32 = 400.0;

/// Percentile of weights to use for quantization scaling.
/// This ensures that most weights are in a reasonable range,
/// and that extreme outliers don't stretch the range and waste precision.
/// 99.9% proved a good value during testing.
pub const QUANTIZATION_PERCENTILE: f32 = 0.999;
