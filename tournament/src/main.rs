mod arena;
mod args;

use arena::Arena;
use args::Args;
use candle_core::Device;
use candle_nn::VarMap;
use clap::Parser;
use evaluation::TraditionalEvaluator;
use log::LevelFilter;
use nnue::{version::VersionManager, NNUE};
use search::Engine;
use search::NegamaxEngine;
use simplelog::{Config, SimpleLogger};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;
    let manager = VersionManager::new()?;
    let engines = get_engines(&manager)?;

    let mut arena = Arena::new(engines);
    let results = arena.run_tournament(args.depth);

    println!("{:?}", results);

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}

fn get_engines(manager: &VersionManager) -> Result<Vec<NegamaxEngine>, Box<dyn Error>> {
    let mut engines: Vec<NegamaxEngine> = Vec::new();

    // Add traditional evaluator
    engines.push(NegamaxEngine::new(Box::new(TraditionalEvaluator)));

    // Add all NNUEs
    let versions = manager.get_all_versions()?;
    for version in versions {
        let file_path = manager.file_path(version, "model.safetensors");
        let mut varmap = VarMap::new();
        varmap.load(file_path).unwrap();

        let nnue = Box::new(NNUE::new(&varmap, &Device::Cpu, version));

        engines.push(NegamaxEngine::new(nnue));
    }

    Ok(engines)
}
