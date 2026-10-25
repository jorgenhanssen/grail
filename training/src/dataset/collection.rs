use std::fs;
use std::io;
use std::path::Path;

use crate::paths::DATA_DIR;

use super::DatasetWriter;

pub struct DatasetCollection;

impl DatasetCollection {
    pub fn create(name: &str, val_ratio: f64, test_ratio: f64) -> io::Result<DatasetWriter> {
        let path = Path::new(DATA_DIR).join(name);

        fs::create_dir_all(DATA_DIR)?;
        fs::create_dir(&path)?;

        let writer = DatasetWriter::new(&path, val_ratio, test_ratio)?;
        log::info!("Writing samples to {}", path.display());

        Ok(writer)
    }
}
