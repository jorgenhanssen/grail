use std::collections::hash_map::RandomState;

use hyperloglogplus::{HyperLogLog, HyperLogLogPlus};
use nnue::network::{OUTPUT_BUCKETS, output_bucket};
use training::dataset::{GameOutcome, TrainingSample};

const EXAMPLES: usize = 5;
const HYPER_LOG_LOG_PRECISION: u8 = 18;

pub struct DatasetStats {
    threshold: i32,
    total_samples: usize,
    total_games: usize,
    unique_fens: HyperLogLogPlus<String, RandomState>,
    bucket_counts: [usize; OUTPUT_BUCKETS],
    white_wins: usize,
    draws: usize,
    black_wins: usize,
    draw_disagreements: usize,
    decisive_disagreements: usize,
    draw_examples: Vec<String>,
    decisive_examples: Vec<String>,
}

impl DatasetStats {
    pub fn new(threshold: i32) -> Self {
        Self {
            threshold,
            total_samples: 0,
            total_games: 0,
            unique_fens: HyperLogLogPlus::new(HYPER_LOG_LOG_PRECISION, RandomState::new()).unwrap(),
            bucket_counts: [0; OUTPUT_BUCKETS],
            white_wins: 0,
            draws: 0,
            black_wins: 0,
            draw_disagreements: 0,
            decisive_disagreements: 0,
            draw_examples: Vec::new(),
            decisive_examples: Vec::new(),
        }
    }

    pub fn record_game(&mut self, samples: &[TrainingSample]) {
        self.total_games += 1;
        self.total_samples += samples.len();

        let mut draw_example = None;
        let mut decisive_example = None;

        for sample in samples {
            self.unique_fens.insert(&format!("{:#}", sample.board));
            self.bucket_counts[output_bucket(&sample.board)] += 1;

            match sample.outcome {
                GameOutcome::White => self.white_wins += 1,
                GameOutcome::Draw => self.draws += 1,
                GameOutcome::Black => self.black_wins += 1,
            }

            let score = i32::from(sample.score);

            if sample.outcome == GameOutcome::Draw && score.abs() > self.threshold {
                self.draw_disagreements += 1;
                if draw_example.is_none() {
                    draw_example = Some(sample);
                }
            }

            let white_won_but_eval_black =
                sample.outcome == GameOutcome::White && score < -self.threshold;
            let black_won_but_eval_white =
                sample.outcome == GameOutcome::Black && score > self.threshold;

            if white_won_but_eval_black || black_won_but_eval_white {
                self.decisive_disagreements += 1;
                if decisive_example.is_none() {
                    decisive_example = Some(sample);
                }
            }
        }

        add_example(&mut self.draw_examples, draw_example);
        add_example(&mut self.decisive_examples, decisive_example);
    }

    pub fn print(&mut self) {
        let n = self.total_samples.max(1) as f64;
        let unique_fens = self.unique_fens.count() as usize;

        println!("Threshold:             {} cp", self.threshold);
        println!("Total samples:         {}", self.total_samples);
        println!("Total games:           {}", self.total_games);
        println!(
            "Unique positions:      ~{:.2}%",
            unique_fens as f64 / n * 100.0
        );
        println!(
            "Outcomes: white {:.1}% / draw {:.1}% / black {:.1}%",
            100.0 * self.white_wins as f64 / n,
            100.0 * self.draws as f64 / n,
            100.0 * self.black_wins as f64 / n,
        );

        println!("Output bucket distribution:");
        for (i, &count) in self.bucket_counts.iter().enumerate() {
            let percentage = 100.0 * count as f64 / n;
            println!("  Bucket {}: {} samples ({:.1}%)", i, count, percentage);
        }

        print_examples(
            "Draw disagreements",
            self.draw_disagreements,
            self.total_samples,
            &self.draw_examples,
        );
        print_examples(
            "Decisive opposites",
            self.decisive_disagreements,
            self.total_samples,
            &self.decisive_examples,
        );
    }
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
