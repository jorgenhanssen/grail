use evaluation::scores::MATE_VALUE;

use crate::utils::Bounds;

// Scores above this threshold are considered mate scores requiring ply normalization
pub const MATE_SCORE_BOUND: i16 = MATE_VALUE - 1000;

// Mate Distance Pruning (MDP)
//
// Adjusts alpha-beta bounds based on the maximum possible mate score at current ply.
// Returns true if the search can be pruned immediately.
//
// Example: A mate found at ply P is at least P plies from root, so:
// - Best possible score: MATE_VALUE - ply (mate-in-P)
// - Worst possible score: -(MATE_VALUE - ply) (mated-in-P)
pub fn mate_distance_prune(bounds: &mut Bounds, ply: u8) -> bool {
    let mate_in_ply = MATE_VALUE - ply as i16;
    let mated_in_ply = -(MATE_VALUE - ply as i16);

    bounds.alpha = bounds.alpha.max(mated_in_ply);
    bounds.beta = bounds.beta.min(mate_in_ply);

    bounds.alpha >= bounds.beta
}
