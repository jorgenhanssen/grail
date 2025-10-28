mod args;
mod generator;
mod histogram;

use args::Args;
use clap::Parser;
use generator::Generator;
use log::LevelFilter;
use nnue::{samples::Samples, version::VersionManager};
use simplelog::{Config, SimpleLogger};
use std::{
    error::Error,
    fs::File,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let manager = VersionManager::new()?;

    // Set up SIGINT handler
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_handler = Arc::clone(&stop_flag);

    ctrlc::set_handler(move || {
        log::info!("Received SIGINT, stopping generation...");
        stop_flag_handler.store(true, Ordering::Relaxed);
    })?;

    let generator = Generator::new(num_cpus::get(), &manager, args.book)?;
    let evaluations = generator.run(args.depth, stop_flag);

    let samples = Samples::from_evaluations(&evaluations);

    log::info!("Generated {} samples", samples.len());

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
