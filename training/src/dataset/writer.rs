use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use super::binary::write_game;
use super::{
    DatasetMetadata, GameData, METADATA_FILE, SplitMetadata, TEST_FILE, TRAIN_FILE, VAL_FILE,
};

pub struct DatasetWriter {
    path: PathBuf,
    train: SplitWriter,
    val: SplitWriter,
    test: SplitWriter,
}

impl DatasetWriter {
    pub fn new(path: &Path, val_ratio: f64, test_ratio: f64) -> io::Result<Self> {
        let train_ratio = 1.0 - val_ratio - test_ratio;

        Ok(Self {
            path: path.to_path_buf(),
            train: SplitWriter::new(&path.join(TRAIN_FILE), train_ratio)?,
            val: SplitWriter::new(&path.join(VAL_FILE), val_ratio)?,
            test: SplitWriter::new(&path.join(TEST_FILE), test_ratio)?,
        })
    }

    pub fn write_game(&mut self, game: &GameData) -> io::Result<()> {
        if game.moves.is_empty() {
            return Ok(());
        }

        let samples = game
            .moves
            .iter()
            .filter(|data| data.sample.is_some())
            .count();

        let total =
            self.train.metadata.samples + self.val.metadata.samples + self.test.metadata.samples;

        let train_deficit = self.train.deficit(total);
        let val_deficit = self.val.deficit(total);
        let test_deficit = self.test.deficit(total);

        let split = if train_deficit >= val_deficit && train_deficit >= test_deficit {
            &mut self.train
        } else if val_deficit >= test_deficit {
            &mut self.val
        } else {
            &mut self.test
        };

        write_game(&mut split.writer, game)?;
        split.writer.flush()?;
        split.metadata.samples += samples;
        split.metadata.games += 1;

        self.save_metadata()
    }

    pub fn save_metadata(&self) -> io::Result<()> {
        let metadata = DatasetMetadata {
            train: self.train.metadata,
            val: self.val.metadata,
            test: self.test.metadata,
        };
        let file = File::create(self.path.join(METADATA_FILE))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &metadata)?;

        writer.flush()
    }
}

struct SplitWriter {
    writer: BufWriter<File>,
    metadata: SplitMetadata,
    ratio: f64,
}

impl SplitWriter {
    fn new(path: &Path, ratio: f64) -> io::Result<Self> {
        let writer = BufWriter::new(File::create(path)?);

        Ok(Self {
            writer,
            metadata: SplitMetadata::default(),
            ratio,
        })
    }

    fn deficit(&self, total: usize) -> f64 {
        if self.ratio == 0.0 {
            return f64::NEG_INFINITY;
        }

        total as f64 * self.ratio - self.metadata.samples as f64
    }
}
