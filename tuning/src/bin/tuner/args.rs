use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "Grail Tuner")]
#[command(author = "Jørgen Hanssen")]
#[command(about = "SPSA tuning for Grail")]
pub struct Args {
    #[arg(long, value_name = "TOML", help = "Path to config parameter file")]
    pub params: PathBuf,

    #[arg(long, default_value_t = 100, help = "Games per iteration")]
    pub games: usize,

    #[arg(long, default_value_t = 10_000, help = "Soft node limit per move")]
    pub nodes: u64,

    #[arg(long, value_name = "EPD", help = "Path to the opening book")]
    pub book: PathBuf,

    #[arg(long, help = "Number of parallel matches")]
    pub workers: Option<usize>,
}
