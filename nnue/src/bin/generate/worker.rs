use crate::book::Book;
use crate::game::SelfPlayGame;
use crate::histogram::HistogramHandle;
use evaluation::{hce, NNUE};
use search::{Engine, EngineConfig};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

// Reduced hash per worker to limit total RAM when running many threads.
const WORKER_HASH_SIZE_MB: i32 = 384;

/// A single worker thread that plays self-play games and collects samples.
/// Each worker has its own engine instance to avoid contention.
pub struct SelfPlayWorker {
    _tid: usize,
    sample_counter: Arc<AtomicUsize>,
    game_id_counter: Arc<AtomicUsize>,
    engine: Engine,
    depth: u8,
    opening_book: Arc<Book>,
    histogram: HistogramHandle,
}

impl SelfPlayWorker {
    pub fn new(
        tid: usize,
        sample_counter: Arc<AtomicUsize>,
        game_id_counter: Arc<AtomicUsize>,
        depth: u8,
        nnue: Option<Box<dyn NNUE>>,
        opening_book: Arc<Book>,
        histogram: HistogramHandle,
    ) -> Self {
        let mut config = EngineConfig::default();

        // Reduced hash size to reduce RAM usage
        config.hash_size.value = WORKER_HASH_SIZE_MB;

        let hce = Box::new(hce::Evaluator::new(
            config.get_piece_values(),
            config.get_hce_config(),
        ));

        // Engine stop flag (not used in data generation, but required by Engine)
        let stop = Arc::new(AtomicBool::new(false));

        Self {
            _tid: tid,
            sample_counter,
            game_id_counter,
            depth,
            engine: Engine::new(&config, hce, nnue, stop),
            opening_book,
            histogram,
        }
    }

    pub fn play_games(&mut self, stop_flag: Arc<AtomicBool>) -> Vec<(String, i16, usize)> {
        let mut evaluations = Vec::new();

        while !stop_flag.load(Ordering::Relaxed) {
            let game_id = self.game_id_counter.fetch_add(1, Ordering::Relaxed);
            let opening_fen = self.opening_book.random_position();

            let mut game = SelfPlayGame::new(game_id, opening_fen);
            game.play(&mut self.engine, self.depth);

            let (samples, scores) = game.drain_samples();
            self.record_statistics(&samples, scores);

            evaluations.extend(samples);
        }

        evaluations
    }

    fn record_statistics(&self, samples: &[(String, i16, usize)], scores: Vec<i16>) {
        let num_samples = samples.len();

        self.histogram.record_scores(&scores);
        self.sample_counter
            .fetch_add(num_samples, Ordering::Relaxed);
    }
}
