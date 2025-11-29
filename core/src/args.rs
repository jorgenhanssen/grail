use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "Grail")]
#[command(author = "Jørgen Hanssen <jorgen@hanssen.io>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Args {
    /// Log UCI communication to a file for debugging.
    #[arg(short, long)]
    pub log_file: Option<PathBuf>,
}
