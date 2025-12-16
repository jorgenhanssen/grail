use ahash::AHashMap;
use hyperloglogplus::{HyperLogLog, HyperLogLogPlus};
use rayon::prelude::*;
use std::collections::hash_map::RandomState;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

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
}

impl ShardStats {
    pub fn log(&self) {
        log::info!("Total samples: {}", self.total_samples);
        log::info!(
            "Unique positions: ~{:.2}%",
            (self.unique_fens as f64 / self.total_samples as f64) * 100.0
        );
        log::info!("Total games: {}", self.total_games);
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
            writeln!(writer, "fen,score")?;
            writers.push(Mutex::new(writer));
        }

        Ok(Self {
            writers,
            next_idx: AtomicUsize::new(0),
        })
    }

    fn write(&self, fen: &str, score: i16) {
        // Round-robin to spread correlated samples
        let idx = self.next_idx.fetch_add(1, Ordering::Relaxed) % self.writers.len();
        let mut writer = self.writers[idx].lock().unwrap();
        if let Err(e) = writeln!(writer, "{},{}", fen, score) {
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
}

impl WorkerStats {
    fn new() -> Self {
        Self {
            samples: 0,
            train_samples: 0,
            games: 0,
            unique_fens: HyperLogLogPlus::new(HLL_PRECISION, RandomState::new()).unwrap(),
        }
    }

    fn register_sample(&mut self, fen: &str, split: Split) {
        self.samples += 1;

        if split == Split::Train {
            self.train_samples += 1;
        }

        self.unique_fens.insert(&fen.to_string());
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
        .map(|path| {
            process_file(
                path,
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
    let mut combined_hll: HyperLogLogPlus<String, RandomState> =
        HyperLogLogPlus::new(HLL_PRECISION, RandomState::new()).unwrap();

    for stats in worker_stats {
        total_samples += stats.samples;
        train_samples += stats.train_samples;
        total_games += stats.games;
        combined_hll.merge(&stats.unique_fens).unwrap();
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
    };

    Ok((shard_paths, stats))
}

fn process_file(
    path: &Path,
    val_ratio: f64,
    test_ratio: f64,
    train_writer: &ShardWriter,
    val_writer: &ShardWriter,
    test_writer: &ShardWriter,
    progress: &ShardProgressBar,
) -> WorkerStats {
    let mut stats = WorkerStats::new();
    let mut game_assignments: AHashMap<u32, Split> = AHashMap::new();
    let mut rng = rand::thread_rng();

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

        if let Some((fen, score, game_id)) = parse_csv_line(trimmed) {
            let split = *game_assignments.entry(game_id).or_insert_with(|| {
                stats.register_game();
                pick_split(&mut rng, val_ratio, test_ratio)
            });

            stats.register_sample(fen, split);

            match split {
                Split::Train => train_writer.write(fen, score),
                Split::Val => val_writer.write(fen, score),
                Split::Test => test_writer.write(fen, score),
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

fn parse_csv_line(line: &str) -> Option<(&str, i16, u32)> {
    let mut parts = line.split(',');
    let fen = parts.next()?;
    let score: i16 = parts.next()?.parse().ok()?;
    let game_id: u32 = parts.next()?.parse().ok()?;
    Some((fen, score, game_id))
}

fn pick_split<R: rand::Rng>(rng: &mut R, val_ratio: f64, test_ratio: f64) -> Split {
    let r: f64 = rng.gen();
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
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "csv"))
        .collect();

    files.sort();
    Ok(files)
}
