mod args;
mod game;
mod gradient;
mod matcher;
mod params;
mod plot;
mod progress;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use args::Args;
use clap::Parser;
use config::EngineConfig;
use game::GameConfig;
use gradient::Gradient;
use matcher::Matcher;
use params::Parameters;
use utils::Book;

fn main() -> Result<(), String> {
    let args = Args::parse();

    let mut params = Parameters::load(&args.params)?;
    let mut history = vec![params.clone()];

    let stop = abort_listener()?;

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

    plot::save_png(&history);

    let mut iterations = 0u64;
    while !stop.load(Ordering::Relaxed) {
        if args.iterations.is_some_and(|max| iterations >= max) {
            break;
        }
        iterations += 1;

        let grad = Gradient::random(&params);
        let a = params.apply(&grad);
        let b = params.apply(&-&grad);

        print_pair(&params, &a, &b);

        let score = matcher.run_match(
            &a.to_config(EngineConfig::default()),
            &b.to_config(EngineConfig::default()),
            &book,
        );

        params.update(&grad, &score, args.gain);
        history.push(params.clone());

        plot::save_png(&history);
    }

    print_summary(&history, iterations);

    Ok(())
}

/// Sets up a ctrl+c listener and returns a stop flag.
///
/// First time = wait for current iteration to finish. Second = die.
fn abort_listener() -> Result<Arc<AtomicBool>, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let handler = Arc::clone(&stop);

    ctrlc::set_handler(move || {
        if handler.load(Ordering::Relaxed) {
            std::process::exit(1);
        }
        println!("\nCtrl+C, finishing iteration... (press again to kill)");
        handler.store(true, Ordering::Relaxed);
    })
    .map_err(|e| format!("failed to set Ctrl+C handler: {e}"))?;

    Ok(stop)
}

fn print_pair(params: &Parameters, a: &Parameters, b: &Parameters) {
    for param in params.iter() {
        println!(
            "{}: {:.3} ({} vs {})",
            param.name,
            param.value,
            a.get(&param.name).value.round() as i64,
            b.get(&param.name).value.round() as i64,
        );
    }
}

fn print_summary(history: &[Parameters], iterations: u64) {
    println!("\nDone after {iterations} iterations");

    let initial = &history[0];
    let params = history.last().unwrap();

    for param in params.iter() {
        let start = initial.get(&param.name).value;
        let end = param.value;
        let delta = end - start;
        println!("{}: {start:.0} -> {end:.0} ({delta:+.3})", param.name);
    }
}
