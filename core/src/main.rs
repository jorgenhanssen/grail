mod bench;
mod display;
mod engine;
mod grail;
mod nnue;
mod search_metadata;
mod worker;

use std::error::Error;

use grail::Grail;

fn main() -> Result<(), Box<dyn Error>> {
    Grail::new().run(parse_args_to_uci())
}

/// Parse CLI arguments as a UCI command.
/// Allows a one-time run of a single UCI command, like "grail go depth 15" etc then exit.
/// I see Stockfish does it this way, probably to run the benchmark easier for PGO.
fn parse_args_to_uci() -> Option<String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    (!args.is_empty()).then(|| args.join(" "))
}
