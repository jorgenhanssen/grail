mod args;
mod book;
mod game;
mod generator;
mod histogram;
mod samples;
mod worker;

use args::Args;
use chrono::Local;
use clap::Parser;
use generator::Generator;
use log::LevelFilter;
use samples::Samples;
use simplelog::{Config, SimpleLogger};
use std::{
    error::Error,
    fs::{self, File},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    // Set up SIGINT handler
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_handler = Arc::clone(&stop_flag);

    ctrlc::set_handler(move || {
        log::info!("Received SIGINT, stopping generation...");
        stop_flag_handler.store(true, Ordering::Relaxed);
    })?;

    let generator = Generator::new(num_cpus::get(), args.nnue, args.book)?;
    let evaluations = generator.run(args.depth, stop_flag);

    let samples = Samples::from_evaluations(&evaluations);

    log::info!("Generated {} samples", samples.len());

    fs::create_dir_all("nnue/data")?;

    let timestamp = Local::now().format("%Y-%m-%d-%H:%M");
    let filename = format!("nnue/data/{}.csv", timestamp);

    log::info!("Writing samples to {}", filename);
    let mut file = File::create(&filename)?;
    samples.write(&mut file)?;

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}
