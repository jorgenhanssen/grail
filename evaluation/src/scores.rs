// Score bounds and special values for alpha-beta search.
// TODO: consider removing POS/NEG_INFINITY and using SCORE_INF directly
const SCORE_INF: i16 = 30_000;
pub const POS_INFINITY: i16 = SCORE_INF;
pub const NEG_INFINITY: i16 = -SCORE_INF;
/// Base value for checkmate. Actual mate scores are MATE_VALUE - ply to distinguish faster mates.
pub const MATE_VALUE: i16 = SCORE_INF - 1000;
