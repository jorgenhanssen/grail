use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "NNUE Data Generator")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Number of threads. Defaults to the number of logical CPUs.
    #[arg(long)]
    pub threads: Option<usize>,

    /// Search depth for position evaluation.
    #[arg(long, default_value_t = 8)]
    pub depth: u8,

    /// Path to opening book file (EPD format).
    #[arg(long)]
    pub book: String,

    /// Number of PV lines to search at each decision point.
    #[arg(long, default_value_t = 1)]
    pub pv_lines: u8,

    /// Colon-separated paths to Syzygy tablebase files.
    #[arg(long)]
    pub syzygy_path: Option<String>,
}
