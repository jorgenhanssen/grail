use std::io::{self, BufRead, Write};
use std::str::FromStr;

use candle_core::{Device, Result, Tensor};
use chess::Board;
use rand::rngs::ThreadRng;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

use crate::encoding::{encode_board, NUM_FEATURES};
use crate::network::SCALE_FACTOR;

pub const CP_MAX: i16 = 5000;
pub const CP_MIN: i16 = -5000;

#[derive(Clone, Debug)]
pub struct Samples {
    pub fens: Vec<Box<str>>,
    pub scores: Vec<i16>,
}

impl Samples {
    pub fn new() -> Self {
        Self {
            fens: Vec::new(),
            scores: Vec::new(),
        }
    }

    pub fn from_evaluations(evals: &[(String, i16)]) -> Self {
        let mut fens = Vec::with_capacity(evals.len());
        let mut scores = Vec::with_capacity(evals.len());
        for (fen, score) in evals.iter() {
            fens.push(fen.clone().into_boxed_str());
            scores.push((*score).clamp(CP_MIN, CP_MAX));
        }
        Self { fens, scores }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "fen,score")?; // Header

        for i in 0..self.fens.len() {
            writeln!(writer, "{},{}", self.fens[i], self.scores[i])?;
        }

        Ok(())
    }

    pub fn read<R: BufRead>(mut reader: R) -> io::Result<Self> {
        let mut fens = Vec::new();
        let mut scores = Vec::new();

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

            let score: i16 = score_str.trim().parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Score is not a valid integer")
            })?;

            fens.push(fen_str.trim().to_string().into_boxed_str());
            scores.push(score.clamp(CP_MIN, CP_MAX));
        }

        Ok(Self { fens, scores })
    }

    pub fn to_xy(self, device: &Device) -> Result<(Tensor, Tensor)> {
        let num_samples = self.fens.len();

        let mut feature_data = Vec::with_capacity(num_samples * NUM_FEATURES);
        let mut score_data: Vec<f32> = Vec::with_capacity(num_samples);

        for (fen, score) in self.fens.into_iter().zip(self.scores.into_iter()) {
            let board =
                Board::from_str(&fen).unwrap_or_else(|_| panic!("Invalid FEN in sample: {}", fen));
            let features = encode_board(&board);
            feature_data.extend_from_slice(&features);
            // Scale the target by SCALE_FACTOR to normalize it
            score_data.push((score as f32) / SCALE_FACTOR);
        }

        let x = Tensor::from_iter(feature_data.into_iter(), device)?
            .reshape((num_samples, NUM_FEATURES))?;
        let y = Tensor::from_iter(score_data.into_iter(), device)?.reshape((num_samples, 1))?;

        Ok((x, y))
    }

    pub fn to_xy_batched_indices<'a>(
        &'a self,
        indices: &'a [usize],
        batch_size: usize,
        device: &'a Device,
    ) -> BatchedSamplesIdx<'a> {
        BatchedSamplesIdx {
            fens: &self.fens,
            scores: &self.scores,
            device,
            batch_size,
            idx: 0,
            indices,
        }
    }

    pub fn train_test_indices(
        &self,
        test_ratio: f64,
        random_seed: Option<u64>,
    ) -> (Vec<usize>, Vec<usize>) {
        let total_len = self.fens.len();
        let test_len = (total_len as f64 * test_ratio) as usize;
        let train_len = total_len - test_len;
        let mut idx: Vec<usize> = (0..total_len).collect();
        if let Some(seed) = random_seed {
            let mut rng = StdRng::seed_from_u64(seed);
            idx.shuffle(&mut rng);
        }
        let (train_idx, test_idx) = idx.split_at(train_len);
        (train_idx.to_vec(), test_idx.to_vec())
    }

    pub fn len(&self) -> usize {
        self.fens.len()
    }

    pub fn shuffle(&mut self, rng: &mut ThreadRng) {
        let mut idx: Vec<usize> = (0..self.fens.len()).collect();
        idx.shuffle(rng);

        let mut new_fens = Vec::with_capacity(self.fens.len());
        let mut new_scores = Vec::with_capacity(self.scores.len());
        for i in idx {
            new_fens.push(self.fens[i].clone());
            new_scores.push(self.scores[i]);
        }

        self.fens = new_fens;
        self.scores = new_scores;
    }

    pub fn extend(&mut self, other: Samples) {
        self.fens.extend(other.fens);
        self.scores.extend(other.scores);
    }
}

pub struct BatchedSamplesIdx<'a> {
    fens: &'a [Box<str>],
    scores: &'a [i16],
    device: &'a Device,
    batch_size: usize,
    idx: usize,
    indices: &'a [usize],
}

impl<'a> Iterator for BatchedSamplesIdx<'a> {
    type Item = Result<(Tensor, Tensor)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.indices.len() {
            return None;
        }
        let end = (self.idx + self.batch_size).min(self.indices.len());
        let batch_idx = &self.indices[self.idx..end];
        self.idx = end;

        let mut feature_data = Vec::with_capacity(batch_idx.len() * NUM_FEATURES);
        let mut score_data: Vec<f32> = Vec::with_capacity(batch_idx.len());

        for &i in batch_idx {
            let score = self.scores[i];
            let fen = &self.fens[i];
            let board =
                Board::from_str(fen).unwrap_or_else(|_| panic!("Invalid FEN in sample: {}", fen));
            let features = encode_board(&board);
            feature_data.extend_from_slice(&features);
            // Scale the target by SCALE_FACTOR to normalize it
            score_data.push((score as f32) / SCALE_FACTOR);
        }

        let x = match Tensor::from_iter(feature_data, self.device) {
            Ok(t) => t,
            Err(e) => return Some(Err(e)),
        }
        .reshape((batch_idx.len(), NUM_FEATURES));

        let y = match Tensor::from_iter(score_data, self.device) {
            Ok(t) => t,
            Err(e) => return Some(Err(e)),
        }
        .reshape((batch_idx.len(), 1));

        Some(x.and_then(|xv| y.map(|yv| (xv, yv))))
    }
}
