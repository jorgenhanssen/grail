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
}

#[derive(Subcommand)]
pub enum Engines {
    Minimax {
        #[arg(long, default_value_t = 60)]
        duration: usize,

        #[arg(long, default_value_t = 3000)]
        iterations: usize,

        #[arg(long, default_value_t = 32)]
        epochs: usize,
    },
}
