use cozy_chess::{Board, Move, Piece, Square};

use super::utils::apply_gravity;
use crate::stack::SearchNode;
use crate::{EngineConfig, MAX_DEPTH};

/// Continuation history: scores moves based on the sequence of prior moves.
/// Stockfish-style indexing: [cont_idx][prev_piece][prev_to][curr_piece][curr_to]
///
/// Tracks correlations like "after Nf3, playing Bc4 tends to be good."
/// Index 0 = opponent's last move, index 1 = our previous move, etc.
/// Helps with move ordering by learning common tactical/positional patterns.
///
/// <https://www.chessprogramming.org/Countermove_Heuristic>
#[derive(Clone)]
pub struct ContinuationHistory {
    // Flattened: [continuation_index][prev_piece][prev_to][curr_piece][curr_to]
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
        let size = Self::table_size(max_moves);
        Self {
            continuations: vec![0; size],
            max_moves,
            max_history,
            bonus_multiplier,
            malus_multiplier,
        }
    }

    fn table_size(max_moves: usize) -> usize {
        // [cont_idx][prev_piece][prev_to][curr_piece][curr_to]
        max_moves * Piece::NUM * Square::NUM * Piece::NUM * Square::NUM
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
        let size = Self::table_size(self.max_moves);
        self.continuations = vec![0; size];
    }

    fn get_continuation(
        &self,
        continuation_index: usize,
        prev_piece: Piece,
        prev_to: Square,
        curr_piece: Piece,
        curr_to: Square,
    ) -> i16 {
        if continuation_index >= self.max_moves {
            return 0;
        }
        self.continuations[self.index(continuation_index, prev_piece, prev_to, curr_piece, curr_to)]
    }

    pub fn get(
        &self,
        prev_moves: &[Option<(Piece, Square)>],
        curr_piece: Piece,
        curr_to: Square,
    ) -> i16 {
        let mut score = 0;
        for (continuation_index, prev_move) in prev_moves.iter().enumerate().take(self.max_moves) {
            if let Some((prev_piece, prev_to)) = *prev_move {
                score += self.get_continuation(
                    continuation_index,
                    prev_piece,
                    prev_to,
                    curr_piece,
                    curr_to,
                );
            }
        }
        score
    }

    pub fn get_bonus(&self, depth: u8) -> i32 {
        self.bonus_multiplier * depth.min(MAX_DEPTH as u8) as i32
    }

    pub fn get_malus(&self, depth: u8) -> i32 {
        -self.malus_multiplier * depth.min(MAX_DEPTH as u8) as i32
    }

    pub fn get_prev_moves(&self, search_stack: &[SearchNode]) -> Vec<Option<(Piece, Square)>> {
        let len = search_stack.len();
        let mut vec = vec![None; self.max_moves];
        for i in 0..self.max_moves {
            if i < len {
                let node = &search_stack[len - 1 - i];
                if let (Some(mv), Some(piece)) = (node.last_move, node.piece) {
                    vec[i] = Some((piece, mv.to));
                }
            }
        }
        vec
    }

    fn update_continuations(
        &mut self,
        prev_moves: &[Option<(Piece, Square)>],
        curr_piece: Piece,
        curr_to: Square,
        delta: i32,
    ) {
        for (continuation_index, prev_move) in prev_moves.iter().enumerate().take(self.max_moves) {
            if let Some((prev_piece, prev_to)) = *prev_move {
                let idx = self.index(continuation_index, prev_piece, prev_to, curr_piece, curr_to);
                apply_gravity(&mut self.continuations[idx], delta, self.max_history);
            }
        }
    }

    pub fn update_quiet_all(
        &mut self,
        board: &Board,
        prev_moves: &[Option<(Piece, Square)>],
        mv: Move,
        delta: i32,
    ) {
        let curr_piece = board.piece_on(mv.from).unwrap();
        let curr_to = mv.to;
        self.update_continuations(prev_moves, curr_piece, curr_to, delta);
    }

    fn index(
        &self,
        continuation_index: usize,
        prev_piece: Piece,
        prev_to: Square,
        curr_piece: Piece,
        curr_to: Square,
    ) -> usize {
        let prev_piece_idx = prev_piece as usize;
        let prev_to_idx = prev_to as usize;
        let curr_piece_idx = curr_piece as usize;
        let curr_to_idx = curr_to as usize;

        let cont_stride = Piece::NUM * Square::NUM * Piece::NUM * Square::NUM;
        let prev_piece_stride = Square::NUM * Piece::NUM * Square::NUM;
        let prev_to_stride = Piece::NUM * Square::NUM;
        let curr_piece_stride = Square::NUM;

        continuation_index * cont_stride
            + prev_piece_idx * prev_piece_stride
            + prev_to_idx * prev_to_stride
            + curr_piece_idx * curr_piece_stride
            + curr_to_idx
    }
}
