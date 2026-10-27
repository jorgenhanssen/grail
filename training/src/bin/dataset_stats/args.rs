use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser)]
pub struct Args {
    /// Dataset folders to read. Defaults to all datasets.
    pub datasets: Vec<PathBuf>,

    /// Split to analyze. Defaults to all splits.
    #[arg(long, value_enum)]
    pub split: Option<Split>,

    /// Evaluation threshold in centipawns.
    #[arg(short, long, default_value_t = 600)]
    pub threshold: i32,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Split {
    Train,
    Val,
    Test,
}
