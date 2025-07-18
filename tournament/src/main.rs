mod args;
mod engine;
mod game;
mod outcome;
mod pairing;
mod positions;
mod summary;

use std::{error::Error, fs::File, io::Write};

use args::Args;
use clap::Parser;
use log::LevelFilter;
use simplelog::{Config, SimpleLogger};

use crate::{outcome::GameOutcome, pairing::Pairing, positions::get_positions, summary::Summary};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let positions = get_positions();

    let pairing = Pairing::new(positions, args.engine_a, args.engine_b, args.move_time);

    let outcomes = pairing.run();
    let summary = Summary::new(&outcomes);

    println!("\n\n{}", summary);
    save_tournament_games(&outcomes)?;

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}

#[inline]
fn save_tournament_games(outcomes: &[GameOutcome]) -> Result<(), Box<dyn Error>> {
    let mut file = File::create("tournament-games.txt")?;
    for outcome in outcomes {
        file.write_all(outcome.to_pgn().as_bytes())?;
        file.write_all(b"\n\n")?;
    }
    Ok(())
}
