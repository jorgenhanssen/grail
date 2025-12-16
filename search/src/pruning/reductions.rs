use cozy_chess::{Board, Move, Piece};
use utils::is_capture;

/// Internal Iterative Reductions: reduce depth when no TT move is found.
///
/// When no hash move is found, reduce the search depth instead of doing a
/// full-depth search with poor move ordering. This is simpler and cheaper
/// than IID (which runs a shallow search to find a move).
///
/// TODO: Consider applying only at expected cut-nodes like Stockfish/Ethereal.
///
/// <https://www.chessprogramming.org/Internal_Iterative_Reductions>
pub fn iir(
    max_depth: u8,
    remaining_depth: u8,
    has_tt_move: bool,
    min_depth: u8,
    reduction: u8,
) -> (u8, u8) {
    if !has_tt_move && remaining_depth >= min_depth {
        (
            max_depth.saturating_sub(reduction),
            remaining_depth.saturating_sub(reduction),
        )
    } else {
        (max_depth, remaining_depth)
    }
}

/// Late Move Reductions: reduce depth for late quiet moves.
/// Reduction based on ln(depth) * ln(move_index).
///
/// <https://www.chessprogramming.org/Late_Move_Reductions>
#[allow(clippy::too_many_arguments)]
pub fn lmr(
    remaining_depth: u8,
    tactical: bool,
    move_index: i32,
    is_pv_move: bool,
    is_improving: bool,
    min_depth: u8,
    divisor: f32,
    max_reduction_ratio: f32,
) -> u8 {
    if tactical || remaining_depth < min_depth || is_pv_move {
        return 0;
    }

    let depth_factor = (remaining_depth as f32).ln();
    let move_factor = (move_index as f32).ln();

    let mut reduction = (depth_factor * move_factor / divisor).round() as u8;

    if !is_improving {
        reduction = reduction.saturating_add(1);
    }

    let max_reduction = (remaining_depth as f32 * max_reduction_ratio) as u8;
    reduction.min(max_reduction)
}

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
    board: &Board,
    mv: Move,
    in_check: bool,
    is_pv_node: bool,
    remaining_depth: u8,
    move_index: i32,
    is_improving: bool,
    max_depth: u8,
    base_moves: i32,
    depth_multiplier: i32,
    improving_reduction: i32,
) -> bool {
    let is_cap = is_capture(board, mv);
    let is_promotion = mv.promotion == Some(Piece::Queen);

    if in_check || is_pv_node || is_cap || is_promotion || remaining_depth > max_depth {
        return false;
    }

    let mut limit = lmp_move_limit(remaining_depth, base_moves, depth_multiplier);

    // Be more aggressive (prune earlier) when position isn't improving
    if !is_improving {
        limit = (limit * improving_reduction) / 100;
    }

    move_index > limit
}
