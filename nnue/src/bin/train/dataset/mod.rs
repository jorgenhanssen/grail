mod loader;
mod progress;
mod shard;
mod shard_builder;
mod shard_reader;

use std::io;
use std::path::Path;
use tempfile::TempDir;

pub use loader::DataLoader;
pub use shard_builder::ShardStats;
pub use shard_reader::ShardReader;

use shard_builder::{build_shards, ShardPaths};

/// A sharded dataset ready for training.
///
/// Owns the temporary shard files and cleans them up on drop.
pub struct ShardedDataset {
    _temp_dir: TempDir,
    paths: ShardPaths,
    pub stats: ShardStats,
}

impl ShardedDataset {
    /// Builds shards from CSV files in the given directory.
    pub fn build(
        data_dir: &Path,
        shard_size_mb: usize,
        val_ratio: f64,
        test_ratio: f64,
    ) -> io::Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        log::info!("Building shards from {:?}...", data_dir);

        let (paths, stats) = build_shards(
            data_dir,
            temp_dir.path(),
            shard_size_mb,
            val_ratio,
            test_ratio,
        )?;

        stats.log();

        Ok(Self {
            _temp_dir: temp_dir,
            paths,
            stats,
        })
    }

    pub fn train_path(&self) -> &Path {
        &self.paths.train
    }

    pub fn val_path(&self) -> &Path {
        &self.paths.val
    }

    pub fn test_path(&self) -> &Path {
        &self.paths.test
    }
}
