pub mod accumulator;
pub mod inference;
pub mod linear;
pub mod model;
pub mod simd;

use cozy_chess::Board;
pub use inference::NNUENetwork;
pub use linear::LinearLayer;
pub use model::Network;

/// Size of the accumulator that input features are embedded into.
pub const EMBEDDING_SIZE: usize = 1024;

/// Size of the eval head hidden layers (per output bucket).
pub const EVAL_HIDDEN_SIZE: usize = 16;

/// Size of the policy head hidden layer.
pub const POLICY_HIDDEN_SIZE: usize = 16;

/// Number of policy outputs (one per piece type).
pub const POLICY_OUTPUT_SIZE: usize = 6;

/// Evaluation clipping bound (centipawns). Output is clamped to [-CP_BOUND, CP_BOUND].
pub const CP_BOUND: i16 = 5000;

/// Scale factor between net and real space.
/// Training targets are divided by this, inference output is multiplied back.
/// Keeps network values in a small range for stable gradients during training.
pub const FV_SCALE: f32 = 400.0;

/// Percentile of weights to use for quantization scaling.
/// This ensures that most weights are in a reasonable range,
/// and that extreme outliers don't stretch the range and waste precision.
/// 99.9% proved a good value during testing.
pub const QUANTIZATION_PERCENTILE: f32 = 0.999;

/// Number of output buckets for game-phase-specific evaluation.
pub const OUTPUT_BUCKETS: usize = 8;

/// Compute output bucket from board position based on piece count.
/// Uses standard formula from engines like Stockfish: bucket = (pieceCount - 2) / divisor
#[inline]
pub fn output_bucket(board: &Board) -> usize {
    // Evenly distributes piece counts 2-32 across all buckets.
    const BUCKET_DIVISOR: usize = 32_usize.div_ceil(OUTPUT_BUCKETS);
    (board.occupied().len() as usize - 2) / BUCKET_DIVISOR
}
