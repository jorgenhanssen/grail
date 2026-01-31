use cozy_chess::{Board, Color, Move, Square};
use evaluation::scores::MATE_VALUE;
use utils::{
    generate_zobrist_table, is_capture, zobrist_key, MINOR_PIECES, NON_PAWN_PIECES, PAWN_PIECES,
};

use super::utils::apply_gravity;
use crate::EngineConfig;

// Zobrist tables for correction history indexing.
const PAWN_ZOBRIST: [u64; Square::NUM * Color::NUM * PAWN_PIECES.len()] =
    generate_zobrist_table(0xE2E4_D7D5_E4D5_D8D5);
const MINOR_ZOBRIST: [u64; Square::NUM * Color::NUM * MINOR_PIECES.len()] =
    generate_zobrist_table(0xE2E4_C7C5_D2D4_C5D4);
const NON_PAWN_ZOBRIST: [u64; Square::NUM * NON_PAWN_PIECES.len()] =
    generate_zobrist_table(0xD2D4_D7D5_C2C4_E7E6);

/// Static evaluation correction history.
///
/// Records the difference between static evaluation and search scores,
/// indexed by board features. Used to adjust future static evaluations
/// in positions with similar features.
///
/// Based on Stockfish's implementation.
/// <https://www.chessprogramming.org/Static_Evaluation_Correction_History>
#[derive(Clone)]
pub struct CorrectionHistory {
    pawn_correction: Vec<i16>,
    minor_correction: Vec<i16>,
    white_nonpawn_correction: Vec<i16>,
    black_nonpawn_correction: Vec<i16>,

    /// Table size per color
    table_size: usize,

    /// Maximum absolute correction value
    max_value: i32,

    // Weights for combining corrections
    pawn_weight: i32,
    minor_weight: i32,
    nonpawn_weight: i32,
    combined_divisor: i32,

    // Weights for updating corrections
    minor_update_weight: i32,
    nonpawn_update_weight: i32,
}

impl CorrectionHistory {
    pub fn new(
        table_size: usize,
        max_value: i32,
        pawn_weight: i32,
        minor_weight: i32,
        nonpawn_weight: i32,
        combined_divisor: i32,
        minor_update_weight: i32,
        nonpawn_update_weight: i32,
    ) -> Self {
        let total_size = Color::NUM * table_size;
        Self {
            pawn_correction: vec![0; total_size],
            minor_correction: vec![0; total_size],
            white_nonpawn_correction: vec![0; total_size],
            black_nonpawn_correction: vec![0; total_size],
            table_size,
            max_value,
            pawn_weight,
            minor_weight,
            nonpawn_weight,
            combined_divisor,
            minor_update_weight,
            nonpawn_update_weight,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        let new_table_size = config.correction_table_size.value;
        if self.table_size != new_table_size {
            let total_size = Color::NUM * new_table_size;
            self.pawn_correction = vec![0; total_size];
            self.minor_correction = vec![0; total_size];
            self.white_nonpawn_correction = vec![0; total_size];
            self.black_nonpawn_correction = vec![0; total_size];
            self.table_size = new_table_size;
        }
        self.max_value = config.correction_history_max_value.value;
        self.pawn_weight = config.correction_pawn_weight.value;
        self.minor_weight = config.correction_minor_weight.value;
        self.nonpawn_weight = config.correction_nonpawn_weight.value;
        self.combined_divisor = config.correction_combined_divisor.value;
        self.minor_update_weight = config.correction_minor_update_weight.value;
        self.nonpawn_update_weight = config.correction_nonpawn_update_weight.value;
        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.table_size == config.correction_table_size.value
            && self.max_value == config.correction_history_max_value.value
            && self.pawn_weight == config.correction_pawn_weight.value
            && self.minor_weight == config.correction_minor_weight.value
            && self.nonpawn_weight == config.correction_nonpawn_weight.value
            && self.combined_divisor == config.correction_combined_divisor.value
            && self.minor_update_weight == config.correction_minor_update_weight.value
            && self.nonpawn_update_weight == config.correction_nonpawn_update_weight.value
    }

    pub fn reset(&mut self) {
        self.pawn_correction.fill(0);
        self.minor_correction.fill(0);
        self.white_nonpawn_correction.fill(0);
        self.black_nonpawn_correction.fill(0);
    }

    /// Apply correction to an eval based on pawn structure and piece positions.
    pub fn adjust(&self, board: &Board, eval: i16) -> i16 {
        let color = board.side_to_move();

        let pawn_idx = self.index(color, zobrist_key!(board, PAWN_ZOBRIST, PAWN_PIECES));
        let minor_idx = self.index(color, zobrist_key!(board, MINOR_ZOBRIST, MINOR_PIECES));
        let white_np_idx = self.index(
            color,
            zobrist_key!(board, NON_PAWN_ZOBRIST, NON_PAWN_PIECES, Color::White),
        );
        let black_np_idx = self.index(
            color,
            zobrist_key!(board, NON_PAWN_ZOBRIST, NON_PAWN_PIECES, Color::Black),
        );

        // Weighted sum of corrections.
        let pcv = self.pawn_correction[pawn_idx] as i32;
        let micv = self.minor_correction[minor_idx] as i32;
        let wnpcv = self.white_nonpawn_correction[white_np_idx] as i32;
        let bnpcv = self.black_nonpawn_correction[black_np_idx] as i32;

        let correction_value = self.pawn_weight * pcv
            + self.minor_weight * micv
            + self.nonpawn_weight * (wnpcv + bnpcv);
        let adjustment = (correction_value / self.combined_divisor) as i16;

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

        let pawn_idx = self.index(color, zobrist_key!(board, PAWN_ZOBRIST, PAWN_PIECES));
        let minor_idx = self.index(color, zobrist_key!(board, MINOR_ZOBRIST, MINOR_PIECES));
        let white_np_idx = self.index(
            color,
            zobrist_key!(board, NON_PAWN_ZOBRIST, NON_PAWN_PIECES, Color::White),
        );
        let black_np_idx = self.index(
            color,
            zobrist_key!(board, NON_PAWN_ZOBRIST, NON_PAWN_PIECES, Color::Black),
        );

        // Stockfish's formula:
        // clamp((bestValue - correctedEval) * depth / 8, -limit/4, +limit/4)
        let diff = (best_value as i32) - (corrected_eval as i32);
        let scaled_diff = (diff * depth as i32) / 8;
        let limit_quarter = self.max_value / 4;
        let bonus = scaled_diff.clamp(-limit_quarter, limit_quarter);

        // Weighted updates.
        apply_gravity(&mut self.pawn_correction[pawn_idx], bonus, self.max_value);
        apply_gravity(
            &mut self.minor_correction[minor_idx],
            bonus * self.minor_update_weight / 128,
            self.max_value,
        );
        apply_gravity(
            &mut self.white_nonpawn_correction[white_np_idx],
            bonus * self.nonpawn_update_weight / 128,
            self.max_value,
        );
        apply_gravity(
            &mut self.black_nonpawn_correction[black_np_idx],
            bonus * self.nonpawn_update_weight / 128,
            self.max_value,
        );
    }

    fn index(&self, color: Color, key: u64) -> usize {
        let color_idx = color as usize;
        let key_idx = (key as usize) % self.table_size;
        color_idx * self.table_size + key_idx
    }
}
