mod args;
mod dataset;
mod training;
mod utils;

use args::Args;
use clap::Parser;
use dataset::Dataset;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::error::Error;
use training::Trainer;

const DATA_DIR: &str = "nnue/data";
const MODEL_PATH: &str = "nnue/model.safetensors";

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    let args = Args::parse();

    let mut dataset = Dataset::load(&args, DATA_DIR)?;

    let mut trainer = Trainer::new(&args, MODEL_PATH)?;
    trainer.train(&mut dataset)?;

    Ok(())
}
