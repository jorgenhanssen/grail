mod args;
mod game;
mod generator;
mod histogram;
mod opening;
mod refinery;
mod worker;

use args::Args;
use chrono::Local;
use clap::Parser;
use game::GameConfig;
use generator::Generator;
use log::LevelFilter;
use opening::OpeningSource;
use simplelog::{Config, SimpleLogger};
use std::{
    error::Error,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use training::dataset::DatasetWriter;
use utils::Book;

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    // Set up SIGINT handler
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_handler = Arc::clone(&stop_flag);

    ctrlc::set_handler(move || {
        log::info!("Received SIGINT, stopping generation...");
        stop_flag_handler.store(true, Ordering::Relaxed);
    })?;

    let threads = args.threads.unwrap_or_else(num_cpus::get);
    let book = match &args.book {
        Some(path) => Some(Book::load(path)?),
        None => None,
    };
    let opening = OpeningSource {
        book,
        random_plies: args.random_plies,
    };
    let generator = Generator::new(threads, args.pv_lines, opening, args.syzygy_path)?;
    let game_config = GameConfig {
        scout_nodes: args.scout_nodes,
        eval_nodes: args.eval_nodes,
        sample_rate: args.sample_rate,
        max_opening_imbalance: args.max_opening_imbalance,
        max_game_plies: args.max_game_plies as usize,
    };
    let writer = if args.dry_run {
        None
    } else {
        let name = Local::now().format("%Y-%m-%d-%H:%M").to_string();
        let writer = DatasetWriter::create(&name, args.val_ratio, args.test_ratio)?;
        Some(Arc::new(Mutex::new(writer)))
    };
    let samples = generator.run(game_config, args.max_games, stop_flag, writer)?;

    log::info!("Generated {} samples", samples);

    if args.dry_run {
        log::info!("Skipping dataset write (dry run)");
    }

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}
