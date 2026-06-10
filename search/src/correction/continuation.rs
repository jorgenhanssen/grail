use config::EngineConfig;

use crate::history::{PieceTo, PrevMoves, apply_gravity};

/// Correction indexed by move continuations.
#[derive(Clone)]
pub(super) struct ContinuationCorrection {
    // [lookback][PieceTo].
    table: Vec<i16>,

    max_moves: usize,
    weight: i32,
    update_weight: i32,
}

impl ContinuationCorrection {
    pub(super) fn new(config: &EngineConfig) -> Self {
        let max_moves = config.correction_continuation_max_moves.value;
        Self {
            table: vec![0; Self::table_size(max_moves)],
            max_moves,
            weight: config.correction_continuation_weight.value,
            update_weight: config.correction_continuation_update_weight.value,
        }
    }

    pub(super) fn configure(&mut self, config: &EngineConfig) {
        let new_max_moves = config.correction_continuation_max_moves.value;
        if self.max_moves != new_max_moves {
            self.table = vec![0; Self::table_size(new_max_moves)];
            self.max_moves = new_max_moves;
        }
        self.weight = config.correction_continuation_weight.value;
        self.update_weight = config.correction_continuation_update_weight.value;
    }

    pub(super) fn matches_config(&self, config: &EngineConfig) -> bool {
        self.max_moves == config.correction_continuation_max_moves.value
            && self.weight == config.correction_continuation_weight.value
            && self.update_weight == config.correction_continuation_update_weight.value
    }

    pub(super) fn reset(&mut self) {
        self.table.fill(0);
    }

    /// Sum of the table entries for each filled lookback slot.
    pub(super) fn weighted_value(&self, prev_moves: &PrevMoves) -> i32 {
        let mut sum: i32 = 0;
        for (lookback, slot) in prev_moves.iter().enumerate().take(self.max_moves) {
            if let Some(prev_move) = *slot {
                sum += self.table[Self::index(lookback, prev_move)] as i32;
            }
        }
        self.weight * sum
    }

    /// Update the slot for each filled lookback with a scaled bonus.
    pub(super) fn apply_bonus(&mut self, prev_moves: &PrevMoves, bonus: i32, limit: i32) {
        let scaled = bonus * self.update_weight / 128;
        for (lookback, slot) in prev_moves.iter().enumerate().take(self.max_moves) {
            if let Some(prev_move) = *slot {
                let idx = Self::index(lookback, prev_move);
                apply_gravity(&mut self.table[idx], scaled, limit);
            }
        }
    }

    const fn table_size(max_moves: usize) -> usize {
        max_moves * PieceTo::SIZE
    }

    const fn index(lookback: usize, prev_move: PieceTo) -> usize {
        lookback * PieceTo::SIZE + prev_move.index()
    }
}
