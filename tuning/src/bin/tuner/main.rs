mod args;
mod params;

use args::Args;
use clap::Parser;
use config::EngineConfig;
use params::{load_params, read_param, write_param};

fn main() {
    let args = Args::parse();

    let params = load_params(&args.params);
    assert!(!params.is_empty(), "params file is empty");

    let mut config = EngineConfig::default();

    for (name, param) in &params {
        let default = read_param(&config, name);
        let avg = param.min.saturating_add(param.max) / 2;

        write_param(&mut config, name, avg);
        assert_eq!(read_param(&config, name), avg);

        write_param(&mut config, name, default);
        assert_eq!(read_param(&config, name), default);

        println!(
            "{}: default={} step={} min={} max={}",
            name, default, param.step, param.min, param.max
        );
    }
}
