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

/// Lightweight index pointing to a sample's location in a file + score and game ID
/// TODO: Consider only storing file location and calculating score and game ID on the fly
#[derive(Debug, Clone, Copy)]
pub struct SampleIndex {
    pub byte_offset: u64,
    pub game_id: u32,
    pub score: i16,
    pub file_id: u8,
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
            (self.total_samples * std::mem::size_of::<SampleIndex>()) as f64 / BYTES_PER_MB
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

pub fn build_index(files: &[PathBuf]) -> io::Result<(Vec<SampleIndex>, IndexStats)> {
    let progress = IndexProgressBar::new(files);

    let results: Vec<io::Result<FileIndex>> = files
        .par_iter()
        .enumerate()
        .map(|(file_id, path)| FileIndex::new(file_id, path, &progress))
        .collect();

    progress.finish();

    log::info!("Finalizing index...");

    let (indices, stats) = merge_file_indicies(results)?;

    Ok((indices, stats))
}

struct FileIndex {
    indices: Vec<SampleIndex>,
    unique_hashes: AHashSet<u64>,
    max_game_id: u32,
}

impl FileIndex {
    fn new(file_id: usize, path: &Path, progress: &IndexProgressBar) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        let mut byte_offset: u64 = 0;

        let mut indices = Vec::new();
        let mut unique_hashes = AHashSet::new();
        let mut max_game_id: u32 = 0;
        let mut bytes_since_update: u64 = 0;
        let mut samples_since_update: usize = 0;

        // Read header
        if reader.read_line(&mut line)? > 0 {
            byte_offset += line.len() as u64;
            bytes_since_update += line.len() as u64;
            line.clear();
        }

        while reader.read_line(&mut line)? > 0 {
            let line_len = line.len() as u64;
            let trim_line = line.trim_end();

            if let Some((fen_str, score, game_id)) = parse_csv_line(trim_line) {
                let fen_len = fen_str.len();
                if fen_len <= MAX_FEN_LEN {
                    indices.push(SampleIndex {
                        file_id: file_id as u8,
                        byte_offset,
                        fen_len: fen_len as u8,
                        score,
                        game_id,
                    });

                    // Convert to u64 hash
                    let mut hasher = AHasher::default();
                    fen_str.hash(&mut hasher);
                    unique_hashes.insert(hasher.finish());
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
            indices,
            unique_hashes,
            max_game_id,
        })
    }
}

fn parse_csv_line(line: &str) -> Option<(&str, i16, u32)> {
    let comma_idx = line.find(',')?;
    let fen_str = &line[..comma_idx];
    let rest = &line[comma_idx + 1..];

    let second_comma = rest.find(',')?;
    let score_str = &rest[..second_comma];
    let game_id_str = &rest[second_comma + 1..];

    let score = score_str.parse::<i16>().ok()?;
    let game_id = game_id_str.parse::<u32>().ok()?;

    Some((fen_str, score, game_id))
}

fn merge_file_indicies(
    results: Vec<io::Result<FileIndex>>,
) -> io::Result<(Vec<SampleIndex>, IndexStats)> {
    let mut all_indices = Vec::new();
    let mut all_unique_hashes = AHashSet::new();
    let mut all_game_ids = AHashSet::new();
    let mut game_id_offset: u32 = 0;

    let mut max_offset: u64 = 0;
    let mut max_final_game_id: u32 = 0;

    for res in results {
        let FileIndex {
            mut indices,
            unique_hashes,
            max_game_id,
        } = res?;

        // Track max byte_offset in this file
        if let Some(idx) = indices.iter().max_by_key(|idx| idx.byte_offset) {
            max_offset = max_offset.max(idx.byte_offset);
        }

        // Offset all game IDs in this file's indices
        for idx in &mut indices {
            idx.game_id += game_id_offset;
            all_game_ids.insert(idx.game_id);
            max_final_game_id = max_final_game_id.max(idx.game_id);
        }

        all_indices.extend(indices);
        all_unique_hashes.extend(unique_hashes);

        // Update offset for next file (add 1 to avoid overlap)
        game_id_offset += max_game_id + 1;
    }

    let is_contiguous = check_contiguity(&all_indices);

    let stats = IndexStats {
        total_samples: all_indices.len(),
        unique_fens: all_unique_hashes.len(),
        total_games: all_game_ids.len(),
        is_contiguous,
    };

    Ok((all_indices, stats))
}

fn check_contiguity(indices: &[SampleIndex]) -> bool {
    // ensure each game_id appears in a single contiguous sequence

    let mut current_game_id = None;
    let mut seen_games = AHashSet::new();

    for sample in indices {
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
pub fn split_indices(
    indices: Vec<SampleIndex>,
    test_ratio: f64,
    val_ratio: f64,
) -> (Vec<SampleIndex>, Vec<SampleIndex>, Vec<SampleIndex>) {
    let mut game_map: AHashMap<u32, Vec<SampleIndex>> = AHashMap::new();
    for idx in indices {
        game_map.entry(idx.game_id).or_default().push(idx);
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

    let mut train_indices = Vec::new();
    let mut val_indices = Vec::new();
    let mut test_indices = Vec::new();

    for (gid, samples) in game_map {
        if test_set.contains(&gid) {
            test_indices.extend(samples);
        } else if val_set.contains(&gid) {
            val_indices.extend(samples);
        } else {
            train_indices.extend(samples);
        }
    }

    (train_indices, val_indices, test_indices)
}
