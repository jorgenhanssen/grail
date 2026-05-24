use ahash::AHashMap;
use cozy_chess::Board;
use hyperloglogplus::{HyperLogLog, HyperLogLogPlus};
use nnue::network::{OUTPUT_BUCKETS, output_bucket};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::collections::hash_map::RandomState;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::progress::ShardProgressBar;

const PROGRESS_UPDATE_INTERVAL: usize = 100_000;
const HLL_PRECISION: u8 = 18; // Max precision for HyperLogLogPlus (~256KB per instance)

/// Paths to the train/val/test shard directories.
pub struct ShardPaths {
    pub train: PathBuf,
    pub val: PathBuf,
    pub test: PathBuf,
}

/// Statistics collected during shard building.
pub struct ShardStats {
    pub total_samples: usize,
    pub train_samples: usize,
    pub unique_fens: usize,
    pub total_games: usize,
    pub bucket_counts: [usize; OUTPUT_BUCKETS],
    pub white_wins: usize,
    pub draws: usize,
    pub black_wins: usize,
}

impl ShardStats {
    pub fn log(&self) {
        log::info!("Total samples: {}", self.total_samples);
        log::info!(
            "Unique positions: ~{:.2}%",
            (self.unique_fens as f64 / self.total_samples as f64) * 100.0
        );
        log::info!("Total games: {}", self.total_games);

        let n = self.total_samples as f64;
        log::info!(
            "Outcomes: white {:.1}% / draw {:.1}% / black {:.1}%",
            100.0 * self.white_wins as f64 / n,
            100.0 * self.draws as f64 / n,
            100.0 * self.black_wins as f64 / n,
        );

        log::info!("Output bucket distribution:");
        for (i, &count) in self.bucket_counts.iter().enumerate() {
            let percentage = 100.0 * count as f64 / n;
            log::info!("  Bucket {}: {} samples ({:.1}%)", i, count, percentage);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Split {
    Train,
    Val,
    Test,
}

/// Writes samples to multiple shard files with round-robin distribution.
struct ShardWriter {
    writers: Vec<Mutex<BufWriter<File>>>,
    next_idx: AtomicUsize,
}

impl ShardWriter {
    fn new(dir: &Path, count: usize) -> io::Result<Self> {
        fs::create_dir_all(dir)?;

        let mut writers = Vec::with_capacity(count);
        for i in 0..count {
            let path = dir.join(format!("shard_{}.csv", i));
            let file = File::create(&path)?;
            let mut writer = BufWriter::new(file);
            writeln!(writer, "fen,score,outcome")?;
            writers.push(Mutex::new(writer));
        }

        Ok(Self {
            writers,
            next_idx: AtomicUsize::new(0),
        })
    }

    fn write(&self, fen: &str, score: i16, outcome: &str) {
        let idx = self.next_idx.fetch_add(1, Ordering::Relaxed) % self.writers.len();
        let mut writer = self.writers[idx].lock().unwrap();
        if let Err(e) = writeln!(writer, "{},{},{}", fen, score, outcome) {
            log::error!("Failed to write to shard: {}", e);
        }
    }

    /// Flushes all writers to disk. Call after processing is complete.
    fn flush_all(&self) -> io::Result<()> {
        for writer in &self.writers {
            writer.lock().unwrap().flush()?;
        }
        Ok(())
    }
}

/// Per-worker statistics
struct WorkerStats {
    samples: usize,
    train_samples: usize,
    games: usize,
    unique_fens: HyperLogLogPlus<String, RandomState>,
    bucket_counts: [usize; OUTPUT_BUCKETS],
    white_wins: usize,
    draws: usize,
    black_wins: usize,
}

impl WorkerStats {
    fn new() -> Self {
        Self {
            samples: 0,
            train_samples: 0,
            games: 0,
            unique_fens: HyperLogLogPlus::new(HLL_PRECISION, RandomState::new()).unwrap(),
            bucket_counts: [0; OUTPUT_BUCKETS],
            white_wins: 0,
            draws: 0,
            black_wins: 0,
        }
    }

    fn register_sample(&mut self, fen: &str, outcome: &str, split: Split) {
        self.samples += 1;

        if split == Split::Train {
            self.train_samples += 1;
        }

        match outcome {
            "W" => self.white_wins += 1,
            "D" => self.draws += 1,
            "B" => self.black_wins += 1,
            _ => log::warn!("Unknown outcome: {}", outcome),
        }

        self.unique_fens.insert(&fen.to_string());

        if let Ok(board) = Board::from_str(fen) {
            let bucket = output_bucket(&board);
            self.bucket_counts[bucket] += 1;
        }
    }

    fn register_game(&mut self) {
        self.games += 1;
    }
}

/// Builds shards from CSV data files in a single streaming pass.
///
/// Games are assigned to train/val/test probabilistically based on ratios.
/// Samples are distributed across shards via round-robin to spread correlated
/// positions from the same game.
pub fn build_shards(
    data_dir: &Path,
    temp_dir: &Path,
    shard_size_mb: usize,
    val_ratio: f64,
    test_ratio: f64,
    seed: u64,
) -> io::Result<(ShardPaths, ShardStats)> {
    let files = get_csv_files(data_dir)?;
    log::info!("Found {} CSV files to process", files.len());

    // Calculate total size and number of shards needed
    let total_size: u64 = files
        .iter()
        .map(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        .sum();

    let shard_size_bytes = (shard_size_mb as u64) * 1024 * 1024;
    let train_ratio = 1.0 - val_ratio - test_ratio;

    let num_train_shards = calculate_shard_count(total_size, shard_size_bytes, train_ratio);
    let num_val_shards = calculate_shard_count(total_size, shard_size_bytes, val_ratio);
    let num_test_shards = calculate_shard_count(total_size, shard_size_bytes, test_ratio);

    let train_dir = temp_dir.join("train");
    let val_dir = temp_dir.join("val");
    let test_dir = temp_dir.join("test");

    let train_writer = ShardWriter::new(&train_dir, num_train_shards)?;
    let val_writer = ShardWriter::new(&val_dir, num_val_shards)?;
    let test_writer = ShardWriter::new(&test_dir, num_test_shards)?;

    let progress = ShardProgressBar::new(&files);

    let worker_stats: Vec<WorkerStats> = files
        .par_iter()
        .enumerate()
        .map(|(idx, path)| {
            process_file(
                idx,
                path,
                seed,
                val_ratio,
                test_ratio,
                &train_writer,
                &val_writer,
                &test_writer,
                &progress,
            )
        })
        .collect();

    progress.finish();

    // Merge statistics
    let mut total_samples = 0;
    let mut train_samples = 0;
    let mut total_games = 0;
    let mut bucket_counts = [0usize; OUTPUT_BUCKETS];
    let mut white_wins = 0;
    let mut draws = 0;
    let mut black_wins = 0;
    let mut combined_hll: HyperLogLogPlus<String, RandomState> =
        HyperLogLogPlus::new(HLL_PRECISION, RandomState::new()).unwrap();

    for stats in worker_stats {
        total_samples += stats.samples;
        train_samples += stats.train_samples;
        total_games += stats.games;
        white_wins += stats.white_wins;
        draws += stats.draws;
        black_wins += stats.black_wins;
        combined_hll.merge(&stats.unique_fens).unwrap();

        for (i, count) in stats.bucket_counts.iter().enumerate() {
            bucket_counts[i] += count;
        }
    }

    let unique_fens_count = combined_hll.count() as usize;

    train_writer.flush_all()?;
    val_writer.flush_all()?;
    test_writer.flush_all()?;

    let shard_paths = ShardPaths {
        train: train_dir,
        val: val_dir,
        test: test_dir,
    };

    let stats = ShardStats {
        total_samples,
        train_samples,
        unique_fens: unique_fens_count,
        total_games,
        bucket_counts,
        white_wins,
        draws,
        black_wins,
    };

    Ok((shard_paths, stats))
}

fn process_file(
    file_index: usize,
    path: &Path,
    seed: u64,
    val_ratio: f64,
    test_ratio: f64,
    train_writer: &ShardWriter,
    val_writer: &ShardWriter,
    test_writer: &ShardWriter,
    progress: &ShardProgressBar,
) -> WorkerStats {
    let mut stats = WorkerStats::new();
    let mut game_assignments: AHashMap<u32, Split> = AHashMap::new();
    let mut rng = StdRng::seed_from_u64(seed.wrapping_add(file_index as u64));

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Failed to open {:?}: {}", path, e);
            return stats;
        }
    };

    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut bytes_since_update: u64 = 0;
    let mut samples_since_update: usize = 0;

    // Skip header
    if reader.read_line(&mut line).is_ok() {
        bytes_since_update += line.len() as u64;
        line.clear();
    }

    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        let line_len = line.len() as u64;
        let trimmed = line.trim();

        if let Some((fen, score, outcome, game_id)) = parse_csv_line(trimmed) {
            let split = *game_assignments.entry(game_id).or_insert_with(|| {
                stats.register_game();
                pick_split(&mut rng, val_ratio, test_ratio)
            });

            stats.register_sample(fen, outcome, split);

            match split {
                Split::Train => train_writer.write(fen, score, outcome),
                Split::Val => val_writer.write(fen, score, outcome),
                Split::Test => test_writer.write(fen, score, outcome),
            }

            samples_since_update += 1;
        }

        bytes_since_update += line_len;
        line.clear();

        if samples_since_update >= PROGRESS_UPDATE_INTERVAL {
            progress.update(bytes_since_update);
            bytes_since_update = 0;
            samples_since_update = 0;
        }
    }

    progress.update(bytes_since_update);

    stats
}

fn parse_csv_line(line: &str) -> Option<(&str, i16, &str, u32)> {
    let mut parts = line.split(',');
    let fen = parts.next()?;
    let score: i16 = parts.next()?.parse().ok()?;
    let _best_move = parts.next()?;
    let outcome = parts.next()?;
    let game_id: u32 = parts.next()?.parse().ok()?;
    Some((fen, score, outcome, game_id))
}

fn pick_split<R: rand::Rng>(rng: &mut R, val_ratio: f64, test_ratio: f64) -> Split {
    let r: f64 = rng.random();
    if r < test_ratio {
        Split::Test
    } else if r < test_ratio + val_ratio {
        Split::Val
    } else {
        Split::Train
    }
}

fn calculate_shard_count(total_bytes: u64, shard_size: u64, ratio: f64) -> usize {
    let split_bytes = (total_bytes as f64 * ratio) as u64;
    ((split_bytes / shard_size) + 1).max(1) as usize
}

fn get_csv_files(data_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(data_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "csv"))
        .collect();

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VAL_RATIO: f64 = 0.2;
    const TEST_RATIO: f64 = 0.1;
    const TRAIN_RATIO: f64 = 1.0 - VAL_RATIO - TEST_RATIO;

    fn split_sequence(seed: u64, n: usize) -> Vec<Split> {
        let mut rng = StdRng::seed_from_u64(seed);
        (0..n)
            .map(|_| pick_split(&mut rng, VAL_RATIO, TEST_RATIO))
            .collect()
    }

    #[test]
    fn pick_split_distribution_matches_ratios() {
        let n = 50_000;
        let (mut train, mut val, mut test) = (0, 0, 0);
        for split in split_sequence(42, n) {
            match split {
                Split::Train => train += 1,
                Split::Val => val += 1,
                Split::Test => test += 1,
            }
        }
        let n = n as f64;
        assert!((train as f64 / n - TRAIN_RATIO).abs() < 0.02);
        assert!((val as f64 / n - VAL_RATIO).abs() < 0.02);
        assert!((test as f64 / n - TEST_RATIO).abs() < 0.02);
    }
}
