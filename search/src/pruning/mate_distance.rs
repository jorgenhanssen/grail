use evaluation::scores::MATE_VALUE;

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
pub fn mate_distance_prune(alpha: &mut i16, beta: &mut i16, ply: u8) -> bool {
    let mate_in_ply = MATE_VALUE - ply as i16;
    let mated_in_ply = -(MATE_VALUE - ply as i16);

    *alpha = (*alpha).max(mated_in_ply);
    *beta = (*beta).min(mate_in_ply);

    *alpha >= *beta
}
