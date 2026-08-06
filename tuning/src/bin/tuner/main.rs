mod args;
mod game;
mod gradient;
mod params;
mod state;

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

    let mut state = State::from_params(&params)?;
    let book = Book::load(&args.book).unwrap();
    let matcher = Match::new(&args);

    for iter in 1..=args.iterations {
        let grad = Gradient::random(&params);

        let a = state.apply(&grad, &params);
        let b = state.apply(&-&grad, &params);

        println!("Iteration {}/{}", iter, args.iterations);

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

    Ok(())
}
