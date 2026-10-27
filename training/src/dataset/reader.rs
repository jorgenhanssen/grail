use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::vec::IntoIter;

use rand::Rng;
use rand::seq::SliceRandom;

use super::binary::read_game;
use super::{DatasetMetadata, GameData, METADATA_FILE, TEST_FILE, TRAIN_FILE, VAL_FILE};

pub struct DatasetReader {
    pub path: PathBuf,
}

impl DatasetReader {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn files(&self) -> Vec<PathBuf> {
        vec![
            self.path.join(TRAIN_FILE),
            self.path.join(VAL_FILE),
            self.path.join(TEST_FILE),
        ]
    }

    pub fn metadata(&self) -> io::Result<DatasetMetadata> {
        let file = File::open(self.path.join(METADATA_FILE))?;
        Ok(serde_json::from_reader(BufReader::new(file))?)
    }

    pub fn train_games(&self) -> GameReader {
        GameReader::new(vec![self.path.join(TRAIN_FILE)])
    }

    pub fn val_games(&self) -> GameReader {
        GameReader::new(vec![self.path.join(VAL_FILE)])
    }

    pub fn test_games(&self) -> GameReader {
        GameReader::new(vec![self.path.join(TEST_FILE)])
    }
}

pub struct GameReader {
    paths: IntoIter<PathBuf>,
    reader: Option<BufReader<File>>,
}

impl GameReader {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths: paths.into_iter(),
            reader: None,
        }
    }

    pub fn shuffle_files(&mut self, rng: &mut impl Rng) {
        self.paths.as_mut_slice().shuffle(rng);
    }

    fn next_game(&mut self) -> io::Result<Option<GameData>> {
        loop {
            if self.reader.is_none() {
                let Some(path) = self.paths.next() else {
                    return Ok(None);
                };
                self.reader = Some(BufReader::new(File::open(path)?));
            }

            let reader = self.reader.as_mut().unwrap();
            let Some(game) = read_game(reader)? else {
                self.reader = None;
                continue;
            };

            return Ok(Some(game));
        }
    }
}

impl Iterator for GameReader {
    type Item = io::Result<GameData>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_game().transpose()
    }
}
