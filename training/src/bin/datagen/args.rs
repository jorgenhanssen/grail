use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "NNUE Data Generator")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Number of threads. Defaults to the number of logical CPUs.
    #[arg(long)]
    pub threads: Option<usize>,

    /// Fraction of data for validation set.
    #[arg(long, default_value_t = 0.05)]
    pub val_ratio: f64,

    /// Fraction of data for test set.
    #[arg(long, default_value_t = 0.01)]
    pub test_ratio: f64,

    /// Soft node limit for the scout/shallow search.
    #[arg(long, default_value_t = 3000)]
    pub scout_nodes: u64,

    /// Base chance of selecting a scout position for an eval search.
    #[arg(long, default_value_t = 0.05)]
    pub base_chance: f64,

    /// Increases chance of sampling a position if the features have changed since the last sample.
    #[arg(long, default_value_t = 0.5)]
    pub feature_delta_weight: f64,

    /// Increases chance of sampling a position if the eval has changed since the last sample.
    #[arg(long, default_value_t = 0.5)]
    pub eval_delta_weight: f64,

    /// Increases chance of sampling a position if the static and scout eval disagree.
    #[arg(long, default_value_t = 0.5)]
    pub disagreement_weight: f64,

    /// Soft node limit for the eval search.
    #[arg(long, default_value_t = 25000)]
    pub eval_nodes: u64,

    /// Number of PV lines to search at each decision point.
    #[arg(long, default_value_t = 1)]
    pub pv_lines: u8,

    /// Colon-separated paths to Syzygy tablebase files.
    #[arg(long)]
    pub syzygy_path: Option<String>,

    /// Skip openings where the abs eval exceeds this (centipawns).
    #[arg(long)]
    pub max_opening_imbalance: Option<i16>,

    /// Discard games lasting longer than this many plies
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(1..))]
    pub max_game_plies: u64,

    /// Stop after this many games
    #[arg(long, value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..))]
    pub max_games: Option<usize>,

    /// Generate samples but don't write the dataset to disk.
    #[arg(long)]
    pub dry_run: bool,

    /// Path to an EPD opening book. If omitted, games start from the standard startpos.
    #[arg(long)]
    pub book: Option<String>,

    /// Random legal plies from the selected position.
    #[arg(long, default_value_t = 0)]
    pub random_plies: usize,
}
