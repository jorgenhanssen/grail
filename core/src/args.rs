use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "Grail")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    #[arg(short, long)]
    pub log_file: Option<PathBuf>,

    #[command(subcommand)]
    pub engines: Engines,
}

#[derive(Subcommand, Debug)]
pub enum Engines {
    Negamax {},
}
