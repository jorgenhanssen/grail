use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "Tournament")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    #[arg(long, short = 'a')]
    pub engine_a: PathBuf,

    #[arg(long, short = 'b')]
    pub engine_b: PathBuf,

    #[command(subcommand)]
    pub time_control: TimeControlType,
}

#[derive(Subcommand, Debug, Clone)]
pub enum TimeControlType {
    #[command(name = "inf")]
    Infinite {
        #[arg(long)]
        move_time: u64,
    },

    #[command(name = "tc")]
    TimeControl {
        #[arg(long)]
        inc: u64,

        #[arg(long)]
        time: u64,
    },
}
