mod args;
mod engine;
mod uci;
mod utils;

use args::Args;
use clap::Parser;
use engine::Engine;
use log::{debug, LevelFilter};
use simplelog::{Config, WriteLogger};
use std::error::Error;
use std::fs::File;
use uci::{UciConnection, UciInput, UciOutput};

const ENGINE_NAME: &str = "Grail";
const ENGINE_AUTHOR: &str = "Jørgen Hanssen";

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let mut uci = UciConnection::new();
    let mut engine = engine::create(&args);

    uci.listen(|input, output| {
        match input {
            UciInput::Uci => {
                output.send(UciOutput::IdName(ENGINE_NAME.to_string()))?;
                output.send(UciOutput::IdAuthor(ENGINE_AUTHOR.to_string()))?;
                output.send(UciOutput::UciOk)?;
            }
            UciInput::IsReady => {
                output.send(UciOutput::ReadyOk)?;
            }
            UciInput::UciNewGame => {}
            UciInput::Position(board) => {
                engine.set_position(board.clone());
            }
            UciInput::Go(params) => {
                let best_move = engine.search(params, &output);

                output.send(UciOutput::BestMove {
                    best_move: best_move,
                    ponder: None,
                })?;
            }
            UciInput::Stop => {
                engine.stop();
            }
            UciInput::Quit => {
                engine.stop();
            }
            UciInput::Unknown(line) => {
                debug!("Unknown command: {}", line);
            }
        }
        Ok(())
    })?;

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create(&args.log_file)?,
    )
    .unwrap();

    Ok(args)
}
