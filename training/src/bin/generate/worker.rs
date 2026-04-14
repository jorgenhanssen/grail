use crate::book::Book;
use crate::game::SelfPlayGame;
use crate::histogram::HistogramHandle;
use crate::samples::Sample;
use search::{Engine, EngineConfig};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// TT Hash size per worker.
const WORKER_HASH_SIZE_MB: i32 = 256;

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
    /// Create a new worker.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tid: usize,
        sample_counter: Arc<AtomicUsize>,
        game_id_counter: Arc<AtomicUsize>,
        depth: u8,
        multi_pv: u8,
        create_evaluator: fn() -> nnue::Evaluator,
        opening_book: Arc<Book>,
        histogram: HistogramHandle,
    ) -> Self {
        let mut config = EngineConfig::default();

        config.hash_size.value = WORKER_HASH_SIZE_MB;
        config.multi_pv.value = multi_pv;

        // Engine stop flag (not used in data generation, but required by Engine)
        let stop = Arc::new(AtomicBool::new(false));
        let engine = Engine::new(&config, stop, create_evaluator);

        Self {
            _tid: tid,
            sample_counter,
            game_id_counter,
            depth,
            engine,
            opening_book,
            histogram,
        }
    }

    pub fn play_games(&mut self, stop_flag: Arc<AtomicBool>) -> (Vec<Sample>, Vec<u16>) {
        let mut evaluations = Vec::new();
        let mut game_lengths = Vec::new();

        while !stop_flag.load(Ordering::Relaxed) {
            let game_id = self.game_id_counter.fetch_add(1, Ordering::Relaxed);
            let opening_fen = self.opening_book.random_position();

            let mut game = SelfPlayGame::new(game_id, opening_fen, self.depth);
            game.play(&mut self.engine);

            game_lengths.push(game.game_length());
            let (samples, scores) = game.get_samples();
            self.record_statistics(&samples, scores);

            evaluations.extend(samples);
        }

        (evaluations, game_lengths)
    }

    fn record_statistics(&self, samples: &[Sample], scores: Vec<i16>) {
        let num_samples = samples.len();

        self.histogram.record_scores(&scores);
        self.sample_counter
            .fetch_add(num_samples, Ordering::Relaxed);
    }
}
