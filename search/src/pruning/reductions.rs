use cozy_chess::{Board, Move, Piece};
use utils::is_capture;

// Late Move Reduction (LMR)
// Reduces search depth for moves that are likely to be bad (searched late in move ordering)
#[allow(clippy::too_many_arguments)]
#[inline(always)]
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

// Late Move Pruning (LMP)
// Conservative triangular limits per remaining depth for how many quiet moves
// are searched before pruning subsequent quiets in non-PV, non-check nodes.
#[inline(always)]
pub fn lmp_move_limit(depth: u8, base_moves: i32, depth_multiplier: i32) -> i32 {
    // Triangular number pattern: base + depth * (depth + multiplier) / 2
    base_moves + (depth as i32 * (depth as i32 + depth_multiplier)) / 2
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
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
