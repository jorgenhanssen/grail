mod args;
mod game;
mod gradient;
mod matcher;
mod params;
mod progress;
mod state;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use args::Args;
use clap::Parser;
use config::EngineConfig;
use game::GameConfig;
use gradient::Gradient;
use matcher::Matcher;
use params::Parameters;
use state::State;
use utils::Book;

fn main() -> Result<(), String> {
    let args = Args::parse();

    let params = Parameters::load(&args.params);
    if params.is_empty() {
        return Err("params file is empty".into());
    }

    let stop = abort_listener()?;

    let mut state = State::from_params(&params)?;
    let initial = state.clone();

    let book = Book::load(&args.book).unwrap();
    let workers = args.workers.unwrap_or_else(|| num_cpus::get() as u64);

    let matcher = Matcher::new(
        workers,
        args.pairs,
        GameConfig {
            nodes: args.nodes,
            max_plies: args.max_plies,
            resign_score: args.resign_score,
            resign_moves: args.resign_moves,
            draw_score: args.draw_score,
            draw_moves: args.draw_moves,
            draw_after: args.draw_after,
        },
    );

    let mut iterations = 0u64;
    while !stop.load(Ordering::Relaxed) {
        if args.iterations.is_some_and(|max| iterations >= max) {
            break;
        }
        iterations += 1;

        let grad = Gradient::random(&params);
        let a = state.apply(&grad, &params);
        let b = state.apply(&-&grad, &params);

        print_pair(&params, &state, &a, &b);

        let score = matcher.run_match(
            &a.to_config(EngineConfig::default()),
            &b.to_config(EngineConfig::default()),
            &book,
        );

        state.update(&grad, &score, &params, args.gain);
    }

    print_summary(&params, &initial, &state, iterations);

    Ok(())
}

/// Sets up a ctrl+c listener and returns a stop flag.
fn abort_listener() -> Result<Arc<AtomicBool>, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let handler = Arc::clone(&stop);

    ctrlc::set_handler(move || {
        println!("\nCtrl+C, finishing current iteration...");
        handler.store(true, Ordering::Relaxed);
    })
    .map_err(|e| format!("failed to set Ctrl+C handler: {e}"))?;

    Ok(stop)
}

fn print_pair(params: &Parameters, state: &State, a: &State, b: &State) {
    for (name, _) in params.iter() {
        println!(
            "{}: state={:.3} a={} b={}",
            name,
            state.values[name],
            a.values[name].round() as i64,
            b.values[name].round() as i64,
        );
    }
}

fn print_summary(params: &Parameters, initial: &State, state: &State, iterations: u64) {
    println!("\nDone after {iterations} iterations");
    for (name, _) in params.iter() {
        let start = initial.values[name];
        let end = state.values[name];
        let delta = end - start;
        println!("{name}: {start:.0} -> {end:.0} ({delta:+.3})");
    }
}
