mod args;
mod generator;

use args::Args;
use clap::Parser;
use generator::Generator;
use log::LevelFilter;
use simplelog::{Config, SimpleLogger};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let generator = Generator::new(num_cpus::get_physical());
    let samples = generator.run(args.duration, args.depth);

    for (board, score) in samples.iter() {
        log::info!("{} => {}", board.get_hash(), score);
    }

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}
