use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use config::EngineConfig;
use cozy_chess::Color;
use rayon::ThreadPoolBuilder;
use search::Engine;
use utils::Book;

use crate::game::{Game, GameConfig, Outcome};
use crate::progress::MatchProgress;

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
            Outcome::Win(winner) => {
                if winner == perspective {
                    self.wins += 1;
                } else {
                    self.losses += 1;
                }
            }
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
            "{}W {}L {}D [{:.3}]",
            self.wins, self.losses, self.draws, ratio
        )
    }
}

/// Runs paired games between two engine configurations.
pub struct Matcher {
    workers: u64,
    pairs: u64,
    game: GameConfig,
}

impl Matcher {
    pub fn new(workers: u64, pairs: u64, game: GameConfig) -> Self {
        Self {
            workers,
            pairs,
            game,
        }
    }

    pub fn run_match(
        &self,
        config_a: &EngineConfig,
        config_b: &EngineConfig,
        book: &Book,
    ) -> Score {
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
