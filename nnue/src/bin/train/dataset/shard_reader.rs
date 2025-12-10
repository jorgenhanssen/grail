use rand::seq::SliceRandom;
use rand::Rng;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::shard::{Sample, Shard};

/// Reads samples from multiple shards concurrently.
///
/// Maintains a pool of active shards that can be accessed in parallel.
/// When a shard is exhausted, it's replaced with the next pending shard.
pub struct ShardReader {
    /// Active shards - each independently lockable for parallel reads.
    shards: Vec<Mutex<Shard>>,

    /// Shard paths waiting to be loaded.
    pending: Mutex<Vec<PathBuf>>,
}

impl ShardReader {
    /// Creates a new ShardReader for the given shard directory.
    ///
    /// Shuffles shard order and loads the first `initial_shards` into memory.
    /// Remaining shards are loaded on-demand as active ones are exhausted.
    pub fn new(shard_dir: &Path, initial_shards: usize) -> io::Result<Self> {
        let mut paths: Vec<PathBuf> = fs::read_dir(shard_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "csv"))
            .collect();

        paths.shuffle(&mut rand::thread_rng());

        let to_load = paths.len().min(initial_shards);
        let mut shards = Vec::with_capacity(to_load);

        for path in paths.drain(..to_load) {
            match Shard::open(&path) {
                Ok(shard) => shards.push(Mutex::new(shard)),
                Err(e) => log::warn!("Failed to open shard {:?}: {}", path, e),
            }
        }

        Ok(Self {
            shards,
            pending: Mutex::new(paths),
        })
    }

    /// Gets the next sample, picking shards randomly for good mixing.
    pub fn next(&self) -> Option<Sample> {
        let num_shards = self.shards.len();
        if num_shards == 0 {
            return None;
        }

        let start = rand::thread_rng().gen_range(0..num_shards);

        for i in 0..num_shards {
            let idx = (start + i) % num_shards;

            if let Some(sample) = self.try_read_or_replace(idx) {
                return Some(sample);
            }
        }

        None
    }

    /// Tries to read from shard at idx, replacing it if exhausted.
    fn try_read_or_replace(&self, idx: usize) -> Option<Sample> {
        let mut shard = self.shards[idx].lock().unwrap();

        if let Some(sample) = shard.next() {
            return Some(sample);
        }

        // Shard exhausted - try to swap in a fresh one
        if let Some(new_shard) = self.pop_pending() {
            *shard = new_shard;
            return shard.next();
        }

        None
    }

    /// Pops the next available shard from pending queue.
    fn pop_pending(&self) -> Option<Shard> {
        let mut pending = self.pending.lock().unwrap();

        while let Some(path) = pending.pop() {
            match Shard::open(&path) {
                Ok(shard) => return Some(shard),
                Err(e) => log::warn!("Failed to open shard {:?}: {}", path, e),
            }
        }

        None
    }
}
