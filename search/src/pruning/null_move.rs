use chess::Board;
use utils::is_zugzwang;

// Null Move Pruning
// Try passing the turn to the opponent. If they still can't beat beta with a free move,
// the position is likely so good we can prune this branch.
#[inline(always)]
pub fn can_null_move_prune(
    board: &Board,
    remaining_depth: u8,
    in_check: bool,
    min_depth: u8,
) -> bool {
    remaining_depth >= min_depth && !in_check && !is_zugzwang(board)
}

#[inline(always)]
pub fn null_move_reduction(
    base_remaining: u8,
    static_eval: Option<i16>,
    beta: i16,
    base_reduction: u8,
    depth_divisor: u8,
    eval_margin: i16,
) -> u8 {
    // Deeper positions get more reduction
    let mut r = base_reduction + (base_remaining / depth_divisor);

    if let Some(se) = static_eval {
        // Strong positions get extra reduction (more aggressive pruning)
        if se >= beta + eval_margin {
            r = r.saturating_add(1);
        // Weak positions get less reduction (more conservative)
        } else if se <= beta - eval_margin {
            r = r.saturating_sub(1).max(base_reduction);
        }
    }

    // Ensure reduction doesn't exceed remaining depth
    if r >= base_remaining {
        r = base_remaining.saturating_sub(1).max(base_reduction);
    }

    r
}

