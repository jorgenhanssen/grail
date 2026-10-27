use std::collections::hash_map::RandomState;

use hyperloglogplus::{HyperLogLog, HyperLogLogPlus};
use nnue::network::{OUTPUT_BUCKETS, output_bucket};
use training::dataset::{GameData, GameOutcome, TrainingSample};

const EXAMPLES: usize = 5;
const HYPER_LOG_LOG_PRECISION: u8 = 18;

#[derive(Clone)]
pub struct DatasetStats {
    threshold: i32,
    total_samples: usize,
    total_games: usize,
    unique_fens: HyperLogLogPlus<String, RandomState>,
    bucket_counts: [usize; OUTPUT_BUCKETS],
    white_wins: usize,
    black_wins: usize,
    draws: usize,
    draw_disagreements: Disagreements,
    decisive_disagreements: Disagreements,
}

impl DatasetStats {
    pub fn new(threshold: i32, hash_builder: RandomState) -> Self {
        Self {
            threshold,
            total_samples: 0,
            total_games: 0,
            unique_fens: HyperLogLogPlus::new(HYPER_LOG_LOG_PRECISION, hash_builder).unwrap(),
            bucket_counts: [0; OUTPUT_BUCKETS],
            white_wins: 0,
            black_wins: 0,
            draws: 0,
            draw_disagreements: Disagreements::default(),
            decisive_disagreements: Disagreements::default(),
        }
    }

    pub fn record_game(&mut self, game_id: usize, game: GameData) {
        self.total_games += 1;

        for sample in game.into_samples() {
            self.total_samples += 1;
            self.unique_fens.insert(&format!("{:#}", sample.board));
            self.bucket_counts[output_bucket(&sample.board)] += 1;

            let score = i32::from(sample.score);

            match sample.outcome {
                GameOutcome::White => {
                    self.white_wins += 1;
                    if score < -self.threshold {
                        self.decisive_disagreements.record(game_id, &sample);
                    }
                }
                GameOutcome::Black => {
                    self.black_wins += 1;
                    if score > self.threshold {
                        self.decisive_disagreements.record(game_id, &sample);
                    }
                }
                GameOutcome::Draw => {
                    self.draws += 1;
                    if score.abs() > self.threshold {
                        self.draw_disagreements.record(game_id, &sample);
                    }
                }
            }
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.total_samples += other.total_samples;
        self.total_games += other.total_games;
        self.white_wins += other.white_wins;
        self.black_wins += other.black_wins;
        self.draws += other.draws;
        self.unique_fens.merge(&other.unique_fens).unwrap();

        for (count, other_count) in self.bucket_counts.iter_mut().zip(other.bucket_counts) {
            *count += other_count;
        }

        self.draw_disagreements.merge(other.draw_disagreements);
        self.decisive_disagreements
            .merge(other.decisive_disagreements);
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
            "Outcomes: white {:.1}% / black {:.1}% / draw {:.1}%",
            100.0 * self.white_wins as f64 / n,
            100.0 * self.black_wins as f64 / n,
            100.0 * self.draws as f64 / n,
        );

        println!("Output bucket distribution:");
        for (i, &count) in self.bucket_counts.iter().enumerate() {
            let percentage = 100.0 * count as f64 / n;
            println!("  Bucket {}: {} samples ({:.1}%)", i, count, percentage);
        }

        self.draw_disagreements
            .print("Draw disagreements", self.total_samples);
        self.decisive_disagreements
            .print("Decisive opposites", self.total_samples);
    }
}

#[derive(Clone, Default)]
struct Disagreements {
    count: usize,
    examples: Vec<Example>,
}

#[derive(Clone)]
struct Example {
    game_id: usize,
    line: String,
}

impl Disagreements {
    fn record(&mut self, game_id: usize, sample: &TrainingSample) {
        self.count += 1;

        if self.examples.len() == EXAMPLES {
            return;
        }
        if self
            .examples
            .last()
            .is_some_and(|example| example.game_id == game_id)
        {
            return;
        }

        self.examples.push(Example {
            game_id,
            line: format!(
                "  {:+5}  {}  {:#}",
                sample.score, sample.outcome, sample.board
            ),
        });
    }

    fn merge(&mut self, other: Self) {
        self.count += other.count;
        self.examples.extend(other.examples);
        self.examples
            .sort_unstable_by_key(|example| example.game_id);
        self.examples.truncate(EXAMPLES);
    }

    fn print(&self, title: &str, total: usize) {
        let percentage = 100.0 * self.count as f64 / total.max(1) as f64;

        println!("\n{title}: {} ({percentage:.3}%)", self.count);
        for example in &self.examples {
            println!("{}", example.line);
        }
    }
}
