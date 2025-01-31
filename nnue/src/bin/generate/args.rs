use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "NNUE Data Generator")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    #[arg(long, default_value_t = 120)]
    pub duration: u64,

    #[arg(long, default_value_t = 5)]
    pub depth: u64,
}
