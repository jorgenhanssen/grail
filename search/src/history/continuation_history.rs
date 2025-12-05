use cozy_chess::{Board, Color, Move, Square};

use super::utils::apply_gravity;
use crate::stack::SearchNode;
use crate::{EngineConfig, MAX_DEPTH};

/// Continuation history: scores moves based on the sequence of prior moves.
///
/// Tracks correlations like "after Nf3, playing e4 tends to be good."
/// Index 0 = opponent's last move, index 1 = our previous move, etc.
/// Helps with move ordering by learning common tactical/positional patterns.
///
/// <https://www.chessprogramming.org/Countermove_Heuristic>
#[derive(Clone)]
pub struct ContinuationHistory {
    // Flattened: [continuation_index][color][prev_to][curr_from][curr_to]
    // continuation_index 0 = opponent's last move, 1 = our previous move, etc.
    continuations: Vec<i16>,

    max_moves: usize,
    max_history: i32,
    bonus_multiplier: i32,
    malus_multiplier: i32,
}

impl ContinuationHistory {
    pub fn new(
        max_moves: usize,
        max_history: i32,
        bonus_multiplier: i32,
        malus_multiplier: i32,
    ) -> Self {
        let size = max_moves * Color::NUM * Square::NUM * Square::NUM * Square::NUM;
        Self {
            continuations: vec![0; size],
            max_moves,
            max_history,
            bonus_multiplier,
            malus_multiplier,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.max_moves = config.continuation_max_moves.value;
        self.max_history = config.continuation_max_value.value;
        self.bonus_multiplier = config.continuation_bonus_multiplier.value;
        self.malus_multiplier = config.continuation_malus_multiplier.value;

        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.max_moves == config.continuation_max_moves.value
            && self.max_history == config.continuation_max_value.value
            && self.bonus_multiplier == config.continuation_bonus_multiplier.value
            && self.malus_multiplier == config.continuation_malus_multiplier.value
    }

    pub fn reset(&mut self) {
        let size = self.max_moves * Color::NUM * Square::NUM * Square::NUM * Square::NUM;
        self.continuations = vec![0; size];
    }

    fn get_continuation(
        &self,
        continuation_index: usize,
        color: Color,
        prev_to: Option<Square>,
        from: Square,
        to: Square,
    ) -> i16 {
        if continuation_index >= self.max_moves {
            return 0;
        }
        if let Some(p_to) = prev_to {
            self.continuations[self.index(continuation_index, color, p_to, from, to)]
        } else {
            0
        }
    }

    pub fn get(&self, color: Color, prev_to: &[Option<Square>], from: Square, to: Square) -> i16 {
        let mut score = 0;
        for (continuation_index, p_to) in prev_to.iter().enumerate().take(self.max_moves) {
            score += self.get_continuation(continuation_index, color, *p_to, from, to);
        }
        score
    }

    pub fn get_bonus(&self, remaining_depth: u8) -> i32 {
        self.bonus_multiplier * remaining_depth.min(MAX_DEPTH as u8) as i32
    }

    pub fn get_malus(&self, remaining_depth: u8) -> i32 {
        -self.malus_multiplier * remaining_depth.min(MAX_DEPTH as u8) as i32
    }

    pub fn get_prev_to_squares(&self, search_stack: &[SearchNode]) -> Vec<Option<Square>> {
        let len = search_stack.len();
        let mut vec = vec![None; self.max_moves];
        for i in 0..self.max_moves {
            if i < len {
                if let Some(mv) = search_stack[len - 1 - i].last_move {
                    vec[i] = Some(mv.to);
                }
            }
        }
        vec
    }

    fn update_continuations(
        &mut self,
        color: Color,
        prev_to: &[Option<Square>],
        from: Square,
        to: Square,
        delta: i32,
    ) {
        for (continuation_index, p_to_opt) in prev_to.iter().enumerate().take(self.max_moves) {
            if let Some(p_to) = *p_to_opt {
                let idx = self.index(continuation_index, color, p_to, from, to);
                apply_gravity(&mut self.continuations[idx], delta, self.max_history);
            }
        }
    }

    pub fn update_quiet_all(
        &mut self,
        board: &Board,
        prev_to: &[Option<Square>],
        mv: Move,
        delta: i32,
    ) {
        let color = board.side_to_move();
        self.update_continuations(color, prev_to, mv.from, mv.to, delta);
    }

    fn index(
        &self,
        continuation_index: usize,
        color: Color,
        prev_to: Square,
        from: Square,
        to: Square,
    ) -> usize {
        let color_idx = color as usize;
        let prev_to_idx = prev_to as usize;
        let from_idx = from as usize;
        let to_idx = to as usize;

        let continuation_stride = Color::NUM * Square::NUM * Square::NUM * Square::NUM;
        let color_stride = Square::NUM * Square::NUM * Square::NUM;
        let prev_to_stride = Square::NUM * Square::NUM;
        let from_stride = Square::NUM;

        continuation_index * continuation_stride
            + color_idx * color_stride
            + prev_to_idx * prev_to_stride
            + from_idx * from_stride
            + to_idx
    }
}
