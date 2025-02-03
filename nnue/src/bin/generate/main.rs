mod args;
mod generator;

use args::Args;
use clap::Parser;
use generator::Generator;
use log::LevelFilter;
use nnue::{samples::Samples, version::VersionManager};
use simplelog::{Config, SimpleLogger};
use std::{error::Error, fs::File};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let manager = VersionManager::new()?;

    let generator = Generator::new(num_cpus::get(), &manager)?;
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
    const BIN_SIZE: f32 = 0.1;
    const NUM_BINS: usize = 20;
    const MIN_SCORE: f32 = -1.0;
    const MAX_WIDTH: usize = 50; // Maximum number of '=' characters for visualization

    let mut bins = vec![0; NUM_BINS];

    for (_, score) in &samples.samples {
        let bin_idx = (((score.clamp(MIN_SCORE, -MIN_SCORE) - MIN_SCORE) / BIN_SIZE) as usize)
            .min(NUM_BINS - 1);
        bins[bin_idx] += 1;
    }

    // Find the maximum bin count for scaling
    let max_count = *bins.iter().max().unwrap_or(&1);

    for (i, count) in bins.iter().enumerate() {
        let range_start = MIN_SCORE + (i as f32 * BIN_SIZE);
        let range_end = range_start + BIN_SIZE;
        if *count > 0 {
            let bar_length = ((*count * MAX_WIDTH) as f32 / max_count as f32).round() as usize;
            let bar = "=".repeat(bar_length.max(1));
            log::info!(
                "[{:5.2} to {:5.2}]: {:6} samples |{}",
                range_start,
                range_end,
                count,
                bar
            );
        }
    }
}
