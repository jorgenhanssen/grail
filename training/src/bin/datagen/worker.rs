use crate::game::{GameConfig, SelfPlayGame};
use crate::histogram::HistogramHandle;
use crate::opening::OpeningSource;
use crate::refinery::{RefinementStats, Refinery};
use config::EngineConfig;
use pyrrhic_rs::TableBases;
use search::{CozyAdapter, Engine};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use training::dataset::{DatasetWriter, GameData};

// TT Hash size per worker.
const WORKER_HASH_SIZE_MB: i32 = 256;

/// A single worker thread that plays self-play games and collects samples.
/// Each worker has its own engine instance to avoid contention.
pub struct SelfPlayWorker {
    _tid: usize,
    sample_counter: Arc<AtomicUsize>,
    game_id_counter: Arc<AtomicUsize>,
    engine: Engine,
    game_config: GameConfig,
    max_games: Option<usize>,
    opening_source: Arc<OpeningSource>,
    histogram: HistogramHandle,
    refinery: Refinery,
}

impl SelfPlayWorker {
    /// Create a new worker.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tid: usize,
        sample_counter: Arc<AtomicUsize>,
        game_id_counter: Arc<AtomicUsize>,
        game_config: GameConfig,
        max_games: Option<usize>,
        multi_pv: u8,
        evaluator: nnue::Evaluator,
        opening_source: Arc<OpeningSource>,
        histogram: HistogramHandle,
        tablebases: Option<TableBases<CozyAdapter>>,
    ) -> Self {
        let config = EngineConfig {
            hash_size: WORKER_HASH_SIZE_MB,
            multi_pv,
            ..EngineConfig::default()
        };

        let stop = Arc::new(AtomicBool::new(false));
        let engine = Engine::new(&config, stop, evaluator);

        if let Some(ref tb) = tablebases {
            engine.set_tablebases(tb.clone());
        }

        Self {
            _tid: tid,
            sample_counter,
            game_id_counter,
            game_config,
            max_games,
            engine,
            opening_source,
            histogram,
            refinery: Refinery::new(tablebases),
        }
    }

    pub fn play_games(
        &mut self,
        stop_flag: Arc<AtomicBool>,
        writer: Option<Arc<Mutex<DatasetWriter>>>,
    ) -> io::Result<RefinementStats> {
        while !stop_flag.load(Ordering::Relaxed) {
            let game_id = self.game_id_counter.fetch_add(1, Ordering::Relaxed);

            if let Some(max) = self.max_games {
                if game_id >= max {
                    break;
                }
            }

            let opening = self.opening_source.next_opening();

            let mut game = SelfPlayGame::new(opening, self.game_config);
            game.play(&mut self.engine);

            let outcome = game.outcome();
            let mut data = game.into_data();
            self.refinery.refine(&mut data, outcome);

            if let Some(writer) = &writer {
                if let Err(error) = writer.lock().unwrap().write_game(&data) {
                    stop_flag.store(true, Ordering::Relaxed);
                    return Err(error);
                }
            }

            self.record_statistics(&data);
        }

        Ok(self.refinery.stats())
    }

    fn record_statistics(&self, game: &GameData) {
        let scores: Vec<i16> = game
            .moves
            .iter()
            .filter_map(|data| data.sample.as_ref())
            .map(|sample| sample.score)
            .collect();

        self.histogram.record_scores(&scores);
        self.sample_counter
            .fetch_add(scores.len(), Ordering::Relaxed);
    }
}
