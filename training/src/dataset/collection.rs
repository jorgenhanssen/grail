use std::fs;
use std::io;
use std::path::Path;

use crate::paths::DATA_DIR;

use super::{
    DatasetMetadata, DatasetReader, DatasetWriter, GameReader, TEST_FILE, TRAIN_FILE, VAL_FILE,
};

pub struct DatasetCollection {
    datasets: Vec<DatasetReader>,
}

impl DatasetCollection {
    pub fn open() -> io::Result<Self> {
        let mut paths = Vec::new();
        for entry in fs::read_dir(DATA_DIR)? {
            let path = entry?.path();
            if path.is_dir() {
                paths.push(path);
            }
        }
        paths.sort();

        let datasets = paths.into_iter().map(DatasetReader::new).collect();

        Ok(Self { datasets })
    }

    pub fn get(name: &str) -> io::Result<DatasetReader> {
        let path = Path::new(DATA_DIR).join(name);
        fs::metadata(&path)?;

        Ok(DatasetReader::new(path))
    }

    pub fn create(name: &str, val_ratio: f64, test_ratio: f64) -> io::Result<DatasetWriter> {
        let path = Path::new(DATA_DIR).join(name);

        fs::create_dir_all(DATA_DIR)?;
        fs::create_dir(&path)?;

        let writer = DatasetWriter::new(&path, val_ratio, test_ratio)?;
        log::info!("Writing samples to {}", path.display());

        Ok(writer)
    }

    pub fn metadata(&self) -> io::Result<DatasetMetadata> {
        let mut total = DatasetMetadata::default();

        for dataset in &self.datasets {
            total += dataset.metadata()?;
        }

        Ok(total)
    }

    pub fn train_games(&self) -> io::Result<GameReader> {
        self.games(TRAIN_FILE)
    }

    pub fn val_games(&self) -> io::Result<GameReader> {
        self.games(VAL_FILE)
    }

    pub fn test_games(&self) -> io::Result<GameReader> {
        self.games(TEST_FILE)
    }

    fn games(&self, filename: &str) -> io::Result<GameReader> {
        let paths = self
            .datasets
            .iter()
            .map(|dataset| dataset.path.join(filename))
            .collect();

        GameReader::new(paths)
    }
}
