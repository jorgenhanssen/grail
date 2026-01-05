use cozy_chess::{Move, Piece};

use utils::Node;

/// Move limit for LMP: few moves near the horizon, growing toward the root.
fn lmp_move_limit(depth: u8, base_moves: i32, depth_multiplier: i32) -> i32 {
    base_moves + (depth as i32 * (depth as i32 + depth_multiplier)) / 2
}

/// Late Move Pruning: at the horizon, focus only on the best-ordered quiet moves.
/// As iterative deepening extends the horizon, nodes that were at the frontier open up
/// to search more moves. This forms a right-triangle search shape, narrow tip at the
/// current horizon, widening toward the root.
///
/// <https://www.chessprogramming.org/Futility_Pruning#MoveCountBasedPruning>
#[allow(clippy::too_many_arguments)]
pub fn should_lmp_prune(
    node: &Node,
    mv: Move,
    in_check: bool,
    remaining_depth: u8,
    move_index: i32,
    is_improving: bool,
    max_depth: u8,
    base_moves: i32,
    depth_multiplier: i32,
    improving_reduction: i32,
) -> bool {
    let is_cap = node.is_capture(mv);
    let is_promotion = mv.promotion == Some(Piece::Queen);

    if in_check || node.is_pv() || is_cap || is_promotion || remaining_depth > max_depth {
        return false;
    }

    let mut limit = lmp_move_limit(remaining_depth, base_moves, depth_multiplier);

    // Be more aggressive (prune earlier) when position isn't improving
    if !is_improving {
        limit = (limit * improving_reduction) / 100;
    }

    move_index > limit
}
