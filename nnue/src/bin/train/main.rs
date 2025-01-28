mod args;

use args::Args;
use clap::Parser;
use log::LevelFilter;
use nnue::{samples::Samples, version::VersionManager};
use simplelog::{Config, SimpleLogger};
use std::{error::Error, fs::File};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let manager = VersionManager::new("nnue/versions")?;

    let version = manager.get_latest_version()?.expect("No version found");

    log::info!("Loading data for version {}", version);
    let path = manager.file_path(version, "data.bin");
    let mut file = File::open(&path)?;
    let samples = Samples::read_from_reader(&mut file)?;

    log::info!("Read {} samples from {:?}", samples.len(), path);

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}
