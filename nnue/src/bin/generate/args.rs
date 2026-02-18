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
    #[arg(long, default_value_t = 10)]
    pub depth: u8,

    /// Path to opening book file (EPD format).
    #[arg(long)]
    pub book: String,

    /// Number of PV lines to search at each decision point.
    #[arg(long, default_value_t = 3)]
    pub pv_lines: u8,

    /// Use existing NNUE for evaluation instead of HCE.
    #[arg(long, default_value_t = false)]
    pub nnue: bool,
}
