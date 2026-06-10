mod continuation;
mod position;

use cozy_chess::{Board, Move};
use utils::is_capture;

use config::EngineConfig;

use crate::history::PieceTo;
use crate::scores::{MATE_SCORE_BOUND, MATE_VALUE};

use continuation::ContinuationCorrection;
use position::PositionCorrection;

/// Static evaluation correction.
///
/// Tracks how far off the static eval is from what the search actually
/// returns. Used to adjust the static eval next time we hit a similar position.
///
/// Based on Stockfish.
/// <https://www.chessprogramming.org/Static_Evaluation_Correction_History>
#[derive(Clone)]
pub struct Correction {
    position: PositionCorrection,
    continuation: ContinuationCorrection,

    combined_divisor: i32,
    max_correction: i32,
}

impl Correction {
    pub fn new(config: &EngineConfig) -> Self {
        Self {
            position: PositionCorrection::new(config),
            continuation: ContinuationCorrection::new(config),
            combined_divisor: config.correction_combined_divisor.value,
            max_correction: config.correction_history_max_correction.value,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.position.configure(config);
        self.continuation.configure(config);
        self.combined_divisor = config.correction_combined_divisor.value;
        self.max_correction = config.correction_history_max_correction.value;
        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.position.matches_config(config)
            && self.continuation.matches_config(config)
            && self.combined_divisor == config.correction_combined_divisor.value
            && self.max_correction == config.correction_history_max_correction.value
    }

    pub fn reset(&mut self) {
        self.position.reset();
        self.continuation.reset();
    }

    /// Returns the static eval adjusted by the correction history.
    pub fn adjust(&self, board: &Board, prev_moves: &[Option<PieceTo>], eval: i16) -> i16 {
        let total =
            self.position.weighted_value(board) + self.continuation.weighted_value(prev_moves);
        let adjustment = (total / self.combined_divisor) as i16;

        eval.saturating_add(adjustment)
            .clamp(-(MATE_VALUE - 100), MATE_VALUE - 100)
    }

    /// Update correction history after a search completes.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        board: &Board,
        prev_moves: &[Option<PieceTo>],
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

        if best_value.abs() >= MATE_SCORE_BOUND {
            return;
        }

        // Don't update if the search and the eval disagree on the direction.
        if best_value >= beta && best_value < corrected_eval {
            return;
        }
        if best_value <= alpha && best_value > corrected_eval {
            return;
        }

        // Stockfish bonus: clamp((best_value - corrected_eval) * depth / 8, +- limit/4).
        let diff = (best_value as i32) - (corrected_eval as i32);
        let scaled_diff = (diff * depth as i32) / 8;
        let limit_quarter = self.max_correction / 4;
        let bonus = scaled_diff.clamp(-limit_quarter, limit_quarter);

        self.position.apply_bonus(board, bonus, self.max_correction);
        self.continuation
            .apply_bonus(prev_moves, bonus, self.max_correction);
    }
}
