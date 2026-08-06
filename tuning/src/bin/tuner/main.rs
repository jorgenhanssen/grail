mod args;
mod gradient;
mod params;
mod state;

use args::Args;
use clap::Parser;
use config::EngineConfig;
use gradient::Gradient;
use params::Parameters;
use state::State;

fn main() -> Result<(), String> {
    let args = Args::parse();

    let params = Parameters::load(&args.params);
    assert!(!params.is_empty(), "params file is empty");

    let state = State::from_params(&params)?;
    let grad = Gradient::random(&params);

    let a = state.apply(&grad, &params);
    let b = state.apply(&-grad, &params);

    let _ = a.to_config(EngineConfig::default());
    let _ = b.to_config(EngineConfig::default());

    for (name, _) in params.iter() {
        println!(
            "{}: state={} a={} b={}",
            name, state.values[name], a.values[name], b.values[name],
        );
    }

    Ok(())
}
