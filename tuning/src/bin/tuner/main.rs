mod args;
mod params;
mod state;

use args::Args;
use clap::Parser;
use config::EngineConfig;
use params::{load_params, read_param};
use state::State;

fn main() {
    let args = Args::parse();

    let params = load_params(&args.params);
    assert!(!params.is_empty(), "params file is empty");

    let state = State::from_params(&params);
    let config = state.to_config(EngineConfig::default());

    for (name, param) in &params {
        let value = state.values[name];
        assert_eq!(read_param(&config, name), value);
        println!(
            "{}: value={} step={} min={} max={}",
            name, value, param.step, param.min, param.max
        );
    }
}
