// Score bounds and special values for alpha-beta search.
pub const SCORE_INF: i16 = 30_000;
/// Base value for checkmate. Actual mate scores are MATE_VALUE - ply to distinguish faster mates.
pub const MATE_VALUE: i16 = SCORE_INF - 1000;
/// Scores above this threshold are considered mate scores requiring ply normalization.
pub const MATE_SCORE_BOUND: i16 = MATE_VALUE - 1000;
