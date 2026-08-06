use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use ahash::AHashSet;
use config::EngineConfig;
use cozy_chess::{Board, Color, Move};
use search::Engine;
use uci::commands::GoParams;
use utils::{Book, has_check, has_insufficient_material, has_legal_moves};

use crate::args::Args;
use crate::progress::MatchProgress;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win(Color),
    Draw,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Win(Color::White) => write!(f, "1-0"),
            Outcome::Win(Color::Black) => write!(f, "0-1"),
            Outcome::Draw => write!(f, "1/2-1/2"),
        }
    }
}

pub struct Game {
    board: Board,
    position_counts: HashMap<u64, usize>,
    plies: u64,
    nodes: u64,
    max_plies: u64,
}

impl Game {
    pub fn new(opening: Board, nodes: u64, max_plies: u64) -> Self {
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

            let result = engine.search(&go, None).expect("search returned nothing");
            let mv = result
                .primary()
                .and_then(|pv| pv.best_move())
                .expect("search returned no move");

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

// Score is from a's point of view.
#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    pub wins: usize,
    pub losses: usize,
    pub draws: usize,
}

impl Score {
    fn record(&mut self, outcome: Outcome, perspective: Color) {
        match outcome {
            Outcome::Draw => self.draws += 1,
            Outcome::Win(winner) if winner == perspective => self.wins += 1,
            Outcome::Win(_) => self.losses += 1,
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
            "{} - {} - {}  [{:.3}]",
            self.wins, self.losses, self.draws, ratio
        )
    }
}

pub struct Match {
    pairs: u64,
    nodes: u64,
    max_plies: u64,
    workers: u64,
}

impl Match {
    pub fn new(args: &Args) -> Self {
        let default_workers = || {
            thread::available_parallelism()
                .map(|n| n.get() as u64)
                .unwrap_or(1)
        };

        Self {
            pairs: args.pairs,
            nodes: args.nodes,
            max_plies: args.max_plies,
            workers: args.workers.unwrap_or_else(default_workers).max(1),
        }
    }

    pub fn play(&self, config_a: &EngineConfig, config_b: &EngineConfig, book: &Book) -> Score {
        let workers = self.workers.min(self.pairs) as usize;
        let next = AtomicUsize::new(0);
        let score = Mutex::new(Score::default());
        let progress = MatchProgress::new(self.pairs as usize * 2);

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .expect("failed to build match thread pool");

        pool.broadcast(|_| {
            let mut a = Engine::new(config_a, Arc::new(AtomicBool::new(false)), load_nnue);
            let mut b = Engine::new(config_b, Arc::new(AtomicBool::new(false)), load_nnue);

            while (next.fetch_add(1, Ordering::Relaxed) as u64) < self.pairs {
                let opening = book.random_position();

                let mut game = Game::new(opening.clone(), self.nodes, self.max_plies);
                let outcome = game.play(&mut a, &mut b);
                {
                    let mut score = score.lock().unwrap();
                    score.record(outcome, Color::White);
                    progress.update(&score);
                }

                let mut game = Game::new(opening, self.nodes, self.max_plies);
                let outcome = game.play(&mut b, &mut a);
                {
                    let mut score = score.lock().unwrap();
                    score.record(outcome, Color::Black);
                    progress.update(&score);
                }
            }
        });

        let score = score.into_inner().unwrap();
        progress.finish(&score);

        score
    }
}

fn load_nnue() -> nnue::Evaluator {
    // TODO: Consider sharing the model path everywhere
    const MODEL_PATH: &str = "nnue/model.safetensors";

    let mut varmap = candle_nn::VarMap::new();
    let mut evaluator = nnue::Evaluator::new(&varmap, &candle_core::Device::Cpu);
    varmap.load(MODEL_PATH).unwrap();
    evaluator.enable_nnue();
    evaluator
}
