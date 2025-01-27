mod args;
mod generator;

use args::Args;
use chess::Board;
use clap::Parser;
use generator::Generator;
use log::LevelFilter;
use nnue::encode_board;
use simplelog::{Config, SimpleLogger};
use std::{error::Error, fs::File, io::Write};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let generator = Generator::new(num_cpus::get_physical());
    let samples = generator.run(args.duration, args.depth);

    log::info!("Generated {} samples", samples.len());

    let csv = samples_to_csv(&samples);

    // save file
    let mut file = File::create("samples.csv")?;
    file.write_all(csv.as_bytes())?;

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}

fn samples_to_csv(samples: &Vec<(Board, f32)>) -> String {
    let mut csv = String::new();
    for (board, score) in samples.iter() {
        let encoded = encode_board(&board);
        csv.push_str(&format!(
            "{},{}\n",
            score,
            encoded
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    csv
}
