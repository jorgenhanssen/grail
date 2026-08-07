use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ahash::AHashSet;
use config::EngineConfig;
use cozy_chess::{Board, Color, Move};
use rayon::ThreadPoolBuilder;
use search::Engine;
use uci::commands::GoParams;
use utils::{Book, has_check, has_insufficient_material, has_legal_moves};

use crate::progress::MatchProgress;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win(Color),
    Draw,
}

/// Configuration for a game.
#[derive(Clone, Copy)]
pub struct GameConfig {
    pub nodes: u64,
    pub max_plies: u64,

    // Adjudication
    pub resign_score: i16,
    pub resign_moves: u64,
    pub draw_score: i16,
    pub draw_moves: u64,
    pub draw_after: u64,
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
    resign_streak: u64,
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
            resign_streak: 0,
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
    /// Very much inspired by the fastgchess implementation.
    fn adjudicate(&mut self, score: i16, stm: Color) -> Option<Outcome> {
        if score.abs() >= self.config.resign_score {
            self.resign_streak += 1;
        } else {
            self.resign_streak = 0;
        }
        if score < 0 && self.resign_streak >= self.config.resign_moves * 2 {
            return Some(Outcome::Win(!stm));
        }

        if self.board.halfmove_clock() == 0 {
            self.draw_streak = 0;
        }

        if score.abs() <= self.config.draw_score {
            self.draw_streak += 1;
        } else {
            self.draw_streak = 0;
        }
        if self.plies >= self.config.draw_after * 2
            && self.draw_streak >= self.config.draw_moves * 2
        {
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

/// Match result from engine A's point of view.
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
        let ratio = self.points() / self.played() as f64;

        write!(
            f,
            "{} - {} - {}  [{:.3}]",
            self.wins, self.losses, self.draws, ratio
        )
    }
}

/// A match between two engine configurations.
pub struct Match {
    workers: u64,
    pairs: u64,
    game: GameConfig,
}

impl Match {
    pub fn new(workers: u64, pairs: u64, game: GameConfig) -> Self {
        Self {
            workers,
            pairs,
            game,
        }
    }

    pub fn play(&self, config_a: &EngineConfig, config_b: &EngineConfig, book: &Book) -> Score {
        let workers = self.workers.min(self.pairs) as usize;
        let next = AtomicUsize::new(0);
        let score = Mutex::new(Score::default());
        let progress = MatchProgress::new(self.pairs as usize * 2);

        let pool = ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .expect("failed to build match thread pool");

        // Runs the following scope once for each thread in the pool.
        // This is nice so we only set up engines a and b once per thread per match.
        pool.broadcast(|_| {
            let mut a = Engine::new(config_a, Arc::new(AtomicBool::new(false)), load_nnue);
            let mut b = Engine::new(config_b, Arc::new(AtomicBool::new(false)), load_nnue);

            while (next.fetch_add(1, Ordering::Relaxed) as u64) < self.pairs {
                let opening = book.random_position();

                // A as white
                let mut game = Game::new(opening.clone(), self.game);
                let outcome = game.start(&mut a, &mut b);
                {
                    let mut score = score.lock().unwrap();
                    score.record(outcome, Color::White);
                    progress.update(&score);
                }

                // A as black
                let mut game = Game::new(opening, self.game);
                let outcome = game.start(&mut b, &mut a);
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
