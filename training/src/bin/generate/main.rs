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
use samples::write_samples;
use simplelog::{Config, SimpleLogger};
use std::{
    error::Error,
    fs::{self, File},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
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

    let threads = args.threads.unwrap_or_else(num_cpus::get);
    let generator = Generator::new(threads, args.pv_lines, args.book)?;
    let samples = generator.run(args.depth, stop_flag);

    log::info!("Generated {} samples", samples.len());

    fs::create_dir_all("nnue/data")?;

    let timestamp = Local::now().format("%Y-%m-%d-%H:%M");
    let filename = format!("nnue/data/{}.csv", timestamp);

    log::info!("Writing samples to {}", filename);
    let mut file = File::create(&filename)?;
    write_samples(&mut file, &samples)?;

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}
