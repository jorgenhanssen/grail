mod args;
mod game;
mod generator;
mod histogram;
mod limit;
mod opening;
mod samples;
mod worker;

use args::{Args, Opening};
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
    let opening = match args.opening {
        Opening::Book { path } => OpeningSource::Book(Book::load(&path)?),
        Opening::Random { plies } => OpeningSource::Random { plies },
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
        max_game_plies: args.max_game_plies as usize,
        dense_sampling: !args.sparse_samples,
    };
    let samples = generator.run(game_config, args.max_games, stop_flag);

    log::info!("Generated {} samples", samples.len());

    if args.dry_run {
        log::info!("Skipping dataset write (dry run)");
        return Ok(());
    }

    fs::create_dir_all("nnue/data")?;

    let timestamp = Local::now().format("%Y-%m-%d-%H:%M");
    let filename = format!("nnue/data/{}.csv", timestamp);

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
