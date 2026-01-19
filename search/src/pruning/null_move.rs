use utils::Node;

// Null Move Pruning
// Try passing the turn to the opponent. If they still can't beat beta with a free move,
// the position is likely so good we can prune this branch.
pub fn can_null_move_prune(node: &Node, depth: u8, in_check: bool, min_depth: u8) -> bool {
    !in_check && node.is_cut() && depth >= min_depth && !node.is_zugzwang()
}

pub fn null_move_reduction(
    depth: u8,
    static_eval: Option<i16>,
    beta: i16,
    base_reduction: u8,
    depth_divisor: u8,
    eval_margin: i16,
) -> u8 {
    // Deeper positions get more reduction
    let mut r = base_reduction + (depth / depth_divisor);

    if let Some(se) = static_eval {
        if se >= beta + eval_margin {
            // Strong positions get extra reduction
            r = r.saturating_add(1);
        } else if se <= beta - eval_margin {
            // Weak positions get less reduction
            r = r.saturating_sub(1).max(base_reduction);
        }
    }

    // Ensure reduction doesn't exceed depth
    if r >= depth {
        r = depth.saturating_sub(1).max(base_reduction);
    }

    r
}
