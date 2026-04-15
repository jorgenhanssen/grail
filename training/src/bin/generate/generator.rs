use crate::book::Book;
use crate::histogram::ScoreHistogram;
use crate::samples::Sample;
use crate::worker::SelfPlayWorker;
use candle_core::Device;
use candle_nn::VarMap;
use indicatif::MultiProgress;
use pyrrhic_rs::TableBases;
use search::CozyAdapter;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

const MODEL_PATH: &str = "nnue/model.safetensors";
const PROGRESS_UPDATE_INTERVAL_MS: u64 = 200;

/// Coordinates multi-threaded self-play data generation.
pub struct Generator {
    threads: usize,
    pv_lines: u8,
    opening_book: Arc<Book>,
    tablebases: Option<TableBases<CozyAdapter>>,
}

impl Generator {
    pub fn new(
        threads: usize,
        pv_lines: u8,
        opening_book_path: String,
        syzygy_path: Option<String>,
    ) -> Result<Self, Box<dyn Error>> {
        let opening_book = Arc::new(Book::load(&opening_book_path)?);

        if !PathBuf::from(MODEL_PATH).exists() {
            return Err(format!(
                "NNUE model not found at {}. Create an initial model first.",
                MODEL_PATH
            )
            .into());
        }

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
            opening_book,
            tablebases,
        })
    }

    pub fn run(&self, depth: u8, stop_flag: Arc<AtomicBool>) -> Vec<Sample> {
        log::info!(
            "Generating samples (depth={}, multi_pv={}, threads={}) - Press Ctrl+C to stop",
            depth,
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
                let opening_book = Arc::clone(&self.opening_book);
                let stop_flag = Arc::clone(&stop_flag);
                let histogram_handle = histogram.clone_handle();
                let tb = self.tablebases.clone();

                std::thread::spawn(move || {
                    let mut worker = SelfPlayWorker::new(
                        tid,
                        sample_counter,
                        game_id_counter,
                        depth,
                        pv_lines,
                        Self::load_nnue,
                        opening_book,
                        histogram_handle,
                        tb,
                    );
                    worker.play_games(stop_flag)
                })
            })
            .collect();

        // Spawn progress update thread
        let progress_handle =
            Self::spawn_progress_updater(sample_counter.clone(), histogram, stop_flag.clone());

        // Wait for all workers to complete
        let samples: Vec<_> = worker_handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect();

        progress_handle.join().unwrap();

        samples
    }

    fn load_nnue() -> nnue::Evaluator {
        let mut varmap = VarMap::new();
        let mut evaluator = nnue::Evaluator::new(&varmap, &Device::Cpu);
        varmap.load(MODEL_PATH).unwrap();
        evaluator.enable_nnue();
        evaluator
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
