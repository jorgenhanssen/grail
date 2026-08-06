mod args;
mod game;
mod gradient;
mod params;
mod progress;
mod state;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use args::Args;
use clap::Parser;
use config::EngineConfig;
use game::Match;
use gradient::Gradient;
use params::Parameters;
use state::State;
use utils::Book;

fn main() -> Result<(), String> {
    let args = Args::parse();

    let params = Parameters::load(&args.params);
    assert!(!params.is_empty(), "params file is empty");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_handler = Arc::clone(&stop);
    ctrlc::set_handler(move || {
        println!("\nCtrl+C, finishing current iteration...");
        stop_handler.store(true, Ordering::Relaxed);
    })
    .expect("failed to set Ctrl+C handler");

    let mut state = State::from_params(&params)?;
    let initial = state.clone();

    let book = Book::load(&args.book).unwrap();
    let matcher = Match::new(&args);

    let mut iterations = 0u64;
    while !stop.load(Ordering::Relaxed) {
        iterations += 1;

        let grad = Gradient::random(&params);
        let a = state.apply(&grad, &params);
        let b = state.apply(&-&grad, &params);

        for (name, _) in params.iter() {
            println!(
                "{}: state={:.3} a={} b={}",
                name,
                state.values[name],
                a.values[name].round() as i64,
                b.values[name].round() as i64,
            );
        }

        let score = matcher.play(
            &a.to_config(EngineConfig::default()),
            &b.to_config(EngineConfig::default()),
            &book,
        );

        state.update(&grad, &score, &params, args.ak);
    }

    println!("\nDone after {iterations} iterations");
    for (name, _) in params.iter() {
        let start = initial.values[name];
        let end = state.values[name];
        let delta = end - start;
        println!("{name}: {start:.0} -> {end:.0} ({delta:+.3})");
    }

    Ok(())
}
