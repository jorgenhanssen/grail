mod args;
mod stats;

use std::collections::hash_map::RandomState;
use std::io;

use args::{Args, Split};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use stats::DatasetStats;
use training::dataset::{DatasetCollection, GameReader};

fn main() -> io::Result<()> {
    let args = Args::parse();

    let datasets = if args.datasets.is_empty() {
        DatasetCollection::all()?
    } else {
        DatasetCollection::open(args.datasets)
    };

    let metadata = datasets.metadata()?;

    let (games, total_games) = match args.split {
        Some(Split::Train) => (datasets.train_games(), metadata.train.games),
        Some(Split::Val) => (datasets.val_games(), metadata.val.games),
        Some(Split::Test) => (datasets.test_games(), metadata.test.games),
        None => (
            GameReader::new(datasets.files()),
            metadata.train.games + metadata.val.games + metadata.test.games,
        ),
    };

    let progress = ProgressBar::new(total_games as u64);
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} Analyzing games... {percent:>3}%")
            .unwrap(),
    );

    let mut stats = DatasetStats::new(args.threshold, RandomState::new());

    let results: io::Result<Vec<_>> = games
        .enumerate()
        .par_bridge()
        .try_fold_with(stats.clone(), |mut stats, (game_id, game)| {
            stats.record_game(game_id, game?);
            progress.inc(1);
            Ok(stats)
        })
        .collect();

    for result in results? {
        stats.merge(result);
    }

    progress.finish_and_clear();

    stats.print();

    Ok(())
}
