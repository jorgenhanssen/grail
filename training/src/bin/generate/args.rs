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

    /// Number of PV lines to search at each decision point.
    #[arg(long, global = true, default_value_t = 1)]
    pub pv_lines: u8,

    /// Colon-separated paths to Syzygy tablebase files.
    #[arg(long, global = true)]
    pub syzygy_path: Option<String>,

    /// Skip openings where the abs eval exceeds this (centipawns).
    #[arg(long, global = true)]
    pub max_opening_imbalance: Option<i16>,

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
