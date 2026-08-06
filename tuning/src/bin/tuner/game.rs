use std::collections::HashMap;
use std::fmt;

use ahash::AHashSet;
use cozy_chess::{Board, Color, Move};
use search::Engine;
use uci::commands::GoParams;
use utils::{Book, has_check, has_insufficient_material, has_legal_moves};

use crate::args::Args;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    White,
    Black,
    Draw,
}

impl Outcome {
    fn win(color: Color) -> Self {
        match color {
            Color::White => Outcome::White,
            Color::Black => Outcome::Black,
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::White => write!(f, "1-0"),
            Outcome::Black => write!(f, "0-1"),
            Outcome::Draw => write!(f, "1/2-1/2"),
        }
    }
}

pub struct Game {
    board: Board,
    position_counts: HashMap<u64, usize>,
    plies: usize,
    nodes: u64,
    max_plies: usize,
}

impl Game {
    pub fn new(opening: Board, nodes: u64, max_plies: usize) -> Self {
        let mut position_counts = HashMap::new();

        // Count the opening as a position we have seen (for threefold)
        position_counts.insert(opening.hash(), 1);

        Self {
            board: opening,
            position_counts,
            plies: 0,
            nodes,
            max_plies,
        }
    }

    pub fn play(&mut self, white: &mut Engine, black: &mut Engine) -> Outcome {
        white.new_game();
        black.new_game();

        let go = GoParams {
            soft_nodes: Some(self.nodes),
            ..Default::default()
        };

        loop {
            if let Some(outcome) = self.outcome() {
                return outcome;
            }
            if self.plies >= self.max_plies {
                return Outcome::Draw;
            }

            let engine = match self.board.side_to_move() {
                Color::White => &mut *white,
                Color::Black => &mut *black,
            };

            engine.set_position(self.board.clone(), Some(self.history()));

            let Some(result) = engine.search(&go, None) else {
                return Outcome::Draw;
            };
            let Some(mv) = result.primary().and_then(|pv| pv.best_move()) else {
                return Outcome::Draw;
            };

            self.play_move(mv);
        }
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
                return Some(Outcome::win(!self.board.side_to_move()));
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

#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    pub wins: usize,
    pub losses: usize,
    pub draws: usize,
}

impl Score {
    fn record(&mut self, outcome: Outcome, a_is_white: bool) {
        match (outcome, a_is_white) {
            (Outcome::Draw, _) => self.draws += 1,
            (Outcome::White, true) | (Outcome::Black, false) => self.wins += 1,
            (Outcome::White, false) | (Outcome::Black, true) => self.losses += 1,
        }
    }

    pub fn played(&self) -> usize {
        self.wins + self.losses + self.draws
    }

    pub fn points(&self) -> f64 {
        self.wins as f64 + 0.5 * self.draws as f64
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let played = self.played();
        let ratio = if played == 0 {
            0.0
        } else {
            self.points() / played as f64
        };

        write!(
            f,
            "{} - {} - {}  [{:.3}] {}",
            self.wins, self.losses, self.draws, ratio, played
        )
    }
}

pub struct Match {
    games: usize,
    nodes: u64,
    max_plies: usize,
}

impl Match {
    pub fn new(args: &Args) -> Self {
        Self {
            games: args.games,
            nodes: args.nodes,
            max_plies: args.max_plies,
        }
    }

    pub fn play(&self, a: &mut Engine, b: &mut Engine, book: &Book) -> Score {
        let mut score = Score::default();
        let mut played = 0;

        while played < self.games {
            let opening = book.random_position();

            let outcome = Game::new(opening.clone(), self.nodes, self.max_plies).play(a, b);
            score.record(outcome, true);
            played += 1;
            println!("{score}");

            if played >= self.games {
                break;
            }

            let outcome = Game::new(opening, self.nodes, self.max_plies).play(b, a);
            score.record(outcome, false);
            played += 1;
            println!("{score}");
        }

        score
    }
}
