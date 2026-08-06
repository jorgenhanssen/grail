mod args;
mod params;
mod state;

use args::Args;
use clap::Parser;
use config::EngineConfig;
use params::load_params;
use state::State;

fn main() -> Result<(), String> {
    let args = Args::parse();

    let params = load_params(&args.params);
    assert!(!params.is_empty(), "params file is empty");

    let state = State::from_params(&params)?;
    let config = state.to_config(EngineConfig::default());

    let json = serde_json::to_value(config).unwrap();
    for (name, param) in &params {
        let value = state.values[name];
        assert_eq!(json[name].as_i64().unwrap(), value);
        println!(
            "{}: value={} step={} min={} max={}",
            name, value, param.step, param.min, param.max
        );
    }

    Ok(())
}
