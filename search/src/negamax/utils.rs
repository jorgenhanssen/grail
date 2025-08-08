use chess::{Board, ChessMove};
use evaluation::{scores::MATE_VALUE, total_material};

use crate::utils::is_zugzwang;

pub const RAZOR_MAX_DEPTH: u8 = 3;
pub const RAZOR_NEAR_MATE: i16 = MATE_VALUE - 200;

pub enum MatePrune {
    Proceed { next_alpha: i16, next_beta: i16 },
    Prune { value: i16 },
}

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

#[inline(always)]
pub fn lmr(remaining_depth: u8, tactical: bool, move_index: i32) -> u8 {
    if tactical || remaining_depth < 3 || move_index < 3 {
        return 0;
    }

    let depth_factor = (remaining_depth as f32).ln();
    let move_factor = (move_index as f32).ln();

    let reduction = (depth_factor * move_factor / 2.5).round() as u8;

    // Clamp between 0 and half the remaining depth
    let half_depth = (remaining_depth / 2).max(1);
    reduction.min(half_depth)
}

pub fn can_delta_prune(board: &Board, in_check: bool, phase: f32) -> bool {
    !in_check && total_material(board, phase) >= 1500
}

#[inline(always)]
pub fn can_null_move_prune(board: &Board, remaining_depth: u8, in_check: bool) -> bool {
    remaining_depth >= 3 && !in_check && !is_zugzwang(board)
}

#[inline(always)]
pub fn can_razor_prune(remaining_depth: u8, in_check: bool) -> bool {
    remaining_depth <= RAZOR_MAX_DEPTH && remaining_depth > 0 && !in_check
}

pub const FUTILITY_MAX_DEPTH: u8 = 3;
pub const FUTILITY_MARGINS: [i16; FUTILITY_MAX_DEPTH as usize + 1] = [0, 200, 300, 500];

#[inline(always)]
pub fn can_futility_prune(remaining_depth: u8, in_check: bool) -> bool {
    remaining_depth <= FUTILITY_MAX_DEPTH && !in_check
}

// History Leaf Pruning parameters
// History scores range roughly in [-16384, 16384]
pub const HISTORY_REDUCE_THRESHOLD: i16 = 0; // reduce quiet late moves if history <= 0
pub const HISTORY_LEAF_THRESHOLD: i16 = -1000; // prune quiet late moves at leaf if history very low
pub const HISTORY_MOVE_GATE: i32 = 5; // only consider after some moves have been tried

#[inline(always)]
pub fn can_history_leaf_prune(
    remaining_depth: u8,
    in_check: bool,
    is_tactical: bool,
    is_pv_move: bool,
    move_index: i32,
) -> bool {
    remaining_depth > 0
        && !in_check
        && !is_tactical
        && !is_pv_move
        && move_index >= HISTORY_MOVE_GATE
}
