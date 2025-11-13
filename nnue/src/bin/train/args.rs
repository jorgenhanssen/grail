use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "NNUE Trainer")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    #[arg(long, default_value_t = 100)]
    pub epochs: usize,

    #[arg(long, default_value_t = 8192)]
    pub batch_size: usize,

    #[arg(long, default_value_t = 0.001)]
    pub learning_rate: f64,

    #[arg(long, default_value_t = 0.95)]
    pub lr_decay: f64,

    #[arg(long, default_value_t = 2)]
    pub early_stop_patience: u64,

    #[arg(long, default_value_t = 0.1)]
    pub validation_split: f32,

    #[arg(long, default_value_t = 0.01)]
    pub test_split: f64,
}
