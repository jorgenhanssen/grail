use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    evaluator: nnue::Evaluator,
}

impl Matcher {
    pub fn new(workers: u64, pairs: u64, game: GameConfig) -> Self {
        let bytes = std::fs::read(nnue::MODEL_PATH).expect("Failed to read NNUE model");
        let evaluator =
            nnue::Evaluator::from_safetensors(&bytes).expect("Failed to load NNUE model");

        Self {
            workers,
            pairs,
            game,
            evaluator,
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
            let mut a = Engine::new(config_a, self.evaluator.clone());
            let mut b = Engine::new(config_b, self.evaluator.clone());

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
