use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use nnue::network::EVAL_OUTPUT_BOUND;

use super::GameData;

const CSV_HEADER: &str = "position,move,score,outcome";

pub struct DatasetWriter {
    train: SplitWriter,
    val: SplitWriter,
    test: SplitWriter,
}

impl DatasetWriter {
    pub(super) fn new(path: &Path, val_ratio: f64, test_ratio: f64) -> io::Result<Self> {
        let train_ratio = 1.0 - val_ratio - test_ratio;

        Ok(Self {
            train: SplitWriter::new(&path.join("train.csv"), train_ratio)?,
            val: SplitWriter::new(&path.join("val.csv"), val_ratio)?,
            test: SplitWriter::new(&path.join("test.csv"), test_ratio)?,
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
        let mut writer = BufWriter::new(File::create(path)?);
        writeln!(writer, "{CSV_HEADER}")?;

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

fn write_game<W: Write>(writer: &mut W, game: &GameData) -> io::Result<()> {
    // each game starts with the opening
    writeln!(writer, "{:#},,,", game.start_position)?;

    for data in &game.moves {
        let mv = data.mv;
        let mut score = String::new();
        let mut outcome = String::new();

        if let Some(sample) = &data.sample {
            score = sample
                .score
                .clamp(-EVAL_OUTPUT_BOUND, EVAL_OUTPUT_BOUND)
                .to_string();
            outcome = sample.outcome.to_string();
        }

        // NB: omits the position (these are moves)
        writeln!(writer, ",{mv},{score},{outcome}")?;
    }

    writer.flush()
}
