use std::fs::{self, File};
use std::io::{self, BufReader, Seek};
use std::path::PathBuf;
use std::vec::IntoIter;

use rand::Rng;
use rand::seq::SliceRandom;

use super::binary::read_game;
use super::{DatasetMetadata, METADATA_FILE, TEST_FILE, TRAIN_FILE, TrainingSample, VAL_FILE};

pub struct DatasetReader {
    pub path: PathBuf,
}

impl DatasetReader {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn metadata(&self) -> io::Result<DatasetMetadata> {
        let file = File::open(self.path.join(METADATA_FILE))?;
        Ok(serde_json::from_reader(BufReader::new(file))?)
    }

    pub fn train_games(&self) -> io::Result<GameReader> {
        GameReader::new(vec![self.path.join(TRAIN_FILE)])
    }

    pub fn val_games(&self) -> io::Result<GameReader> {
        GameReader::new(vec![self.path.join(VAL_FILE)])
    }

    pub fn test_games(&self) -> io::Result<GameReader> {
        GameReader::new(vec![self.path.join(TEST_FILE)])
    }
}

pub struct GameReader {
    paths: IntoIter<PathBuf>,
    reader: Option<BufReader<File>>,
    completed_bytes: u64,
    total_bytes: u64,
}

impl GameReader {
    pub fn new(paths: Vec<PathBuf>) -> io::Result<Self> {
        let mut total_bytes = 0;
        for path in &paths {
            total_bytes += fs::metadata(path)?.len();
        }

        Ok(Self {
            paths: paths.into_iter(),
            reader: None,
            completed_bytes: 0,
            total_bytes,
        })
    }

    pub fn bytes_read(&mut self) -> io::Result<u64> {
        let current_bytes = match &mut self.reader {
            Some(reader) => reader.stream_position()?,
            None => 0,
        };

        Ok(self.completed_bytes + current_bytes)
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub fn shuffle_files(&mut self, rng: &mut impl Rng) {
        self.paths.as_mut_slice().shuffle(rng);
    }

    fn next_game(&mut self) -> io::Result<Option<Vec<TrainingSample>>> {
        loop {
            if self.reader.is_none() {
                let Some(path) = self.paths.next() else {
                    return Ok(None);
                };
                self.reader = Some(BufReader::new(File::open(path)?));
            }

            let reader = self.reader.as_mut().unwrap();
            let Some(game) = read_game(reader)? else {
                self.completed_bytes += reader.stream_position()?;
                self.reader = None;
                continue;
            };

            let mut board = game.start_position;
            let mut samples = Vec::new();

            for data in game.moves {
                if let Some(sample) = data.sample {
                    samples.push(TrainingSample {
                        board: board.clone(),
                        score: sample.score,
                        outcome: sample.outcome,
                    });
                }

                board.play_unchecked(data.mv);
            }

            return Ok(Some(samples));
        }
    }
}

impl Iterator for GameReader {
    type Item = io::Result<Vec<TrainingSample>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_game().transpose()
    }
}
