use evaluation::scores::MATE_VALUE;

// Scores above this threshold are considered mate scores requiring ply normalization
pub const MATE_SCORE_BOUND: i16 = MATE_VALUE - 1000;

// Mate Distance Pruning (MDP)
//
// Adjusts alpha-beta bounds based on the maximum possible mate score at current depth.
// Returns true if the search can be pruned immediately.
//
// Example: A mate found at depth D is at least D plies from root, so:
// - Best possible score: MATE_VALUE - depth (mate-in-D)
// - Worst possible score: -(MATE_VALUE - depth) (mated-in-D)
#[inline(always)]
pub fn mate_distance_prune(alpha: &mut i16, beta: &mut i16, depth: u8) -> bool {
    let mate_in_depth = MATE_VALUE - depth as i16;
    let mated_in_depth = -(MATE_VALUE - depth as i16);

    *alpha = (*alpha).max(mated_in_depth);
    *beta = (*beta).min(mate_in_depth);

    *alpha >= *beta
}

