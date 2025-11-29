use ahash::{AHashMap, AHashSet, AHasher};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::dataset::progress::IndexProgressBar;

pub const MAX_FEN_LEN: usize = 255;

const PROGRESS_UPDATE_INTERVAL: usize = 100_000;

/// Compact reference to a sample stored on disk.
///
/// Instead of loading full FEN strings into memory, we store compact references
/// pointing to each sample's location (file + byte offset). At training time,
/// we seek to that position and read just the FEN we need. The index can still
/// be large with millions of samples, but each entry is much smaller without the FEN.
#[derive(Debug, Clone, Copy)]
pub struct SampleRef {
    pub file_id: u8,
    pub byte_start: u64,
    pub game_id: u32,
    pub score: i16,
    pub fen_len: u8,
}

pub struct IndexStats {
    pub total_samples: usize,
    pub unique_fens: usize,
    pub total_games: usize,
    pub is_contiguous: bool,
}

impl IndexStats {
    pub fn log(&self) {
        const BYTES_PER_MB: f64 = 1_048_576.0;

        let unique_percentage = (self.unique_fens as f64 / self.total_samples as f64) * 100.0;

        log::info!(
            "Index size: {:.0} MB",
            (self.total_samples * std::mem::size_of::<SampleRef>()) as f64 / BYTES_PER_MB
        );

        log::info!("Total positions: {}", self.total_samples);
        log::info!("Unique positions: {:.2}%", unique_percentage);
        log::info!("Total games: {}", self.total_games);

        if self.is_contiguous {
            log::info!("All game IDs appear in contiguous sequences");
        } else {
            log::warn!("Warning: Some game IDs appear in non-contiguous sequences!");
        }
    }
}

pub fn build_index(files: &[PathBuf]) -> io::Result<(Vec<SampleRef>, IndexStats)> {
    let progress = IndexProgressBar::new(files);

    let results: Vec<io::Result<FileIndex>> = files
        .par_iter()
        .enumerate()
        .map(|(file_id, path)| FileIndex::new(file_id, path, &progress))
        .collect();

    progress.finish();

    log::info!("Finalizing index...");

    let (samples, stats) = merge_file_indicies(results)?;

    Ok((samples, stats))
}

struct FileIndex {
    samples: Vec<SampleRef>,
    unique_fens: AHashSet<u64>,
    max_game_id: u32,
}

impl FileIndex {
    fn new(file_id: usize, path: &Path, progress: &IndexProgressBar) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        let mut byte_offset: u64 = 0;

        let mut samples = Vec::new();
        let mut unique_fens = AHashSet::new();
        let mut max_game_id: u32 = 0;

        let mut bytes_since_update: u64 = 0;
        let mut samples_since_update: usize = 0;

        // Header
        if reader.read_line(&mut line)? > 0 {
            byte_offset += line.len() as u64;
            bytes_since_update += line.len() as u64;
            line.clear();
        }

        while reader.read_line(&mut line)? > 0 {
            let line_len = line.len() as u64;
            let trim_line = line.trim_end();

            if let Some((fen, score, game_id)) = parse_csv_line(trim_line) {
                let fen_len = fen.len();
                if fen_len <= MAX_FEN_LEN {
                    samples.push(SampleRef {
                        file_id: file_id as u8,
                        byte_start: byte_offset,
                        fen_len: fen_len as u8,
                        score,
                        game_id,
                    });

                    // Convert to u64 hash
                    let mut hasher = AHasher::default();
                    fen.hash(&mut hasher);
                    unique_fens.insert(hasher.finish());

                    max_game_id = max_game_id.max(game_id);
                    samples_since_update += 1;
                }
            }

            byte_offset += line_len;
            bytes_since_update += line_len;
            line.clear();

            if samples_since_update >= PROGRESS_UPDATE_INTERVAL {
                progress.update(bytes_since_update, samples_since_update);
                bytes_since_update = 0;
                samples_since_update = 0;
            }
        }

        // Push final update for remaining bytes/samples
        progress.update(bytes_since_update, samples_since_update);

        Ok(Self {
            samples,
            unique_fens,
            max_game_id,
        })
    }
}

fn parse_csv_line(line: &str) -> Option<(&str, i16, u32)> {
    let mut parts = line.split(',');

    let fen = parts.next()?;
    let score = parts.next()?.parse().ok()?;
    let game_id = parts.next()?.parse().ok()?;

    Some((fen, score, game_id))
}

fn merge_file_indicies(
    results: Vec<io::Result<FileIndex>>,
) -> io::Result<(Vec<SampleRef>, IndexStats)> {
    let mut all_samples = Vec::new();
    let mut all_unique_fen_hashes = AHashSet::new();
    let mut all_game_ids = AHashSet::new();
    let mut game_id_offset: u32 = 0;

    let mut max_offset: u64 = 0;
    let mut max_final_game_id: u32 = 0;

    for res in results {
        let FileIndex {
            mut samples,
            unique_fens,
            max_game_id,
        } = res?;

        // Track max byte_offset in this file
        if let Some(sample) = samples.iter().max_by_key(|r| r.byte_start) {
            max_offset = max_offset.max(sample.byte_start);
        }

        // Offset all game IDs in this file's sample refs
        for sample in &mut samples {
            sample.game_id += game_id_offset;
            all_game_ids.insert(sample.game_id);
            max_final_game_id = max_final_game_id.max(sample.game_id);
        }

        all_samples.extend(samples);
        all_unique_fen_hashes.extend(unique_fens);

        // Update offset for next file (add 1 to avoid overlap)
        game_id_offset += max_game_id + 1;
    }

    let is_contiguous = check_contiguity(&all_samples);

    let stats = IndexStats {
        total_samples: all_samples.len(),
        unique_fens: all_unique_fen_hashes.len(),
        total_games: all_game_ids.len(),
        is_contiguous,
    };

    Ok((all_samples, stats))
}

fn check_contiguity(samples: &[SampleRef]) -> bool {
    // ensure each game_id appears in a single contiguous sequence

    let mut current_game_id = None;
    let mut seen_games = AHashSet::new();

    for sample in samples {
        if Some(sample.game_id) != current_game_id {
            if seen_games.contains(&sample.game_id) {
                return false;
            }
            seen_games.insert(sample.game_id);
            current_game_id = Some(sample.game_id);
        }
    }

    true
}
/// Splits samples into train/val/test sets BY GAME, not by sample.
///
/// Critical: positions from the same game are highly correlated (similar structure,
/// consecutive moves). If we split randomly by sample, the model would see near-identical
/// positions in both train and val, causing inflated validation scores and overfitting.
///
/// By splitting at the game level, all positions from a game stay together in one set.
pub fn split_index(
    index: Vec<SampleRef>,
    test_ratio: f64,
    val_ratio: f64,
) -> (Vec<SampleRef>, Vec<SampleRef>, Vec<SampleRef>) {
    // Group samples by game_id
    let mut game_map: AHashMap<u32, Vec<SampleRef>> = AHashMap::new();
    for sample in index {
        game_map.entry(sample.game_id).or_default().push(sample);
    }

    let mut games: Vec<u32> = game_map.keys().copied().collect();

    games.shuffle(&mut thread_rng());

    let num_games = games.len();
    let num_test = (num_games as f64 * test_ratio) as usize;
    let num_val = (num_games as f64 * val_ratio) as usize;

    let test_games = &games[..num_test];
    let val_games = &games[num_test..num_test + num_val];

    let test_set: AHashSet<u32> = test_games.iter().copied().collect();
    let val_set: AHashSet<u32> = val_games.iter().copied().collect();

    let mut train = Vec::new();
    let mut val = Vec::new();
    let mut test = Vec::new();

    for (gid, samples) in game_map {
        if test_set.contains(&gid) {
            test.extend(samples);
        } else if val_set.contains(&gid) {
            val.extend(samples);
        } else {
            train.extend(samples);
        }
    }

    (train, val, test)
}
