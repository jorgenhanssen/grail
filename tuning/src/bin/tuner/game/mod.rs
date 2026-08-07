mod config;

pub use config::GameConfig;

use std::collections::HashMap;

use ahash::AHashSet;
use cozy_chess::{Board, Color, Move};
use search::Engine;
use uci::commands::GoParams;
use utils::{has_check, has_insufficient_material, has_legal_moves};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win(Color),
    Draw,
}

/// A game between two engines.
pub struct Game {
    // Current board
    board: Board,

    // Position counts for threefold repetition + history to engines
    position_counts: HashMap<u64, usize>,

    // Current ply
    plies: u64,

    // Game configuration
    config: GameConfig,

    // Adjudication states
    decisive_streak: u64,
    draw_streak: u64,
}

impl Game {
    pub fn new(opening: Board, config: GameConfig) -> Self {
        let mut position_counts = HashMap::new();

        // Count the opening as a position we have seen
        position_counts.insert(opening.hash(), 1);

        Self {
            board: opening,
            position_counts,
            plies: 0,
            config,
            decisive_streak: 0,
            draw_streak: 0,
        }
    }

    pub fn start(&mut self, white: &mut Engine, black: &mut Engine) -> Outcome {
        white.new_game();
        black.new_game();

        let go = GoParams {
            soft_nodes: Some(self.config.nodes),
            ..Default::default()
        };

        loop {
            if self.plies >= self.config.max_plies {
                return Outcome::Draw;
            }

            let stm = self.board.side_to_move();
            let engine = match stm {
                Color::White => &mut *white,
                Color::Black => &mut *black,
            };

            engine.set_position(self.board.clone(), Some(self.history()));

            let result = engine.search(&go, None).expect("search returned nothing");
            let pv = result.primary().expect("search returned no move");
            let mv = pv.best_move().expect("search returned no move");
            let score = pv.score;

            self.play_move(mv);

            if let Some(outcome) = self.outcome() {
                return outcome;
            }
            if let Some(outcome) = self.adjudicate(score, stm) {
                return outcome;
            }
        }
    }

    /// Check if the game could end early as a win or draw.
    ///
    /// Saves time by aborting a game if it is clearly won or drawn.
    /// Very much inspired by the fastchess implementation.
    fn adjudicate(&mut self, score: i16, stm: Color) -> Option<Outcome> {
        if score.abs() >= self.config.resign_score {
            self.decisive_streak += 1;
        } else {
            self.decisive_streak = 0;
        }

        // If there has been enough decisive plies in a row and the current
        // engine agrees it is losing, we adjudicate a win for the other side.
        let stm_is_losing = score < 0;
        let has_been_decisive_for_a_while = self.decisive_streak >= self.config.resign_moves * 2; // 2x because moves => plies
        if stm_is_losing && has_been_decisive_for_a_while {
            return Some(Outcome::Win(!stm));
        }

        // Don't consider draws if we have progress
        if self.board.halfmove_clock() == 0 {
            self.draw_streak = 0;
        }

        if score.abs() <= self.config.draw_score {
            self.draw_streak += 1;
        } else {
            self.draw_streak = 0;
        }

        // If we are far enough into the game and scores have been
        // near-equal for long enough, we adjudicate a draw.
        let is_out_of_opening = self.plies >= self.config.draw_after * 2; // 2x because moves => plies
        let has_been_drawish_for_a_while = self.draw_streak >= self.config.draw_moves * 2; // 2x because moves => plies
        if is_out_of_opening && has_been_drawish_for_a_while {
            return Some(Outcome::Draw);
        }

        None
    }

    fn play_move(&mut self, mv: Move) {
        self.board.play_unchecked(mv);
        self.plies += 1;

        let hash = self.board.hash();
        let count = self.position_counts.get(&hash).copied().unwrap_or(0);
        self.position_counts.insert(hash, count + 1);
    }

    fn outcome(&self) -> Option<Outcome> {
        if !has_legal_moves(&self.board) {
            if has_check(&self.board) {
                return Some(Outcome::Win(!self.board.side_to_move()));
            }
            return Some(Outcome::Draw);
        }

        if has_insufficient_material(&self.board) {
            return Some(Outcome::Draw);
        }
        if self.board.halfmove_clock() >= 100 {
            return Some(Outcome::Draw);
        }
        if self.repetitions(self.board.hash()) >= 3 {
            return Some(Outcome::Draw);
        }

        None
    }

    fn repetitions(&self, hash: u64) -> usize {
        self.position_counts.get(&hash).copied().unwrap_or(0)
    }

    fn history(&self) -> AHashSet<u64> {
        let current = self.board.hash();

        self.position_counts
            .keys()
            .copied()
            .filter(|&hash| hash != current)
            .collect()
    }
}
