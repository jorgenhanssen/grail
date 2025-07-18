use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "NNUE Tournament")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    #[arg(long, default_value_t = 1000)]
    pub move_time: u64,

    #[arg(long)]
    pub a: PathBuf,

    #[arg(long)]
    pub b: PathBuf,
}
