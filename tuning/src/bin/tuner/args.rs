use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "Grail Tuner")]
#[command(author = "Jørgen Hanssen")]
#[command(about = "SPSA tuning for Grail")]
pub struct Args {
    /// Path to config parameter file.
    #[arg(long, value_name = "TOML")]
    pub params: PathBuf,

    /// Game pairs per iteration.
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u64).range(1..))]
    pub pairs: u64,

    /// Soft node limit per move.
    #[arg(long, default_value_t = 10_000)]
    pub nodes: u64,

    /// Path to the opening book.
    #[arg(long, value_name = "EPD")]
    pub book: PathBuf,

    /// Abort as draw after this many plies.
    #[arg(long, default_value_t = 300)]
    pub max_plies: u64,

    /// Worker threads. Defaults to logical CPUs.
    #[arg(long)]
    pub workers: Option<u64>,

    /// SPSA ak
    /// https://www.chessprogramming.org/SPSA
    #[arg(long, default_value_t = 10.0)]
    pub ak: f64,

    /// Resign score (cp).
    #[arg(long, default_value_t = 400)]
    pub resign_score: i16,

    /// Resign movecount.
    #[arg(long, default_value_t = 3)]
    pub resign_moves: u64,

    /// Draw score (cp).
    #[arg(long, default_value_t = 10)]
    pub draw_score: i16,

    /// Draw movecount.
    #[arg(long, default_value_t = 8)]
    pub draw_moves: u64,

    /// Moves after opening before draw adj.
    #[arg(long, default_value_t = 40)]
    pub draw_after: u64,
}
