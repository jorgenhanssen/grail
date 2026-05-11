use cozy_chess::{Board, Move};

use config::EngineConfig;

use super::piece_to::PieceTo;
use super::utils::apply_gravity;
use crate::MAX_DEPTH;
use crate::stack::SearchNode;

/// Continuation history: scores moves based on the sequence of prior moves.
/// Indexing: [lookback][prev: PieceTo][curr: PieceTo]
///
/// Tracks correlations like "after White's Nf3, playing Bc4 tends to be good."
/// lookback 0 = opponent's last move, 1 = our previous move, etc.
/// Helps with move ordering by learning common tactical/positional patterns.
///
/// <https://www.chessprogramming.org/Countermove_Heuristic>
#[derive(Clone)]
pub struct ContinuationHistory {
    // Flattened: [lookback][prev: PieceTo][curr: PieceTo]
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
        // [lookback][prev: PieceTo][curr: PieceTo]
        max_moves * PieceTo::SIZE * PieceTo::SIZE
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
        if self.continuations.len() == size {
            self.continuations.fill(0);
        } else {
            self.continuations = vec![0; size];
        }
    }

    fn get_continuation(&self, lookback: usize, prev: PieceTo, curr: PieceTo) -> i16 {
        if lookback >= self.max_moves {
            return 0;
        }
        self.continuations[self.index(lookback, prev, curr)]
    }

    pub fn get(&self, prev_moves: &[Option<PieceTo>], curr: PieceTo) -> i16 {
        let mut score = 0;
        for (lookback, prev_move) in prev_moves.iter().enumerate().take(self.max_moves) {
            if let Some(prev) = *prev_move {
                score += self.get_continuation(lookback, prev, curr);
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

    pub fn get_prev_moves(&self, search_stack: &[SearchNode]) -> Vec<Option<PieceTo>> {
        let len = search_stack.len();
        let mut vec = vec![None; self.max_moves];
        for i in 0..self.max_moves {
            if i < len {
                let node = &search_stack[len - 1 - i];
                if let (Some(mv), Some(piece), Some(color)) =
                    (node.last_move, node.piece, node.color)
                {
                    vec[i] = Some(PieceTo::new(color, piece, mv.to));
                }
            }
        }
        vec
    }

    fn update_continuations(&mut self, prev_moves: &[Option<PieceTo>], curr: PieceTo, delta: i32) {
        for (lookback, prev_move) in prev_moves.iter().enumerate().take(self.max_moves) {
            if let Some(prev) = *prev_move {
                let idx = self.index(lookback, prev, curr);
                apply_gravity(&mut self.continuations[idx], delta, self.max_history);
            }
        }
    }

    pub fn update_quiet_all(
        &mut self,
        board: &Board,
        prev_moves: &[Option<PieceTo>],
        mv: Move,
        delta: i32,
    ) {
        let curr = PieceTo::new(
            board.side_to_move(),
            board.piece_on(mv.from).unwrap(),
            mv.to,
        );
        self.update_continuations(prev_moves, curr, delta);
    }

    fn index(&self, lookback: usize, prev: PieceTo, curr: PieceTo) -> usize {
        lookback * PieceTo::SIZE * PieceTo::SIZE + prev.index() * PieceTo::SIZE + curr.index()
    }
}
