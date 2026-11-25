use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(name = "NNUE Trainer")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Number of positions per training batch.
    #[arg(long, default_value_t = 16384)]
    pub batch_size: usize,

    /// Initial learning rate for optimizer.
    #[arg(long, default_value_t = 0.001)]
    pub learning_rate: f64,

    /// Maximum number of training epochs.
    #[arg(long, default_value_t = 200)]
    pub epochs: usize,

    /// Size of the sample buffer.
    #[arg(long, default_value_t = 1_000_000)]
    pub buffer_size: usize,

    /// Learning rate decay factor per epoch.
    #[arg(long, default_value_t = 0.95)]
    pub lr_decay: f64,

    /// Epochs without improvement before early stopping.
    #[arg(long, default_value_t = 5)]
    pub patience: u64,

    /// WDL blending weight (0.0 = pure eval, 1.0 = pure WDL).
    #[arg(long, default_value_t = 0.2)]
    pub wdl: f64,

    /// Target win-probability for drawn games (smaller = prefer wins over draws).
    #[arg(long, default_value_t = 0.5)]
    pub draw_target: f64,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Create a random model and reset training state.
    Init {
        /// Random seed for weights (GPU only), dataset read order and buffer shuffling.
        #[arg(long)]
        seed: Option<u64>,
    },
}
