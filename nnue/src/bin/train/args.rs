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
    #[arg(long, default_value_t = 100)]
    pub epochs: usize,

    /// Number of data loader workers.
    #[arg(long, default_value_t = 4)]
    pub workers: usize,

    /// Fraction of data for validation set.
    #[arg(long, default_value_t = 0.1)]
    pub val_ratio: f64,

    /// Fraction of data for test set.
    #[arg(long, default_value_t = 0.01)]
    pub test_ratio: f64,

    /// Learning rate decay factor per epoch.
    #[arg(long, default_value_t = 0.95)]
    pub lr_decay: f64,

    /// Epochs without improvement before early stopping.
    #[arg(long, default_value_t = 2)]
    pub patience: u64,
}
