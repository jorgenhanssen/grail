use chess::{Board, ChessMove, MoveGen, Piece};
use evaluation::scores::MATE_VALUE;

use crate::utils::is_zugzwang;

pub const RAZOR_NEAR_MATE: i16 = MATE_VALUE - 200;

// Scores above this threshold are considered mate scores requiring ply normalization
pub const MATE_SCORE_BOUND: i16 = MATE_VALUE - 1000;

#[inline(always)]
pub fn razor_margin(depth: u8, base_margin: i16, depth_coefficient: i16) -> i16 {
    if depth == 0 {
        0
    } else {
        base_margin + depth_coefficient * (depth as i16 * depth as i16)
    }
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
    mv: ChessMove,
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
    let is_capture = board.piece_on(mv.get_dest()).is_some();
    let is_promotion = mv.get_promotion() == Some(Piece::Queen);

    if in_check || is_pv_node || is_capture || is_promotion || remaining_depth > max_depth {
        return false;
    }

    let mut limit = lmp_move_limit(remaining_depth, base_moves, depth_multiplier);

    // Be more aggressive (prune earlier) when position isn't improving
    if !is_improving {
        limit = (limit * improving_reduction) / 100;
    }

    move_index > limit
}

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

pub fn can_delta_prune(in_check: bool, material_threshold: i16, total_material: i16) -> bool {
    !in_check && total_material >= material_threshold
}

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

#[inline(always)]
pub fn can_razor_prune(remaining_depth: u8, in_check: bool, max_depth: u8) -> bool {
    remaining_depth <= max_depth && remaining_depth > 0 && !in_check
}

#[inline(always)]
pub fn futility_margin(depth: u8, base_margin: i16, depth_multiplier: i16) -> i16 {
    if depth == 0 {
        0
    } else {
        base_margin + (depth as i16 - 1) * depth_multiplier
    }
}

#[inline(always)]
pub fn can_futility_prune(remaining_depth: u8, in_check: bool, max_depth: u8) -> bool {
    remaining_depth <= max_depth && !in_check
}

// Reverse Futility Pruning (static beta pruning)

#[inline(always)]
pub fn rfp_margin(
    depth: u8,
    base_margin: i16,
    depth_multiplier: i16,
    is_improving: bool,
    improving_bonus: i16,
) -> i16 {
    let margin = if depth == 0 {
        0
    } else {
        base_margin + (depth as i16 - 1) * depth_multiplier
    };

    if is_improving {
        margin - improving_bonus
    } else {
        margin
    }
}

#[inline(always)]
pub fn can_reverse_futility_prune(
    remaining_depth: u8,
    in_check: bool,
    is_pv_node: bool,
    max_depth: u8,
) -> bool {
    remaining_depth <= max_depth && remaining_depth > 0 && !in_check && !is_pv_node
}

#[inline(always)]
pub fn only_move(board: &Board) -> bool {
    let mut g = MoveGen::new_legal(board);
    matches!((g.next(), g.next()), (Some(_), None))
}

// Small margin to avoid evaluation noise affecting position improvement detection
const IMPROVING_MARGIN: i16 = 20;

#[inline(always)]
pub fn improving(eval: i16, eval_stack: &[i16]) -> bool {
    // Eval comparison with small margin to filter noise: current eval vs eval from 2 plies back
    if eval_stack.len() < 2 {
        return false;
    }

    let prev_move_eval = eval_stack[eval_stack.len() - 2];
    eval > prev_move_eval - IMPROVING_MARGIN
}

// Mate distance pruning (MDP)
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
