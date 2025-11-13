use crate::book::Book;
use crate::histogram::ScoreHistogram;
use crate::worker::SelfPlayWorker;
use candle_core::Device;
use candle_nn::VarMap;
use evaluation::NNUE;
use indicatif::MultiProgress;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_NNUE_PATH: &str = "nnue/model.safetensors";
const PROGRESS_UPDATE_INTERVAL_MS: u64 = 200;

pub struct Generator {
    threads: usize,
    nnue_path: Option<PathBuf>,
    opening_book: Arc<Book>,
}

impl Generator {
    pub fn new(
        threads: usize,
        use_nnue: bool,
        opening_book_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let opening_book = Arc::new(Book::load(&opening_book_path)?);

        let nnue_path = if use_nnue {
            let path = PathBuf::from(DEFAULT_NNUE_PATH);
            if path.exists() {
                log::info!("Using NNUE ({})", path.display());
                Some(path)
            } else {
                log::warn!("NNUE ({}) not found, falling back to HCE", path.display());
                None
            }
        } else {
            log::info!("Using HCE");
            None
        };

        Ok(Self {
            threads,
            nnue_path,
            opening_book,
        })
    }

    pub fn run(&self, depth: u8, stop_flag: Arc<AtomicBool>) -> Vec<(String, i16, usize)> {
        log::info!(
            "Generating samples using {} threads - Press Ctrl+C to stop",
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
                let nnue_path = self.nnue_path.clone();
                let sample_counter = Arc::clone(&sample_counter);
                let game_id_counter = Arc::clone(&game_id_counter);
                let opening_book = Arc::clone(&self.opening_book);
                let stop_flag = Arc::clone(&stop_flag);
                let histogram_handle = histogram.clone_handle();

                std::thread::spawn(move || {
                    let nnue = Self::load_nnue(nnue_path);
                    let mut worker = SelfPlayWorker::new(
                        tid,
                        sample_counter,
                        game_id_counter,
                        depth,
                        nnue,
                        opening_book,
                        histogram_handle,
                    );
                    worker.play_games(stop_flag)
                })
            })
            .collect();

        // Spawn progress update thread
        let progress_handle =
            Self::spawn_progress_updater(sample_counter.clone(), histogram, stop_flag.clone());

        // Wait for all workers to complete
        let evaluations: Vec<_> = worker_handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect();

        progress_handle.join().unwrap();

        evaluations
    }

    fn load_nnue(nnue_path: Option<PathBuf>) -> Option<Box<dyn NNUE>> {
        nnue_path.map(|path| {
            let mut varmap = VarMap::new();
            let mut nnue = nnue::Evaluator::new(&varmap, &Device::Cpu);
            varmap.load(path).unwrap();
            nnue.enable_nnue();
            Box::new(nnue) as Box<dyn NNUE>
        })
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
