mod args;
mod dataset;
mod training;
mod utils;

use args::Args;
use clap::Parser;
use dataset::ShardedDataset;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use training::Trainer;

const DATA_DIR: &str = "nnue/data";
const MODEL_PATH: &str = "nnue/model.safetensors";

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    let args = Args::parse();
    let shutdown = setup_shutdown_handler()?;

    let dataset = ShardedDataset::build(
        Path::new(DATA_DIR),
        args.shard_size_mb,
        args.val_ratio,
        args.test_ratio,
    )?;

    let mut trainer = Trainer::new(&args, MODEL_PATH)?;
    trainer.train(&dataset, shutdown)?;

    Ok(())
}

fn setup_shutdown_handler() -> Result<Arc<AtomicBool>, Box<dyn Error>> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let handler = Arc::clone(&shutdown);

    ctrlc::set_handler(move || {
        log::info!("Received SIGINT, stopping training...");
        handler.store(true, Ordering::Relaxed);
    })?;

    Ok(shutdown)
}
