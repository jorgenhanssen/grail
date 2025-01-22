use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "Grail")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "/Users/jorgenoptima/code/projects/grail/uci.log"
    )]
    pub log_file: PathBuf,

    #[command(subcommand)]
    pub engines: Engines,
}

#[derive(Subcommand, Debug)]
pub enum Engines {
    Minimax {},
}
