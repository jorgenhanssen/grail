use indicatif::{ProgressBar, ProgressStyle};

use crate::game::Score;

/// Progress bar for a match.
pub struct MatchProgress {
    bar: ProgressBar,
}

impl MatchProgress {
    pub fn new(games: usize) -> Self {
        let bar = ProgressBar::new(games as u64);

        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.cyan} {pos}/{len} [{wide_bar:.cyan/blue}] {msg}")
                .unwrap(),
        );

        Self { bar }
    }

    pub fn update(&self, score: &Score) {
        self.bar.set_message(score.to_string());
        self.bar.inc(1);
    }

    pub fn finish(&self, score: &Score) {
        self.bar.set_message(score.to_string());
        self.bar.finish();
    }
}
