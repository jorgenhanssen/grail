use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "NNUE Trainer")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    #[arg(long, default_value_t = 30)]
    pub epochs: usize,

    #[arg(long, default_value_t = 128)]
    pub batch_size: usize,
}
