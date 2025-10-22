use std::io::{self, BufRead, Write};
use std::str::FromStr;
use std::sync::Arc;
use std::thread;

use candle_core::{Device, Result, Tensor};
use chess::{Board, Color};
use rand::rngs::ThreadRng;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use utils::board_metrics::BoardMetrics;

use crate::encoding::{encode_board, NUM_FEATURES};

pub const CP_MAX: i16 = 5000;
pub const CP_MIN: i16 = -5000;

pub const TRAINING_SCALE: f32 = 400.0;

#[derive(Clone, Debug)]
pub struct Samples {
    pub fens: Vec<Box<str>>,
    pub scores: Vec<i16>,
    pub wdl: Vec<f32>, // Win/Draw/Loss: 1.0 = white win, 0.5 = draw, 0.0 = black win
}

impl Samples {
    pub fn new() -> Self {
        Self {
            fens: Vec::new(),
            scores: Vec::new(),
            wdl: Vec::new(),
        }
    }

    pub fn from_evaluations(evals: &[(String, i16, f32)]) -> Self {
        let mut fens = Vec::with_capacity(evals.len());
        let mut scores = Vec::with_capacity(evals.len());
        let mut wdl = Vec::with_capacity(evals.len());
        for (fen, score, outcome) in evals.iter() {
            fens.push(fen.clone().into_boxed_str());
            scores.push((*score).clamp(CP_MIN, CP_MAX));
            wdl.push(*outcome);
        }
        Self { fens, scores, wdl }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "fen,score,wdl")?; // Header

        for i in 0..self.fens.len() {
            writeln!(
                writer,
                "{},{},{}",
                self.fens[i], self.scores[i], self.wdl[i]
            )?;
        }

        Ok(())
    }

    pub fn read<R: BufRead>(mut reader: R) -> io::Result<Self> {
        let mut fens = Vec::new();
        let mut scores = Vec::new();
        let mut wdl = Vec::new();

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
            let wdl_str = parts
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing wdl field"))?;

            let score: i16 = score_str.trim().parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Score is not a valid integer")
            })?;
            let wdl_val: f32 = wdl_str.trim().parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "WDL is not a valid float")
            })?;

            fens.push(fen_str.trim().to_string().into_boxed_str());
            scores.push(score.clamp(CP_MIN, CP_MAX));
            wdl.push(wdl_val);
        }

        Ok(Self { fens, scores, wdl })
    }

    pub fn to_xy(self, device: &Device) -> Result<(Tensor, Tensor)> {
        let num_samples = self.fens.len();

        // Simple 768 features (single perspective)
        let mut feature_data = Vec::with_capacity(num_samples * NUM_FEATURES);
        let mut score_data: Vec<f32> = Vec::with_capacity(num_samples);

        for (fen, score) in self.fens.into_iter().zip(self.scores.into_iter()) {
            let board =
                Board::from_str(&fen).unwrap_or_else(|_| panic!("Invalid FEN in sample: {}", fen));

            // Compute tactical features
            let metrics = BoardMetrics::new(&board);
            let white_attacks = metrics.attacks[Color::White.to_index()];
            let black_attacks = metrics.attacks[Color::Black.to_index()];
            let white_support = metrics.support[Color::White.to_index()];
            let black_support = metrics.support[Color::Black.to_index()];

            let features = encode_board(
                &board,
                white_attacks,
                black_attacks,
                white_support,
                black_support,
            );
            feature_data.extend_from_slice(&features);
            score_data.push((score as f32) / TRAINING_SCALE);
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
        BatchedSamplesIdx::new(
            &self.fens,
            &self.scores,
            &self.wdl,
            indices,
            batch_size,
            device,
        )
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
        let mut new_wdl = Vec::with_capacity(self.wdl.len());
        for i in idx {
            new_fens.push(self.fens[i].clone());
            new_scores.push(self.scores[i]);
            new_wdl.push(self.wdl[i]);
        }

        self.fens = new_fens;
        self.scores = new_scores;
        self.wdl = new_wdl;
    }

    pub fn extend(&mut self, other: Samples) {
        self.fens.extend(other.fens);
        self.scores.extend(other.scores);
        self.wdl.extend(other.wdl);
    }
}

// Pre-encoded batch data
type EncodedBatch = (Vec<f32>, Vec<f32>, Vec<f32>, usize); // (features, scores, wdl, batch_size)

pub struct BatchedSamplesIdx<'a> {
    receiver: std::sync::mpsc::Receiver<Option<EncodedBatch>>,
    device: &'a Device,
    _workers: Vec<thread::JoinHandle<()>>,
}

impl<'a> BatchedSamplesIdx<'a> {
    fn new(
        fens: &'a [Box<str>],
        scores: &'a [i16],
        wdl: &'a [f32],
        indices: &'a [usize],
        batch_size: usize,
        device: &'a Device,
    ) -> Self {
        const BUFFER_SIZE: usize = 4; // Keep 2 batches prefetched (reduce memory)
        const NUM_WORKERS: usize = 4; // 4 encoding worker threads

        let (sender, receiver) = std::sync::mpsc::sync_channel(BUFFER_SIZE);

        // Share data across workers using Arc
        // Note: .to_vec() clones ~50GB of FEN strings, but needed for 'static lifetime
        let fens = Arc::new(fens.to_vec());
        let scores = Arc::new(scores.to_vec());
        let wdl = Arc::new(wdl.to_vec());
        let indices = Arc::new(indices.to_vec());

        // Shared atomic counter for work stealing
        let next_batch = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let total_batches = (indices.len() + batch_size - 1) / batch_size;

        let mut workers = Vec::new();

        for _ in 0..NUM_WORKERS {
            let sender = sender.clone();
            let fens = Arc::clone(&fens);
            let scores = Arc::clone(&scores);
            let wdl = Arc::clone(&wdl);
            let indices = Arc::clone(&indices);
            let next_batch = Arc::clone(&next_batch);

            let worker = thread::spawn(move || {
                loop {
                    // Atomically grab the next batch number
                    let batch_num = next_batch.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    if batch_num >= total_batches {
                        break; // No more work
                    }

                    let start_idx = batch_num * batch_size;
                    let end_idx = (start_idx + batch_size).min(indices.len());
                    let batch_indices = &indices[start_idx..end_idx];

                    // Encode batch on this worker thread (simple 768 features)
                    let mut feature_data = Vec::with_capacity(batch_indices.len() * NUM_FEATURES);
                    let mut score_data = Vec::with_capacity(batch_indices.len());
                    let mut wdl_data = Vec::with_capacity(batch_indices.len());

                    for &i in batch_indices {
                        let score = scores[i];
                        let wdl_val = wdl[i];
                        let fen = &fens[i];
                        let board = Board::from_str(fen)
                            .unwrap_or_else(|_| panic!("Invalid FEN in sample: {}", fen));

                        // Compute tactical features
                        let metrics = BoardMetrics::new(&board);
                        let white_attacks = metrics.attacks[Color::White.to_index()];
                        let black_attacks = metrics.attacks[Color::Black.to_index()];
                        let white_support = metrics.support[Color::White.to_index()];
                        let black_support = metrics.support[Color::Black.to_index()];

                        let features = encode_board(
                            &board,
                            white_attacks,
                            black_attacks,
                            white_support,
                            black_support,
                        );
                        feature_data.extend_from_slice(&features);
                        score_data.push((score as f32) / TRAINING_SCALE);
                        wdl_data.push(wdl_val);
                    }

                    // Send to channel (will block if channel is full)
                    if sender
                        .send(Some((
                            feature_data,
                            score_data,
                            wdl_data,
                            batch_indices.len(),
                        )))
                        .is_err()
                    {
                        break; // Receiver dropped
                    }
                }
            });

            workers.push(worker);
        }

        // Drop the original sender so the channel closes when workers finish
        drop(sender);

        Self {
            receiver,
            device,
            _workers: workers,
        }
    }
}

impl<'a> Iterator for BatchedSamplesIdx<'a> {
    type Item = Result<(Tensor, Tensor, Tensor)>;

    fn next(&mut self) -> Option<Self::Item> {
        // Receive pre-encoded batch from workers
        match self.receiver.recv() {
            Ok(Some((feature_data, score_data, wdl_data, batch_len))) => {
                let x = match Tensor::from_iter(feature_data, self.device) {
                    Ok(t) => t,
                    Err(e) => return Some(Err(e)),
                }
                .reshape((batch_len, NUM_FEATURES));

                let y = match Tensor::from_iter(score_data, self.device) {
                    Ok(t) => t,
                    Err(e) => return Some(Err(e)),
                }
                .reshape((batch_len, 1));

                let wdl = match Tensor::from_iter(wdl_data, self.device) {
                    Ok(t) => t,
                    Err(e) => return Some(Err(e)),
                }
                .reshape((batch_len, 1));

                Some(x.and_then(|xv| y.and_then(|yv| wdl.map(|wdl_v| (xv, yv, wdl_v)))))
            }
            Ok(None) | Err(_) => None,
        }
    }
}
