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
use game::Game;
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
    let opening = book.random_position();
    println!("opening: {opening}");

    let config_a = a.to_config(EngineConfig::default());
    let config_b = b.to_config(EngineConfig::default());

    let stop = Arc::new(AtomicBool::new(false));
    let mut white = Engine::new(&config_a, Arc::clone(&stop), load_nnue);
    let mut black = Engine::new(&config_b, stop, load_nnue);

    let mut game = Game::new(opening, args.nodes, args.max_plies);
    let outcome = game.play(&mut white, &mut black);

    println!("result: {outcome}");

    Ok(())
}

fn load_nnue() -> nnue::Evaluator {
    let mut varmap = VarMap::new();
    let mut evaluator = nnue::Evaluator::new(&varmap, &Device::Cpu);
    varmap.load(MODEL_PATH).unwrap();
    evaluator.enable_nnue();
    evaluator
}
