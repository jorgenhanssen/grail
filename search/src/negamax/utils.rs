use chess::{Board, ChessMove, MoveGen, Piece};
use evaluation::{scores::MATE_VALUE, total_material};

use crate::utils::is_zugzwang;

pub const RAZOR_MAX_DEPTH: u8 = 3;
pub const RAZOR_NEAR_MATE: i16 = MATE_VALUE - 200;

// Scores above this threshold are considered mate scores requiring ply normalization
pub const MATE_SCORE_BOUND: i16 = MATE_VALUE - 1000;

// Margins from Stockfish: https://www.chessprogramming.org/Razoring#Stockfish
pub const RAZOR_MARGINS: [i16; RAZOR_MAX_DEPTH as usize + 1] = {
    let mut margins = [0i16; RAZOR_MAX_DEPTH as usize + 1];
    let mut depth = 1;
    while depth <= RAZOR_MAX_DEPTH as i16 {
        margins[depth as usize] = 512 + 293 * (depth * depth);
        depth += 1;
    }
    margins
};

// Late Move Pruning (LMP)
// Conservative triangular limits per remaining depth for how many quiet moves
// are searched before pruning subsequent quiets in non-PV, non-check nodes.
pub const LMP_MAX_DEPTH: u8 = 8;
pub const LMP_LIMITS: [i32; LMP_MAX_DEPTH as usize + 1] = [0, 3, 6, 10, 15, 21, 28, 36, 45];

#[inline(always)]
pub fn should_lmp_prune(
    board: &Board,
    mv: ChessMove,
    in_check: bool,
    is_pv_node: bool,
    remaining_depth: u8,
    move_index: i32,
    is_improving: bool,
) -> bool {
    let is_capture = board.piece_on(mv.get_dest()).is_some();
    let is_promotion = mv.get_promotion() == Some(Piece::Queen);

    if in_check || is_pv_node || is_capture || is_promotion || remaining_depth > LMP_MAX_DEPTH {
        return false;
    }

    let mut limit = LMP_LIMITS[remaining_depth as usize];

    // Be more aggressive (prune earlier) when position isn't improving
    // Use conservative 15% reduction to avoid being too aggressive
    if !is_improving {
        limit = (limit * 85) / 100;
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

pub fn can_delta_prune(board: &Board, in_check: bool, phase: f32) -> bool {
    !in_check && total_material(board, phase) >= 1500
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
pub fn can_razor_prune(remaining_depth: u8, in_check: bool) -> bool {
    remaining_depth <= RAZOR_MAX_DEPTH && remaining_depth > 0 && !in_check
}

pub const FUTILITY_MAX_DEPTH: u8 = 3;
pub const FUTILITY_MARGINS: [i16; FUTILITY_MAX_DEPTH as usize + 1] = [0, 150, 250, 400];

#[inline(always)]
pub fn can_futility_prune(remaining_depth: u8, in_check: bool) -> bool {
    remaining_depth <= FUTILITY_MAX_DEPTH && !in_check
}

// Reverse Futility Pruning (static beta pruning)
pub const RFP_MAX_DEPTH: u8 = 3;
pub const RFP_MARGINS: [i16; RFP_MAX_DEPTH as usize + 1] = [0, 150, 250, 400];

#[inline(always)]
pub fn can_reverse_futility_prune(remaining_depth: u8, in_check: bool, is_pv_node: bool) -> bool {
    remaining_depth <= RFP_MAX_DEPTH && remaining_depth > 0 && !in_check && !is_pv_node
}

#[inline(always)]
pub fn rfp_margin(remaining_depth: u8, is_improving: bool) -> i16 {
    let base_margin = RFP_MARGINS[remaining_depth as usize];
    if is_improving {
        base_margin - 50 // Smaller margin for improving positions (more conservative than original -60)
    } else {
        base_margin
    }
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
