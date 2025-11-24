mod args;
mod bench;
mod engine;
mod nnue;

use args::Args;
use clap::Parser;
use log::{debug, LevelFilter};
use search::EngineConfig;
use simplelog::{Config, WriteLogger};
use std::error::Error;
use std::fs::File;
use uci::{UciConnection, UciInput, UciOutput};

const ENGINE_NAME: &str = "Grail";
const ENGINE_AUTHOR: &str = "Jørgen Hanssen";

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    // If bench is specified, run benchmark and exit
    if let Some(depth) = args.bench {
        bench::run(depth);
        return Ok(());
    }

    let mut uci = UciConnection::new();

    let mut config = EngineConfig::default();
    let mut engine = engine::create_engine(&config);

    uci.listen(|input, output| {
        match input {
            UciInput::Uci => {
                output.send(UciOutput::IdName(ENGINE_NAME.to_string()))?;
                output.send(UciOutput::IdAuthor(ENGINE_AUTHOR.to_string()))?;

                config.to_uci(&output)?;

                output.send(UciOutput::UciOk)?;
            }
            UciInput::IsReady => {
                output.send(UciOutput::ReadyOk)?;
            }
            UciInput::SetOption { name, value } => {
                if let Err(e) = config.update_from_uci(name, value) {
                    // TODO: Consider sending info back to the GUI
                    debug!("Option setting failed: {}", e);
                } else {
                    debug!("Set option '{}' to '{}'", name, value);
                    engine.configure(&config, false);
                }
            }
            UciInput::UciNewGame => {
                engine.new_game();
            }
            UciInput::Position {
                board,
                game_history,
            } => {
                engine.set_position(*board, game_history.clone());
            }
            UciInput::Go(params) => {
                let result = engine.search(params, Some(&output));

                if let Some((best_move, _)) = result {
                    output.send(UciOutput::BestMove {
                        best_move,
                        ponder: None,
                    })?;
                }
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

    if let Some(log_file) = &args.log_file {
        WriteLogger::init(
            LevelFilter::Debug,
            Config::default(),
            File::create(log_file)?,
        )
        .unwrap();
    }

    Ok(args)
}
