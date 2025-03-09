use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::str::FromStr;

use candle_core::{Device, Result, Tensor};
use chess::Board;
use rand::rngs::ThreadRng;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

use crate::encoding::{encode_board, NUM_FEATURES};

#[derive(Clone, Debug)]
pub struct Samples {
    pub samples: HashMap<String, f32>,
}

impl Samples {
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
        }
    }

    pub fn from_evaluations(evals: &[(String, f32)]) -> Self {
        let samples = evals
            .iter()
            .map(|(fen, score)| (fen.clone(), *score))
            .collect();
        Self { samples }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "fen,score")?; // Header

        for (fen, score) in &self.samples {
            writeln!(writer, "{},{}", fen, score)?;
        }

        Ok(())
    }

    pub fn read<R: BufRead>(mut reader: R) -> io::Result<Self> {
        let mut samples = HashMap::new();

        // Skip header line
        let mut header_line = String::new();
        let _ = reader.read_line(&mut header_line)?;

        for line_res in reader.lines() {
            let line = line_res?;
            if line.trim().is_empty() {
                continue;
            }
            let mut parts = line.split(',');
            let fen_str = parts
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing fen field"))?;
            let score_str = parts
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing score field"))?;

            let score: f32 = score_str.trim().parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Score is not a valid float")
            })?;

            samples.insert(fen_str.trim().to_string(), score);
        }

        Ok(Self { samples })
    }

    pub fn to_xy(self, device: &Device) -> Result<(Tensor, Tensor)> {
        let num_samples = self.samples.len();

        let mut feature_data = Vec::with_capacity(num_samples * NUM_FEATURES);
        let mut score_data = Vec::with_capacity(num_samples);

        for (fen, score) in self.samples {
            let board =
                Board::from_str(&fen).unwrap_or_else(|_| panic!("Invalid FEN in sample: {}", fen));
            let features = encode_board(&board);
            feature_data.extend_from_slice(&features);
            score_data.push(score);
        }

        let x = Tensor::from_iter(feature_data.into_iter(), device)?
            .reshape((num_samples, NUM_FEATURES))?;
        let y = Tensor::from_iter(score_data.into_iter(), device)?.reshape((num_samples, 1))?;

        Ok((x, y))
    }

    pub fn to_xy_batched<'a>(
        &'a self,
        batch_size: usize,
        device: &'a Device,
    ) -> BatchedSamples<'a> {
        BatchedSamples {
            samples: &self.samples,
            device,
            batch_size,
            idx: 0,
            keys: self.samples.keys().collect(),
        }
    }

    pub fn train_test_split(
        &self,
        test_ratio: f64,
        random_seed: Option<u64>,
    ) -> (Samples, Samples) {
        let total_len = self.samples.len();
        let test_len = (total_len as f64 * test_ratio) as usize;
        let train_len = total_len - test_len;

        // Get all keys
        let mut keys: Vec<_> = self.samples.keys().collect();

        // Shuffle the keys
        if let Some(seed) = random_seed {
            let mut rng = StdRng::seed_from_u64(seed);
            keys.shuffle(&mut rng);
        }

        // Split keys
        let (train_keys, test_keys) = keys.split_at(train_len);

        // Create new HashMaps
        let train_samples = train_keys
            .iter()
            .map(|k| ((*k).clone(), *self.samples.get(*k).unwrap()))
            .collect();
        let test_samples = test_keys
            .iter()
            .map(|k| ((*k).clone(), *self.samples.get(*k).unwrap()))
            .collect();

        (
            Samples {
                samples: train_samples,
            },
            Samples {
                samples: test_samples,
            },
        )
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn shuffle(&mut self, rng: &mut ThreadRng) {
        let mut keys: Vec<_> = self.samples.keys().collect();
        keys.shuffle(rng);
        self.samples = keys
            .iter()
            .map(|k| ((*k).clone(), *self.samples.get(*k).unwrap()))
            .collect();
    }

    pub fn extend(&mut self, other: Samples) {
        for (key, value) in other.samples {
            if !self.samples.contains_key(&key) {
                self.samples.insert(key, value);
            }
        }
    }
}

pub struct BatchedSamples<'a> {
    samples: &'a HashMap<String, f32>,
    device: &'a Device,
    batch_size: usize,
    idx: usize,
    keys: Vec<&'a String>,
}

impl<'a> Iterator for BatchedSamples<'a> {
    type Item = Result<(Tensor, Tensor)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.keys.len() {
            return None;
        }
        let end = (self.idx + self.batch_size).min(self.keys.len());
        let batch_keys = &self.keys[self.idx..end];
        self.idx = end;

        let mut feature_data = Vec::with_capacity(batch_keys.len() * NUM_FEATURES);
        let mut score_data = Vec::with_capacity(batch_keys.len());

        for key in batch_keys {
            let score = self.samples.get(*key).unwrap();
            let board =
                Board::from_str(key).unwrap_or_else(|_| panic!("Invalid FEN in sample: {}", key));
            let features = encode_board(&board);
            feature_data.extend_from_slice(&features);
            score_data.push(*score);
        }

        let x = match Tensor::from_iter(feature_data.into_iter(), self.device) {
            Ok(t) => t,
            Err(e) => return Some(Err(e)),
        }
        .reshape((batch_keys.len(), NUM_FEATURES));

        let y = match Tensor::from_iter(score_data.into_iter(), self.device) {
            Ok(t) => t,
            Err(e) => return Some(Err(e)),
        }
        .reshape((batch_keys.len(), 1));

        Some(x.and_then(|xv| y.map(|yv| (xv, yv))))
    }
}
