use std::io::{self, BufRead, Write};
use std::str::FromStr;

use candle_core::{Device, Result, Tensor};
use chess::Board;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

use crate::encoding::{encode_board, NUM_FEATURES};

#[derive(Clone, Debug)]
pub struct Sample {
    pub fen: String,
    pub score: f32,
}

impl Sample {
    pub fn from_board(board: &Board, score: f32) -> Self {
        let fen = board.to_string();
        Self { fen, score }
    }

    pub fn from_fen(fen: &str, score: f32) -> Self {
        Self {
            fen: fen.to_string(),
            score,
        }
    }

    pub fn as_features(&self) -> ([f32; NUM_FEATURES], f32) {
        let board = Board::from_str(&self.fen)
            .unwrap_or_else(|_| panic!("Invalid FEN in sample: {}", self.fen));

        // Encode into a feature array
        let encoded = encode_board(&board);

        (encoded, self.score)
    }
}

#[derive(Clone, Debug)]
pub struct Samples {
    pub samples: Vec<Sample>,
}

impl Samples {
    pub fn from_evaluations(evals: &[(String, f32)]) -> Self {
        let mut samples = Vec::with_capacity(evals.len());
        for (fen, score) in evals {
            samples.push(Sample::from_fen(fen, *score));
        }
        Self { samples }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "fen,score")?; // Header

        for sample in &self.samples {
            writeln!(writer, "{},{}", sample.fen, sample.score)?;
        }

        Ok(())
    }

    pub fn read<R: BufRead>(mut reader: R) -> io::Result<Self> {
        let mut samples = Vec::new();

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

            samples.push(Sample::from_fen(fen_str.trim(), score));
        }

        Ok(Self { samples })
    }

    pub fn to_xy(self, device: &Device) -> Result<(Tensor, Tensor)> {
        let num_samples = self.samples.len();

        let mut feature_data = Vec::with_capacity(num_samples * NUM_FEATURES);
        let mut score_data = Vec::with_capacity(num_samples);

        for sample in self.samples {
            let (features, score) = sample.as_features();
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

        // Shuffle the indices
        let mut indices: Vec<usize> = (0..total_len).collect();
        if let Some(seed) = random_seed {
            let mut rng = StdRng::seed_from_u64(seed);
            indices.shuffle(&mut rng);
        }

        // Split
        let (train_indices, test_indices) = indices.split_at(train_len);

        // Gather samples into new structs
        let train_samples: Vec<Sample> = train_indices
            .iter()
            .map(|&idx| self.samples[idx].clone())
            .collect();
        let test_samples: Vec<Sample> = test_indices
            .iter()
            .map(|&idx| self.samples[idx].clone())
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
}

pub struct BatchedSamples<'a> {
    samples: &'a [Sample],
    device: &'a Device,
    batch_size: usize,
    idx: usize,
}

impl<'a> Iterator for BatchedSamples<'a> {
    type Item = Result<(Tensor, Tensor)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.samples.len() {
            return None;
        }
        let end = (self.idx + self.batch_size).min(self.samples.len());
        let batch_slice = &self.samples[self.idx..end];
        self.idx = end;

        let mut feature_data = Vec::with_capacity(batch_slice.len() * NUM_FEATURES);
        let mut score_data = Vec::with_capacity(batch_slice.len());

        for sample in batch_slice {
            let (features, score) = sample.as_features();
            feature_data.extend_from_slice(&features);
            score_data.push(score);
        }

        let x = match Tensor::from_iter(feature_data.into_iter(), self.device) {
            Ok(t) => t,
            Err(e) => return Some(Err(e)),
        }
        .reshape((batch_slice.len(), NUM_FEATURES));

        let y = match Tensor::from_iter(score_data.into_iter(), self.device) {
            Ok(t) => t,
            Err(e) => return Some(Err(e)),
        }
        .reshape((batch_slice.len(), 1));

        Some(x.and_then(|xv| y.map(|yv| (xv, yv))))
    }
}
