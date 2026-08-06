mod args;

use args::Args;
use clap::Parser;

fn main() {
    let args = Args::parse();
    let workers = args.workers.unwrap_or_else(num_cpus::get);

    println!("Hello SPSA tuner!");
    println!("params:  {}", args.params.display());
    println!("book:    {}", args.book.display());
    println!("games:   {}", args.games);
    println!("nodes:   {}", args.nodes);
    println!("workers: {workers}");
}
