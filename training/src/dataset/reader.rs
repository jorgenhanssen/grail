use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::vec::IntoIter;

use cozy_chess::{Board, Move};
use rand::Rng;
use rand::seq::SliceRandom;

use super::{GameOutcome, TEST_FILE, TRAIN_FILE, TrainingSample, VAL_FILE};

pub struct DatasetReader {
    pub(super) path: PathBuf,
}

impl DatasetReader {
    pub(super) fn new(path: PathBuf) -> Self {
        Self { path }
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
    board: Option<Board>,
    line: String,
    bytes_read: u64,
    total_bytes: u64,
}

impl GameReader {
    pub(super) fn new(paths: Vec<PathBuf>) -> io::Result<Self> {
        let mut total_bytes = 0;
        for path in &paths {
            total_bytes += fs::metadata(path)?.len();
        }

        Ok(Self {
            paths: paths.into_iter(),
            reader: None,
            board: None,
            line: String::new(),
            bytes_read: 0,
            total_bytes,
        })
    }

    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub fn shuffle_files(&mut self, rng: &mut impl Rng) {
        self.paths.as_mut_slice().shuffle(rng);
    }

    fn next_game(&mut self) -> io::Result<Option<Vec<TrainingSample>>> {
        let mut samples = Vec::new();

        loop {
            if self.reader.is_none() {
                let Some(path) = self.paths.next() else {
                    return Ok(None);
                };
                let mut reader = BufReader::new(File::open(path)?);
                self.line.clear();
                self.bytes_read += reader.read_line(&mut self.line)? as u64;
                self.reader = Some(reader);
            }

            self.line.clear();
            let bytes = self.reader.as_mut().unwrap().read_line(&mut self.line)?;
            self.bytes_read += bytes as u64;

            if bytes == 0 {
                self.reader = None;
                if self.board.take().is_some() {
                    return Ok(Some(samples));
                }
                continue;
            }

            let mut fields = self.line.trim_end().split(',');
            let position = fields.next().unwrap();

            if !position.is_empty() {
                let board = position
                    .parse::<Board>()
                    .map_err(|error| io::Error::other(error.to_string()))?;
                if self.board.replace(board).is_some() {
                    return Ok(Some(samples));
                }
            } else {
                let mv = fields
                    .next()
                    .unwrap()
                    .parse::<Move>()
                    .map_err(|error| io::Error::other(error.to_string()))?;
                let score = fields.next().unwrap();
                let outcome = fields.next().unwrap();
                let board = self.board.as_mut().unwrap();

                if !score.is_empty() {
                    let score = score.parse().map_err(io::Error::other)?;
                    let outcome = match outcome {
                        "W" => GameOutcome::White,
                        "D" => GameOutcome::Draw,
                        "B" => GameOutcome::Black,
                        _ => return Err(io::Error::other("Unknown game outcome")),
                    };
                    samples.push(TrainingSample {
                        board: board.clone(),
                        score,
                        outcome,
                    });
                }

                board.play_unchecked(mv);
            }
        }
    }
}

impl Iterator for GameReader {
    type Item = io::Result<Vec<TrainingSample>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_game().transpose()
    }
}
