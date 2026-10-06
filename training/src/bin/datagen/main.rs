mod args;
mod game;
mod generator;
mod histogram;
mod limit;
mod opening;
mod refinery;
mod samples;
mod worker;

use args::Args;
use chrono::Local;
use clap::Parser;
use game::GameConfig;
use generator::Generator;
use limit::SearchLimit;
use log::LevelFilter;
use opening::OpeningSource;
use samples::write_samples;
use simplelog::{Config, SimpleLogger};
use std::{
    error::Error,
    fs::{self, File},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use training::paths::DATA_DIR;
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
    let limit = match args.nodes {
        Some(nodes) => SearchLimit::SoftNodes(nodes),
        None => SearchLimit::Depth(args.depth),
    };
    let generator = Generator::new(threads, args.pv_lines, opening, args.syzygy_path)?;
    let game_config = GameConfig {
        limit,
        max_opening_imbalance: args.max_opening_imbalance,
        max_teleport_plies: args.max_teleport_plies as usize,
        max_teleport_pv_fraction: args.max_teleport_pv_fraction,
        max_game_plies: args.max_game_plies as usize,
        dense_sampling: !args.sparse_samples,
    };
    let samples = generator.run(game_config, args.max_games, stop_flag);

    log::info!("Generated {} samples", samples.len());

    if args.dry_run {
        log::info!("Skipping dataset write (dry run)");
        return Ok(());
    }

    fs::create_dir_all(DATA_DIR)?;

    let timestamp = Local::now().format("%Y-%m-%d-%H:%M");
    let filename = format!("{}/{}.csv", DATA_DIR, timestamp);

    log::info!("Writing samples to {}", filename);
    let mut file = File::create(&filename)?;
    write_samples(&mut file, &samples)?;

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}
