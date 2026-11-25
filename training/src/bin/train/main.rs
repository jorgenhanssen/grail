mod args;
mod data_loader;
mod feature_encoder;
mod state;
mod trainer;
mod utils;

use args::{Args, Command};
use candle_core::DType;
use candle_nn::{VarBuilder, VarMap};
use clap::Parser;
use nnue::MODEL_PATH;
use nnue::network::Network;
use simplelog::{Config, LevelFilter, SimpleLogger};
use state::TrainingState;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use trainer::Trainer;
use training::dataset::DatasetCollection;
use training::paths::STATE_PATH;
use utils::device::get_device;

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::init(LevelFilter::Info, Config::default())?;
    let args = Args::parse();

    match args.command {
        Some(Command::Init { seed }) => init_model(seed),
        None => train(args),
    }
}

fn init_model(seed: Option<u64>) -> Result<(), Box<dyn Error>> {
    let state_path = Path::new(STATE_PATH);
    let model_path = Path::new(MODEL_PATH);

    let state = TrainingState::new(seed);

    let device = get_device();

    if !device.is_cpu() {
        device.set_seed(state.seed)?;
    } else if seed.is_some() {
        // Candle deosn't support seeded weight initialization on CPU :(
        // "Error: cannot seed the CPU rng with set_seed"
        log::warn!("Seeded CPU weight init is not supported. Using random weights instead.");
    }

    let varmap = VarMap::new();
    let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    Network::new(&vs)?;
    varmap.save(model_path)?;
    state.save(state_path)?;

    log::info!("Saved init model to {} (seed {})", MODEL_PATH, state.seed);
    Ok(())
}

fn train(args: Args) -> Result<(), Box<dyn Error>> {
    let state_path = Path::new(STATE_PATH);
    let model_path = Path::new(MODEL_PATH);

    let mut state = TrainingState::load(state_path)?;

    let shutdown = setup_shutdown_handler()?;
    let datasets = DatasetCollection::all()?;
    let metadata = datasets.metadata()?;

    let mut trainer = Trainer::new(&args, &state, metadata.train.samples)?;
    trainer.load_model(model_path)?;

    if state.has_history() {
        log::info!(
            "Resuming from epoch {} (seed {})",
            state.next_epoch_number(),
            state.seed,
        );
    } else {
        log::info!("Starting fresh training (seed {})", state.seed);
    }

    trainer.train(&datasets, &mut state, model_path, state_path, shutdown)?;
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
