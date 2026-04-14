use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "NNUE Trainer")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Number of positions per training batch.
    #[arg(long, default_value_t = 8192)]
    pub batch_size: usize,

    /// Initial learning rate for optimizer.
    #[arg(long, default_value_t = 0.001)]
    pub learning_rate: f64,

    /// Maximum number of training epochs.
    #[arg(long, default_value_t = 200)]
    pub epochs: usize,

    /// Number of data loader workers.
    #[arg(long, default_value_t = 4)]
    pub workers: usize,

    /// Fraction of data for validation set.
    #[arg(long, default_value_t = 0.05)]
    pub val_ratio: f64,

    /// Fraction of data for test set.
    #[arg(long, default_value_t = 0.01)]
    pub test_ratio: f64,

    /// Learning rate decay factor per epoch.
    #[arg(long, default_value_t = 0.95)]
    pub lr_decay: f64,

    /// Epochs without improvement before early stopping.
    #[arg(long, default_value_t = 5)]
    pub patience: u64,

    /// Size of each shard in megabytes.
    #[arg(long, default_value_t = 500)]
    pub shard_size_mb: usize,

    /// WDL weight far from game end (trust eval).
    #[arg(long, default_value_t = 0.2)]
    pub wdl_start: f64,

    /// WDL weight at game end (trust outcome).
    #[arg(long, default_value_t = 0.8)]
    pub wdl_end: f64,

    /// Save a randomly initialized model and exit.
    #[arg(long)]
    pub init_model: bool,
}
