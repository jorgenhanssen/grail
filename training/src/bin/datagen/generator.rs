use crate::game::GameConfig;
use crate::histogram::ScoreHistogram;
use crate::opening::OpeningSource;
use crate::refinery::RefinementStats;
use crate::worker::SelfPlayWorker;
use indicatif::MultiProgress;
use nnue::MODEL_PATH;
use pyrrhic_rs::TableBases;
use search::CozyAdapter;
use std::error::Error;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use training::dataset::DatasetWriter;

const PROGRESS_UPDATE_INTERVAL_MS: u64 = 200;

/// Coordinates multi-threaded self-play data generation.
pub struct Generator {
    threads: usize,
    pv_lines: u8,
    opening_source: Arc<OpeningSource>,
    tablebases: Option<TableBases<CozyAdapter>>,
    evaluator: nnue::Evaluator,
}

impl Generator {
    pub fn new(
        threads: usize,
        pv_lines: u8,
        opening_source: OpeningSource,
        syzygy_path: Option<String>,
    ) -> Result<Self, Box<dyn Error>> {
        let nnue_bytes = std::fs::read(MODEL_PATH).map_err(|e| {
            format!("NNUE model not found at {MODEL_PATH} ({e}). Create an initial model first.")
        })?;
        let evaluator = nnue::Evaluator::from_safetensors(&nnue_bytes)?;

        let tablebases = match syzygy_path {
            Some(path) => {
                let path = path.replace(';', ":");
                let tb = TableBases::<CozyAdapter>::new(&path)
                    .map_err(|e| format!("Failed to load Syzygy tablebases: {:?}", e))?;
                log::info!("Syzygy tablebases loaded: up to {} pieces", tb.max_pieces());
                Some(tb)
            }
            None => None,
        };

        Ok(Self {
            threads,
            pv_lines,
            opening_source: Arc::new(opening_source),
            tablebases,
            evaluator,
        })
    }

    pub fn run(
        &self,
        game_config: GameConfig,
        max_games: Option<usize>,
        stop_flag: Arc<AtomicBool>,
        writer: Option<Arc<Mutex<DatasetWriter>>>,
    ) -> io::Result<usize> {
        log::info!(
            "Generating samples ({}, multi_pv={}, threads={}). Press Ctrl+C to stop.",
            game_config.limit,
            self.pv_lines,
            self.threads,
        );

        let sample_counter = Arc::new(AtomicUsize::new(0));
        let game_id_counter = Arc::new(AtomicUsize::new(0));

        // Create multi-progress display
        let multi_progress = MultiProgress::new();
        let histogram = ScoreHistogram::new(&multi_progress);

        // Spawn worker threads
        let worker_handles: Vec<_> = (0..self.threads)
            .map(|tid| {
                let pv_lines = self.pv_lines;
                let sample_counter = Arc::clone(&sample_counter);
                let game_id_counter = Arc::clone(&game_id_counter);
                let opening_source = Arc::clone(&self.opening_source);
                let stop_flag = Arc::clone(&stop_flag);
                let histogram_handle = histogram.clone_handle();
                let tb = self.tablebases.clone();
                let evaluator = self.evaluator.clone();
                let writer = writer.clone();

                std::thread::spawn(move || {
                    let mut worker = SelfPlayWorker::new(
                        tid,
                        sample_counter,
                        game_id_counter,
                        game_config,
                        max_games,
                        pv_lines,
                        evaluator,
                        opening_source,
                        histogram_handle,
                        tb,
                    );
                    worker.play_games(stop_flag, writer)
                })
            })
            .collect();

        // Spawn progress update thread
        let progress_handle = Self::spawn_progress_updater(
            Arc::clone(&sample_counter),
            histogram,
            Arc::clone(&stop_flag),
        );

        let mut refinery_stats = RefinementStats::default();
        for handle in worker_handles {
            let worker_stats = handle.join().unwrap()?;
            refinery_stats.merge(&worker_stats);
        }

        stop_flag.store(true, Ordering::Relaxed);
        progress_handle.join().unwrap();

        refinery_stats.log_summary();

        Ok(sample_counter.load(Ordering::Relaxed))
    }

    fn spawn_progress_updater(
        sample_counter: Arc<AtomicUsize>,
        histogram: ScoreHistogram,
        stop_flag: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                let sample_count = sample_counter.load(Ordering::Relaxed);
                histogram.update_display(sample_count);
                std::thread::sleep(Duration::from_millis(PROGRESS_UPDATE_INTERVAL_MS));
            }

            // Final update
            let final_count = sample_counter.load(Ordering::Relaxed);
            histogram.finish(final_count);
        })
    }
}
