mod args;
mod engine;
mod grail;
mod nnue;
mod worker;

use std::error::Error;
use std::fs::File;

use args::Args;
use clap::Parser;
use grail::Grail;
use log::LevelFilter;
use simplelog::{Config, WriteLogger};

fn main() -> Result<(), Box<dyn Error>> {
    init()?;
    Grail::new().run()
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    if let Some(log_file) = &args.log_file {
        WriteLogger::init(
            LevelFilter::Debug,
            Config::default(),
            File::create(log_file)?,
        )
        .unwrap();
    }

    Ok(args)
}
