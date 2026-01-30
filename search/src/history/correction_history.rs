use cozy_chess::{Board, Color, Move};
use evaluation::scores::MATE_VALUE;
use utils::is_capture;

use super::utils::apply_gravity;
use crate::EngineConfig;

/// Size of the correction history table (must be power of 2).
const CORRECTION_TABLE_SIZE: usize = 16384;

/// Static evaluation correction history.
///
/// Records the difference between static evaluation and search scores,
/// indexed by pawn structure. Used to adjust future static evaluations
/// in positions with similar pawn structures.
///
/// Based on Stockfish's implementation:
/// <https://www.chessprogramming.org/Static_Evaluation_Correction_History>
#[derive(Clone)]
pub struct CorrectionHistory {
    /// Correction values indexed by [color][pawn_key % table_size]
    pawn_correction: Vec<i16>,

    /// Maximum absolute correction value
    max_value: i32,
    /// Weight for applying correction to eval
    weight: i32,
    /// Divisor for scaling correction when applied
    divisor: i32,
}

impl CorrectionHistory {
    pub fn new(max_value: i32, weight: i32, divisor: i32) -> Self {
        Self {
            pawn_correction: vec![0; Color::NUM * CORRECTION_TABLE_SIZE],
            max_value,
            weight,
            divisor,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.max_value = config.correction_history_max_value.value;
        self.weight = config.correction_history_weight.value;
        self.divisor = config.correction_history_divisor.value;
        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.max_value == config.correction_history_max_value.value
            && self.weight == config.correction_history_weight.value
            && self.divisor == config.correction_history_divisor.value
    }

    pub fn reset(&mut self) {
        self.pawn_correction.fill(0);
    }

    /// Apply correction to an eval based on pawn structure.
    pub fn adjust(&self, board: &Board, eval: i16) -> i16 {
        let color = board.side_to_move();
        let pawn_key = utils::pawn_key(board);
        let idx = Self::index(color, pawn_key);

        let correction = self.pawn_correction[idx] as i32;
        let adjustment = ((self.weight * correction) / self.divisor) as i16;

        eval.saturating_add(adjustment)
            .clamp(-(MATE_VALUE - 100), MATE_VALUE - 100)
    }

    /// Update correction history after a search completes.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        board: &Board,
        in_check: bool,
        best_move: Option<Move>,
        best_value: i16,
        corrected_eval: i16,
        alpha: i16,
        beta: i16,
        depth: u8,
    ) {
        if in_check {
            return;
        }
        if let Some(mv) = best_move {
            if is_capture(board, mv) || mv.promotion.is_some() {
                return;
            }
        }

        // Check score/eval consistency based on bound type
        // Lower bound (fail high): score should not be below corrected eval
        // Upper bound (fail low): score should not be above corrected eval
        let is_lower_bound = best_value >= beta;
        let is_upper_bound = best_value <= alpha;

        if is_lower_bound && best_value < corrected_eval {
            return;
        }
        if is_upper_bound && best_value > corrected_eval {
            return;
        }

        let color = board.side_to_move();
        let pawn_key = utils::pawn_key(board);
        let idx = Self::index(color, pawn_key);

        // Stockfish's formula:
        // clamp((bestValue - correctedEval) * depth / 8, -limit/4, +limit/4)
        let diff = (best_value as i32) - (corrected_eval as i32);
        let scaled_diff = (diff * depth as i32) / 8;
        let limit_quarter = self.max_value / 4;
        let bonus = scaled_diff.clamp(-limit_quarter, limit_quarter);

        apply_gravity(&mut self.pawn_correction[idx], bonus, self.max_value);
    }

    fn index(color: Color, pawn_key: u64) -> usize {
        let color_idx = color as usize;
        let key_idx = (pawn_key as usize) & (CORRECTION_TABLE_SIZE - 1);
        color_idx * CORRECTION_TABLE_SIZE + key_idx
    }
}
