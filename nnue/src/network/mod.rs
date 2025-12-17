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

/// Number of output buckets for game-phase-specific evaluation.
/// Bucket is selected by piece count: bucket = (piece_count - 2) / 4
/// This gives roughly: endgame (0-1), late middle (2-3), early middle (4-5), opening (6-7)
pub const OUTPUT_BUCKETS: usize = 8;

/// Evaluation clipping bound (centipawns). Output is clamped to [-CP_BOUND, CP_BOUND].
pub const CP_BOUND: i16 = 5000;

/// Scale factor for network I/O.
/// Training targets are divided by this, inference output is multiplied back.
/// Keeps network values in a small range for stable gradients during training.
pub const FV_SCALE: f32 = 400.0;

/// Percentile of weights to use for quantization scaling.
/// This ensures that most weights are in a reasonable range,
/// and that extreme outliers don't stretch the range and waste precision.
/// 99.9% proved a good value during testing.
pub const QUANTIZATION_PERCENTILE: f32 = 0.999;

/// Compute output bucket from board position.
/// Based on piece count: bucket = (pieces - 2) / 4, clamped to [0, OUTPUT_BUCKETS-1]
/// Roughly: endgame (0-1), late middle (2-3), early middle (4-5), opening (6-7)
#[inline]
pub fn output_bucket(board: &cozy_chess::Board) -> usize {
    let piece_count = board.occupied().len() as usize;
    ((piece_count.saturating_sub(2)) / 4).min(OUTPUT_BUCKETS - 1)
}
