use crate::game::SelfPlayGame;
use crate::histogram::HistogramHandle;
use crate::limit::SearchLimit;
use crate::opening::OpeningSource;
use crate::samples::Sample;
use config::EngineConfig;
use pyrrhic_rs::TableBases;
use search::{CozyAdapter, Engine};
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
    limit: SearchLimit,
    max_opening_imbalance: Option<i16>,
    max_teleport_plies: usize,
    max_game_plies: usize,
    max_games: Option<usize>,
    opening_source: Arc<OpeningSource>,
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
        limit: SearchLimit,
        max_opening_imbalance: Option<i16>,
        max_teleport_plies: usize,
        max_game_plies: usize,
        max_games: Option<usize>,
        multi_pv: u8,
        create_evaluator: fn() -> nnue::Evaluator,
        opening_source: Arc<OpeningSource>,
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
            limit,
            max_opening_imbalance,
            max_teleport_plies,
            max_game_plies,
            max_games,
            engine,
            opening_source,
            histogram,
            tablebases,
        }
    }

    pub fn play_games(&mut self, stop_flag: Arc<AtomicBool>) -> Vec<Sample> {
        let mut evaluations = Vec::new();

        while !stop_flag.load(Ordering::Relaxed) {
            let game_id = self.game_id_counter.fetch_add(1, Ordering::Relaxed);

            if let Some(max) = self.max_games {
                if game_id >= max {
                    break;
                }
            }

            let opening = self.opening_source.next_opening();

            let mut game = SelfPlayGame::new(
                game_id,
                opening,
                self.limit,
                self.max_opening_imbalance,
                self.max_teleport_plies,
                self.max_game_plies,
                self.tablebases.clone(),
            );
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
