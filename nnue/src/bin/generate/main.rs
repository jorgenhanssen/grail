mod args;
mod generator;

use args::Args;
use clap::Parser;
use generator::Generator;
use log::LevelFilter;
use nnue::{
    samples::{Samples, CP_MAX, CP_MIN},
    version::VersionManager,
};
use simplelog::{Config, SimpleLogger};
use std::{error::Error, fs::File};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let manager = VersionManager::new()?;

    let generator = Generator::new(num_cpus::get(), &manager, args.opening_book)?;
    let evaluations = generator.run(args.duration, args.depth);

    let samples = Samples::from_evaluations(&evaluations);

    log::info!("Generated {} samples", samples.len());

    print_label_distribution(&samples);

    let next_version = manager.create_next_version()?;
    let next_path = manager.file_path(next_version, "data.csv");

    let mut file = File::create(next_path)?;
    samples.write(&mut file)?;

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}

fn print_label_distribution(samples: &Samples) {
    const BIN_SIZE: f32 = 1000.0;
    const NUM_BINS: usize = 10; // (CP_MAX - CP_MIN) / BIN_SIZE = 10000 / 1000 = 10
    const MAX_WIDTH: usize = 50; // Maximum number of '=' characters for visualization

    let mut bins = vec![0; NUM_BINS];

    for &score in &samples.scores {
        let bin_idx = ((((score as f32).clamp(CP_MIN as f32, CP_MAX as f32) - CP_MIN as f32)
            / BIN_SIZE) as usize)
            .min(NUM_BINS - 1);
        bins[bin_idx] += 1;
    }

    // Find the maximum bin count for scaling
    let max_count = *bins.iter().max().unwrap_or(&1);

    for (i, count) in bins.iter().enumerate() {
        let range_start = CP_MIN as f32 + (i as f32 * BIN_SIZE);
        let range_end = range_start + BIN_SIZE;
        if *count > 0 {
            let bar_length = ((*count * MAX_WIDTH) as f32 / max_count as f32).round() as usize;
            let bar = "=".repeat(bar_length.max(1));
            log::info!(
                "[{:5.0} to {:5.0}]: {:6} samples |{}",
                range_start,
                range_end,
                count,
                bar
            );
        }
    }
}
