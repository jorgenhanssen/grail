use std::collections::hash_map::RandomState;
use std::io;

use hyperloglogplus::{HyperLogLog, HyperLogLogPlus};
use indicatif::{ProgressBar, ProgressStyle};
use nnue::network::{OUTPUT_BUCKETS, output_bucket};

use super::{DatasetCollection, GameOutcome};

const PROGRESS_UPDATE_INTERVAL: usize = 100_000;
const HYPER_LOG_LOG_PRECISION: u8 = 18;

#[derive(Default)]
pub struct DatasetCollectionStats {
    pub total_samples: usize,
    pub train_samples: usize,
    pub unique_fens: usize,
    pub total_games: usize,
    pub bucket_counts: [usize; OUTPUT_BUCKETS],
    pub white_wins: usize,
    pub draws: usize,
    pub black_wins: usize,
}

impl DatasetCollection {
    pub fn stats(&self) -> io::Result<DatasetCollectionStats> {
        let readers = [
            (self.train_games()?, true),
            (self.val_games()?, false),
            (self.test_games()?, false),
        ];

        let total_bytes = readers.iter().map(|(reader, _)| reader.total_bytes()).sum();
        let progress = ProgressBar::new(total_bytes);
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} Reading datasets... {percent:>3}%")
                .unwrap(),
        );

        let mut stats = DatasetCollectionStats::default();
        let mut unique_fens: HyperLogLogPlus<String, RandomState> =
            HyperLogLogPlus::new(HYPER_LOG_LOG_PRECISION, RandomState::new()).unwrap();

        for (mut reader, train) in readers {
            let mut reported_bytes = 0;
            let mut samples_since_update = 0;

            while let Some(game) = reader.next() {
                let samples = game?;

                stats.total_games += 1;
                stats.total_samples += samples.len();

                if train {
                    stats.train_samples += samples.len();
                }

                samples_since_update += samples.len();

                for sample in samples {
                    unique_fens.insert(&format!("{:#}", sample.board));
                    stats.bucket_counts[output_bucket(&sample.board)] += 1;

                    match sample.outcome {
                        GameOutcome::White => stats.white_wins += 1,
                        GameOutcome::Draw => stats.draws += 1,
                        GameOutcome::Black => stats.black_wins += 1,
                    }
                }

                if samples_since_update >= PROGRESS_UPDATE_INTERVAL {
                    progress.inc(reader.bytes_read() - reported_bytes);
                    reported_bytes = reader.bytes_read();
                    samples_since_update = 0;
                }
            }

            progress.inc(reader.bytes_read() - reported_bytes);
        }

        stats.unique_fens = unique_fens.count() as usize;
        progress.finish_and_clear();

        Ok(stats)
    }
}

impl DatasetCollectionStats {
    pub fn log(&self) {
        log::info!("Total samples: {}", self.total_samples);
        log::info!(
            "Unique positions: ~{:.2}%",
            (self.unique_fens as f64 / self.total_samples as f64) * 100.0
        );
        log::info!("Total games: {}", self.total_games);

        let n = self.total_samples as f64;
        log::info!(
            "Outcomes: white {:.1}% / draw {:.1}% / black {:.1}%",
            100.0 * self.white_wins as f64 / n,
            100.0 * self.draws as f64 / n,
            100.0 * self.black_wins as f64 / n,
        );

        log::info!("Output bucket distribution:");
        for (i, &count) in self.bucket_counts.iter().enumerate() {
            let percentage = 100.0 * count as f64 / n;
            log::info!("  Bucket {}: {} samples ({:.1}%)", i, count, percentage);
        }
    }
}
