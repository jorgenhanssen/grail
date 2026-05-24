mod args;
mod dataset;
mod state;
mod training;
mod utils;

use args::{Args, Command};
use candle_core::DType;
use candle_nn::{VarBuilder, VarMap};
use clap::Parser;
use dataset::ShardedDataset;
use nnue::network::Network;
use simplelog::{Config, LevelFilter, SimpleLogger};
use state::TrainingState;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use training::Trainer;
use utils::device::get_device;

const DATA_DIR: &str = "nnue/data";
const MODEL_PATH: &str = "nnue/model.safetensors";
const STATE_PATH: &str = "nnue/training.json";

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::init(LevelFilter::Info, Config::default())?;
    let args = Args::parse();

    match args.command {
        Some(Command::Init) => init_model(),
        None => train(args),
    }
}

fn init_model() -> Result<(), Box<dyn Error>> {
    let state_path = Path::new(STATE_PATH);
    let model_path = Path::new(MODEL_PATH);

    if TrainingState::destroy(state_path)? {
        log::warn!("Discarded existing training state at {}", STATE_PATH);
    }

    let device = get_device()?;
    let varmap = VarMap::new();
    let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    Network::new(&vs)?;
    varmap.save(model_path)?;

    log::info!("Saved random model to {}", MODEL_PATH);
    Ok(())
}

fn train(args: Args) -> Result<(), Box<dyn Error>> {
    let state_path = Path::new(STATE_PATH);
    let model_path = Path::new(MODEL_PATH);

    if args.restart && TrainingState::destroy(state_path)? {
        log::warn!("Discarded existing training state at {}", STATE_PATH);
    }

    let mut state = TrainingState::new(state_path, args.val_ratio, args.test_ratio)?;

    let shutdown = setup_shutdown_handler()?;
    let dataset = ShardedDataset::build(Path::new(DATA_DIR), args.shard_size_mb, &state)?;
    let mut trainer = Trainer::new(&args, &state)?;

    if state.has_history() {
        log::info!(
            "Resuming from epoch {} (seed {}, val/test {:.2}/{:.2})",
            state.next_epoch_number(),
            state.seed,
            state.val_ratio,
            state.test_ratio,
        );
        trainer.load_model(model_path)?;
    } else {
        log::info!(
            "Starting fresh training (seed {}, val/test {:.2}/{:.2})",
            state.seed,
            state.val_ratio,
            state.test_ratio,
        );
        trainer.save_model(model_path)?;
    }

    trainer.train(&dataset, &mut state, model_path, state_path, shutdown)?;
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
