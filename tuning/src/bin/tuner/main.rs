mod args;
mod game;
mod gradient;
mod params;
mod state;

use std::sync::{Arc, atomic::AtomicBool};

use args::Args;
use candle_core::Device;
use candle_nn::VarMap;
use clap::Parser;
use config::EngineConfig;
use game::Match;
use gradient::Gradient;
use params::Parameters;
use search::Engine;
use state::State;
use utils::Book;

// TODO: Consider sharing the modal path everywhere
const MODEL_PATH: &str = "nnue/model.safetensors";

fn main() -> Result<(), String> {
    let args = Args::parse();

    let params = Parameters::load(&args.params);
    assert!(!params.is_empty(), "params file is empty");

    let state = State::from_params(&params)?;
    let grad = Gradient::random(&params);
    let a = state.apply(&grad, &params);
    let b = state.apply(&-grad, &params);

    for (name, _) in params.iter() {
        println!(
            "{}: state={} a={} b={}",
            name, state.values[name], a.values[name], b.values[name],
        );
    }

    let book = Book::load(&args.book).unwrap();

    let stop = Arc::new(AtomicBool::new(false));

    let mut engine_a = Engine::new(
        &a.to_config(EngineConfig::default()),
        Arc::clone(&stop),
        load_nnue,
    );
    let mut engine_b = Engine::new(
        &b.to_config(EngineConfig::default()),
        Arc::clone(&stop),
        load_nnue,
    );

    Match::new(&args).play(&mut engine_a, &mut engine_b, &book);

    Ok(())
}

fn load_nnue() -> nnue::Evaluator {
    let mut varmap = VarMap::new();
    let mut evaluator = nnue::Evaluator::new(&varmap, &Device::Cpu);
    varmap.load(MODEL_PATH).unwrap();
    evaluator.enable_nnue();
    evaluator
}
