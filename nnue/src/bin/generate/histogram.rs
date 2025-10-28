use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use nnue::samples::{CP_MAX, CP_MIN};
use std::sync::{Arc, Mutex};

const BIN_SIZE: f32 = 1000.0;
pub const NUM_BINS: usize = 10;

/// Thread-safe histogram for tracking score distribution during sample generation
pub struct ScoreHistogram {
    bins: Arc<Mutex<Vec<usize>>>,
    progress_bars: Vec<ProgressBar>,
    sample_count_bar: ProgressBar,
}

impl ScoreHistogram {
    pub fn new(multi_progress: &MultiProgress) -> Self {
        // Create sample count spinner at the top
        let sample_count_bar = multi_progress.add(ProgressBar::new_spinner());
        sample_count_bar.set_style(
            ProgressStyle::with_template(" {spinner:.cyan} {elapsed_precise} | {msg}").unwrap(),
        );

        // Create a progress bar for each bin
        let mut progress_bars = Vec::with_capacity(NUM_BINS);
        for i in 0..NUM_BINS {
            let range_start = CP_MIN as f32 + (i as f32 * BIN_SIZE);
            let range_end = range_start + BIN_SIZE;

            let pb = multi_progress.add(ProgressBar::new(100));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(&format!(
                        " [{:+6.0} to {:+6.0}] {{wide_bar:.cyan/blue}} {{msg}}",
                        range_start, range_end
                    ))
                    .unwrap(),
            );
            pb.set_position(0);
            pb.set_message("0 samples");
            progress_bars.push(pb);
        }

        Self {
            bins: Arc::new(Mutex::new(vec![0; NUM_BINS])),
            progress_bars,
            sample_count_bar,
        }
    }

    /// Get a handle that can be cloned and passed to worker threads
    pub fn clone_handle(&self) -> HistogramHandle {
        HistogramHandle {
            bins: Arc::clone(&self.bins),
        }
    }

    /// Update all progress bars with current histogram state
    pub fn update_display(&self, total_samples: usize) {
        let bins = self.bins.lock().unwrap();
        let max_count = *bins.iter().max().unwrap_or(&1).max(&1);

        // Update sample count
        self.sample_count_bar
            .set_message(format!("{} total samples", total_samples));
        self.sample_count_bar.tick();

        // Update each bin's progress bar
        for (i, &count) in bins.iter().enumerate() {
            let percentage = if max_count > 0 {
                ((count as f64 / max_count as f64) * 100.0) as u64
            } else {
                0
            };
            self.progress_bars[i].set_position(percentage);
            self.progress_bars[i].set_message(format!("{} samples", count));
        }
    }

    /// Finish all progress bars
    pub fn finish(&self, total_samples: usize) {
        self.sample_count_bar
            .finish_with_message(format!("Generated {} samples", total_samples));
        for pb in &self.progress_bars {
            pb.finish();
        }
    }
}

/// A handle to update the histogram from worker threads
pub struct HistogramHandle {
    bins: Arc<Mutex<Vec<usize>>>,
}

impl HistogramHandle {
    /// Record a batch of scores into the histogram
    pub fn record_scores(&self, scores: &[i16]) {
        let mut updates = [0usize; NUM_BINS];

        for &score in scores {
            let bin_idx = score_to_bin_index(score);
            updates[bin_idx] += 1;
        }

        if let Ok(mut bins) = self.bins.lock() {
            for (i, &count) in updates.iter().enumerate() {
                bins[i] += count;
            }
        }
    }
}

/// Convert a score to its corresponding bin index
fn score_to_bin_index(score: i16) -> usize {
    let score_f32 = score as f32;
    let clamped = score_f32.clamp(CP_MIN as f32, CP_MAX as f32);
    let normalized = clamped - CP_MIN as f32;
    let bin = (normalized / BIN_SIZE) as usize;
    bin.min(NUM_BINS - 1)
}
