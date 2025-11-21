use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

pub struct IndexProgressBar {
    bar: ProgressBar,
    bytes_read: Arc<AtomicU64>,
    samples_indexed: Arc<AtomicUsize>,
}

impl IndexProgressBar {
    pub fn new(files: &[PathBuf]) -> Self {
        let total_bytes: u64 = files
            .iter()
            .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
            .sum();

        let bar = ProgressBar::new(total_bytes);
        bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} Indexing files... {percent:>3}%")
                .unwrap(),
        );

        Self {
            bar,
            bytes_read: Arc::new(AtomicU64::new(0)),
            samples_indexed: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn update(&self, bytes: u64, samples: usize) {
        let total_bytes = self.bytes_read.fetch_add(bytes, Ordering::Relaxed) + bytes;
        self.samples_indexed.fetch_add(samples, Ordering::Relaxed);

        self.bar.set_position(total_bytes);
    }

    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }
}
