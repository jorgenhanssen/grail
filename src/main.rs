mod args;
mod uci;

use args::Args;
use chess::{ChessMove, Square};
use clap::Parser;
use log::{debug, LevelFilter};
use simplelog::{Config, WriteLogger};
use std::error::Error;
use std::fs::File;
use uci::{UciConnection, UciInput, UciOutput};

const ENGINE_NAME: &str = "Grail";
const ENGINE_AUTHOR: &str = "Jørgen Hanssen";

fn main() -> Result<(), Box<dyn Error>> {
    let _ = init();

    let mut uci = UciConnection::new();

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
                debug!("board: {:?}", board.get_hash());
            }
            UciInput::Go(params) => {
                debug!("go: {:?}", params);

                // TODO: Implement search
                output.send(UciOutput::BestMove {
                    bestmove: ChessMove::new(Square::E2, Square::E4, None),
                    ponder: None,
                })?;
            }
            UciInput::Stop => {}
            UciInput::Quit => {}
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
