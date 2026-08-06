mod args;
mod game;
mod gradient;
mod params;
mod state;

use args::Args;
use clap::Parser;
use config::EngineConfig;
use game::Match;
use gradient::Gradient;
use params::Parameters;
use state::State;
use utils::Book;

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
    let config_a = a.to_config(EngineConfig::default());
    let config_b = b.to_config(EngineConfig::default());

    Match::new(&args).play(&config_a, &config_b, &book);

    Ok(())
}
