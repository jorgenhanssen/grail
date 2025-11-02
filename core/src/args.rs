use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "Grail")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = "0.1.0")]
pub struct Args {
    #[arg(short, long)]
    pub log_file: Option<PathBuf>,

    /// Run a benchmark search from the starting position to the specified depth
    #[arg(long)]
    pub bench: Option<u8>,
}
