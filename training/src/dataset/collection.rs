use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::paths::DATA_DIR;

use super::{DatasetMetadata, DatasetReader, GameReader, TEST_FILE, TRAIN_FILE, VAL_FILE};

pub struct DatasetCollection {
    datasets: Vec<DatasetReader>,
}

impl DatasetCollection {
    pub fn open(paths: Vec<PathBuf>) -> Self {
        let mut datasets = Vec::new();
        for path in paths {
            datasets.push(DatasetReader::new(path));
        }

        Self { datasets }
    }

    pub fn all() -> io::Result<Self> {
        let mut paths = Vec::new();
        for entry in fs::read_dir(DATA_DIR)? {
            let path = entry?.path();
            if path.is_dir() {
                paths.push(path);
            }
        }
        paths.sort();

        Ok(Self::open(paths))
    }

    pub fn get(&self, name: &str) -> Option<&DatasetReader> {
        for dataset in &self.datasets {
            if dataset.path.file_name() == Some(OsStr::new(name)) {
                return Some(dataset);
            }
        }

        None
    }

    pub fn files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for dataset in &self.datasets {
            files.extend(dataset.files());
        }

        files
    }

    pub fn metadata(&self) -> io::Result<DatasetMetadata> {
        let mut total = DatasetMetadata::default();

        for dataset in &self.datasets {
            total += dataset.metadata()?;
        }

        Ok(total)
    }

    pub fn train_games(&self) -> GameReader {
        self.games(TRAIN_FILE)
    }

    pub fn val_games(&self) -> GameReader {
        self.games(VAL_FILE)
    }

    pub fn test_games(&self) -> GameReader {
        self.games(TEST_FILE)
    }

    fn games(&self, filename: &str) -> GameReader {
        let mut paths = Vec::new();
        for dataset in &self.datasets {
            paths.push(dataset.path.join(filename));
        }

        GameReader::new(paths)
    }
}
