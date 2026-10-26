use std::io;
use std::path::PathBuf;

use clap::Parser;
use training::dataset::{DatasetCollection, GameOutcome, GameReader, TrainingSample};

const EXAMPLES: usize = 5;

#[derive(Parser)]
#[command(about = "Analyze generated training datasets")]
struct Args {
    /// Binary files to read. Defaults to all datasets and splits.
    files: Vec<PathBuf>,

    /// Evaluation threshold in centipawns.
    #[arg(short, long, default_value_t = 600)]
    threshold: i32,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let threshold = args.threshold;

    let readers = if args.files.is_empty() {
        let datasets = DatasetCollection::open()?;
        vec![
            datasets.train_games()?,
            datasets.val_games()?,
            datasets.test_games()?,
        ]
    } else {
        vec![GameReader::new(args.files)?]
    };

    let mut total = 0;
    let mut draw_disagreements = 0;
    let mut decisive_disagreements = 0;
    let mut draw_examples = Vec::new();
    let mut decisive_examples = Vec::new();

    for reader in readers {
        for game in reader {
            let samples = game?;
            let mut draw_example = None;
            let mut decisive_example = None;

            total += samples.len();

            for sample in &samples {
                let score = i32::from(sample.score);

                if sample.outcome == GameOutcome::Draw && score.abs() > threshold {
                    draw_disagreements += 1;
                    if draw_example.is_none() {
                        draw_example = Some(sample);
                    }
                }

                let white_won_but_eval_black =
                    sample.outcome == GameOutcome::White && score < -threshold;
                let black_won_but_eval_white =
                    sample.outcome == GameOutcome::Black && score > threshold;

                if white_won_but_eval_black || black_won_but_eval_white {
                    decisive_disagreements += 1;
                    if decisive_example.is_none() {
                        decisive_example = Some(sample);
                    }
                }
            }

            add_example(&mut draw_examples, draw_example);
            add_example(&mut decisive_examples, decisive_example);
        }
    }

    println!("Threshold:             {threshold} cp");
    println!("Total samples:         {total}");

    print_examples(
        "Draw disagreements",
        draw_disagreements,
        total,
        &draw_examples,
    );
    print_examples(
        "Decisive opposites",
        decisive_disagreements,
        total,
        &decisive_examples,
    );

    Ok(())
}

fn add_example(examples: &mut Vec<String>, sample: Option<&TrainingSample>) {
    if examples.len() < EXAMPLES {
        if let Some(sample) = sample {
            examples.push(format!(
                "  {:+5}  {}  {:#}",
                sample.score, sample.outcome, sample.board
            ));
        }
    }
}

fn print_examples(title: &str, count: usize, total: usize, examples: &[String]) {
    let percentage = if total == 0 {
        0.0
    } else {
        100.0 * count as f64 / total as f64
    };

    println!("\n{title}: {count} ({percentage:.3}%)");
    for example in examples {
        println!("{example}");
    }
}
