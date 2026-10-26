use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use super::binary::write_game;
use super::{GameData, TEST_FILE, TRAIN_FILE, VAL_FILE};

pub struct DatasetWriter {
    train: SplitWriter,
    val: SplitWriter,
    test: SplitWriter,
}

impl DatasetWriter {
    pub(super) fn new(path: &Path, val_ratio: f64, test_ratio: f64) -> io::Result<Self> {
        let train_ratio = 1.0 - val_ratio - test_ratio;

        Ok(Self {
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

        let total = self.train.samples + self.val.samples + self.test.samples;

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
        split.samples += samples;

        Ok(())
    }
}

struct SplitWriter {
    writer: BufWriter<File>,
    samples: usize,
    ratio: f64,
}

impl SplitWriter {
    fn new(path: &Path, ratio: f64) -> io::Result<Self> {
        let writer = BufWriter::new(File::create(path)?);

        Ok(Self {
            writer,
            samples: 0,
            ratio,
        })
    }

    fn deficit(&self, total: usize) -> f64 {
        if self.ratio == 0.0 {
            return f64::NEG_INFINITY;
        }

        total as f64 * self.ratio - self.samples as f64
    }
}
