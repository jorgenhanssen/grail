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

    let manager = VersionManager::new("nnue/versions")?;

    let current_version = manager.get_latest_version()?;

    // TODO: load correct evaluation function (model if version, else default)
    match current_version {
        Some(version) => {
            let path = manager.file_path(version, "data.bin");
            let mut file = File::open(&path)?;
            let samples = Samples::read_from_reader(&mut file)?;
            log::info!("Read {} samples from {:?}", samples.len(), path);

            return Ok(());
        }
        _ => {}
    }

    let generator = Generator::new(num_cpus::get());
    let evaluations = generator.run(args.duration, args.depth);

    let samples = Samples::from_evaluations(&evaluations);

    log::info!("Generated {} samples", samples.len());

    let next_version = manager.create_next_version()?;
    let next_path = manager.file_path(next_version, "data.bin");

    let mut file = File::create(next_path)?;
    samples.write_to_writer(&mut file)?;

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}
