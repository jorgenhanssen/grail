use chess::{Board, MoveGen};
use evaluation::{scores::MATE_VALUE, total_material};

use super::trend::Trend;
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

#[inline(always)]
pub fn lmr(
    remaining_depth: u8,
    tactical: bool,
    move_index: i32,
    is_pv_move: bool,
    trend: Trend,
) -> u8 {
    if tactical || remaining_depth < 3 || move_index < 3 || is_pv_move {
        return 0;
    }

    let depth_factor = (remaining_depth as f32).ln();
    let move_factor = (move_index as f32).ln();

    let mut reduction = (depth_factor * move_factor / 2.5).round() as u8;

    match trend {
        Trend::Worsening(s) => {
            reduction = reduction.saturating_add(s);
        }
        Trend::Improving(s) => {
            let strength = match s {
                1 => 1, // Small improvement: search deeper
                2 => 2, // Moderate improvement: search much deeper
                3 => 1, // Large improvement: search deeper
                _ => 0, // Crushing: spend time searching elsewhere
            };
            reduction = reduction.saturating_sub(strength);
        }
        Trend::Neutral => {}
    }

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
pub const FUTILITY_MARGINS: [i16; FUTILITY_MAX_DEPTH as usize + 1] = [0, 150, 250, 400];

#[inline(always)]
pub fn can_futility_prune(remaining_depth: u8, in_check: bool) -> bool {
    remaining_depth <= FUTILITY_MAX_DEPTH && !in_check
}

// Reverse Futility Pruning (static beta pruning)
pub const RFP_MAX_DEPTH: u8 = 3;
pub const RFP_MARGINS: [i16; RFP_MAX_DEPTH as usize + 1] = [0, 150, 250, 400];
const IMPROVING_RFP_DELTA: i16 = 25; // Add or subtract based on improving signal

#[inline(always)]
pub fn can_reverse_futility_prune(remaining_depth: u8, in_check: bool, is_pv_node: bool) -> bool {
    remaining_depth <= RFP_MAX_DEPTH && remaining_depth > 0 && !in_check && !is_pv_node
}

#[inline(always)]
pub fn rfp_margin(remaining_depth: u8, trend: Trend) -> i16 {
    let margin = RFP_MARGINS[remaining_depth as usize];
    match trend {
        Trend::Improving(s) => {
            let delta = IMPROVING_RFP_DELTA * s as i16;
            margin.saturating_sub(delta)
        }
        Trend::Worsening(s) => {
            let delta = IMPROVING_RFP_DELTA * s as i16;
            margin.saturating_add(delta)
        }
        Trend::Neutral => margin,
    }
}

#[inline(always)]
pub fn only_move(board: &Board) -> bool {
    let mut g = MoveGen::new_legal(board);
    matches!((g.next(), g.next()), (Some(_), None))
}
