use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "NNUE Data Generator")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Number of threads. Defaults to the number of logical CPUs.
    #[arg(long, global = true)]
    pub threads: Option<usize>,

    /// Search depth for position evaluation.
    #[arg(long, global = true, default_value_t = 8)]
    pub depth: u8,

    /// Soft node limit per position (finishes the ongoing iteration).
    #[arg(long, global = true, conflicts_with = "depth")]
    pub nodes: Option<u64>,

    /// Number of PV lines to search at each decision point.
    #[arg(long, global = true, default_value_t = 1)]
    pub pv_lines: u8,

    /// Colon-separated paths to Syzygy tablebase files.
    #[arg(long, global = true)]
    pub syzygy_path: Option<String>,

    /// Skip openings where the abs eval exceeds this (centipawns).
    #[arg(long, global = true)]
    pub max_opening_imbalance: Option<i16>,

    /// Max plies to teleport along a PV between recorded positions.
    #[arg(long, global = true, default_value_t = 8, value_parser = clap::value_parser!(u64).range(1..))]
    pub max_teleport_plies: u64,

    /// Discard games lasting longer than this many plies
    #[arg(long, global = true, default_value_t = 300, value_parser = clap::value_parser!(u64).range(1..))]
    pub max_game_plies: u64,

    /// Stop after this many games
    #[arg(long, global = true, value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..))]
    pub max_games: Option<usize>,

    /// Generate samples but don't write the dataset to disk.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Source openings used for self-play games.
    #[command(subcommand)]
    pub opening: Opening,
}

#[derive(Subcommand, Debug)]
pub enum Opening {
    /// Start from positions in an opening book.
    Book {
        /// Path to the opening book (EPD).
        #[arg(long)]
        path: String,
    },

    /// Start from startpos + `plies` random moves.
    Random {
        /// Number of random plies.
        #[arg(long, default_value_t = 8)]
        plies: usize,
    },
}
