use crate::book::Book;
use crate::game::SelfPlayGame;
use crate::histogram::HistogramHandle;
use crate::samples::Sample;
use pyrrhic_rs::TableBases;
use search::{CozyAdapter, Engine, EngineConfig};
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
    tablebases: Option<TableBases<CozyAdapter>>,
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
        tablebases: Option<TableBases<CozyAdapter>>,
    ) -> Self {
        let mut config = EngineConfig::default();

        config.hash_size.value = WORKER_HASH_SIZE_MB;
        config.multi_pv.value = multi_pv;

        let stop = Arc::new(AtomicBool::new(false));
        let engine = Engine::new(&config, stop, create_evaluator);

        if let Some(ref tb) = tablebases {
            engine.set_tablebases(tb.clone());
        }

        Self {
            _tid: tid,
            sample_counter,
            game_id_counter,
            depth,
            engine,
            opening_book,
            histogram,
            tablebases,
        }
    }

    pub fn play_games(&mut self, stop_flag: Arc<AtomicBool>) -> Vec<Sample> {
        let mut evaluations = Vec::new();

        while !stop_flag.load(Ordering::Relaxed) {
            let game_id = self.game_id_counter.fetch_add(1, Ordering::Relaxed);
            let opening_fen = self.opening_book.random_position();

            let mut game =
                SelfPlayGame::new(game_id, opening_fen, self.depth, self.tablebases.clone());
            game.play(&mut self.engine);

            let (samples, scores) = game.get_samples();
            self.record_statistics(&samples, scores);

            evaluations.extend(samples);
        }

        evaluations
    }

    fn record_statistics(&self, samples: &[Sample], scores: Vec<i16>) {
        let num_samples = samples.len();

        self.histogram.record_scores(&scores);
        self.sample_counter
            .fetch_add(num_samples, Ordering::Relaxed);
    }
}
