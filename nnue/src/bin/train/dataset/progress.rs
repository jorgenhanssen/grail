use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

/// Progress bar for shard building.
pub struct ShardProgressBar {
    bar: ProgressBar,
}

impl ShardProgressBar {
    pub fn new(files: &[PathBuf]) -> Self {
        let total_bytes: u64 = files
            .iter()
            .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
            .sum();

        let bar = ProgressBar::new(total_bytes);
        bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} Building shards... {percent:>3}%")
                .unwrap(),
        );

        Self { bar }
    }

    pub fn update(&self, bytes: u64) {
        self.bar.inc(bytes);
    }

    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }
}
