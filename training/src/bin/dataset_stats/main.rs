mod stats;

use std::io;
use std::path::PathBuf;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use stats::DatasetStats;
use training::dataset::{DatasetCollection, GameReader};

const PROGRESS_UPDATE_INTERVAL: usize = 100_000;

#[derive(Parser)]
#[command(about = "Show statistics for generated training datasets")]
struct Args {
    /// Binary files to read. Defaults to all datasets and splits.
    files: Vec<PathBuf>,

    /// Evaluation threshold in centipawns.
    #[arg(short, long, default_value_t = 600)]
    threshold: i32,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let readers = if args.files.is_empty() {
        let datasets = DatasetCollection::open()?;
        vec![
            datasets.train_games()?,
            datasets.val_games()?,
            datasets.test_games()?,
        ]
    } else {
        vec![GameReader::new(args.files)?]
    };

    let total_bytes = readers.iter().map(|reader| reader.total_bytes()).sum();
    let progress = ProgressBar::new(total_bytes);
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} Reading datasets... {percent:>3}%")
            .unwrap(),
    );

    let mut stats = DatasetStats::new(args.threshold);

    for mut reader in readers {
        let mut reported_bytes = 0;
        let mut samples_since_update = 0;

        while let Some(game) = reader.next() {
            let samples = game?;

            stats.record_game(&samples);
            samples_since_update += samples.len();

            if samples_since_update >= PROGRESS_UPDATE_INTERVAL {
                let bytes_read = reader.bytes_read()?;

                progress.inc(bytes_read - reported_bytes);
                reported_bytes = bytes_read;
                samples_since_update = 0;
            }
        }

        progress.inc(reader.bytes_read()? - reported_bytes);
    }

    progress.finish_and_clear();

    stats.print();

    Ok(())
}
