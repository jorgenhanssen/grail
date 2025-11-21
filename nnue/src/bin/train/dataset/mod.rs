mod indexer;
mod loader;
mod progress;

pub use indexer::{build_index, split_index, SampleRef};
pub use loader::DataLoader;

use rand::seq::SliceRandom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io};

use crate::args::Args;

pub struct Dataset {
    train_samples: Vec<SampleRef>,
    val_samples: Vec<SampleRef>,
    test_samples: Vec<SampleRef>,
    files: Arc<Vec<PathBuf>>,
}

impl Dataset {
    pub fn load(args: &Args, data_dir: &str) -> std::io::Result<Self> {
        log::info!("Loading data from {:?}...", data_dir);

        let files = get_files(Path::new(data_dir))?;

        let (index, stats) = build_index(&files)?;

        stats.log();

        let (train_samples, val_samples, test_samples) =
            split_index(index, args.test_ratio, args.val_ratio);

        Ok(Self {
            train_samples,
            val_samples,
            test_samples,
            files: Arc::new(files),
        })
    }

    pub fn train_loader(&mut self, batch_size: usize, workers: usize) -> DataLoader {
        self.train_samples.shuffle(&mut rand::thread_rng());
        DataLoader::new(&self.train_samples, &self.files, batch_size, workers)
    }

    pub fn val_loader(&self, batch_size: usize, workers: usize) -> DataLoader {
        DataLoader::new(&self.val_samples, &self.files, batch_size, workers)
    }

    pub fn test_loader(&self, batch_size: usize, workers: usize) -> DataLoader {
        DataLoader::new(&self.test_samples, &self.files, batch_size, workers)
    }
}

fn get_files(data_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(data_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "csv"))
        .collect();

    log::info!("Found {} CSV files", entries.len());

    // file_id is u8, so we can't have more than u8::MAX (255) files
    if entries.len() > u8::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Too many CSV files: found {}, but maximum is {}!",
                entries.len(),
                u8::MAX
            ),
        ));
    }

    entries.sort();

    Ok(entries)
}
