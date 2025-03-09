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
use std::collections::HashMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;
    let manager = VersionManager::new()?;
    let engines = get_engines(&manager)?;

    log::info!("Running tournament!");
    log::info!("Contestants:");
    for engine in &engines {
        log::info!("- {}", engine.name());
    }

    let mut arena = Arena::new(engines);
    let results = arena.run_tournament(args.depth);

    print_results(results);

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

        let mut nnue = Box::new(NNUE::new(&varmap, &Device::Cpu, version));

        varmap.load(file_path).unwrap();
        nnue.enable_nnue();

        engines.push(NegamaxEngine::new(nnue));
    }

    Ok(engines)
}

fn print_results(results: HashMap<String, i64>) {
    // Convert HashMap to Vec and sort by score (descending)
    let mut sorted_results: Vec<_> = results.into_iter().collect();
    sorted_results.sort_by(|a, b| b.1.cmp(&a.1));

    log::info!("\nTournament Results:");
    log::info!("------------------");
    for (engine, score) in sorted_results {
        log::info!("{}: {}", engine, score);
    }
}
