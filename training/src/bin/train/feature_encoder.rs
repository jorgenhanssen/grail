use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, sync_channel};
use std::thread::{self, JoinHandle};

use cozy_chess::Color;
use nnue::encoding::{FeatureIndices, encode_board_indices};
use nnue::network::{FV_SCALE, output_bucket};
use training::dataset::{GameOutcome, TrainingSample};
use utils::board_metrics::BoardMetrics;

use crate::data_loader::DataLoader;

const BATCH_QUEUE_SIZE: usize = 2;

pub struct FeatureEncoder {
    receiver: Option<Receiver<io::Result<Batch>>>,
    worker: Option<JoinHandle<()>>,
}

impl FeatureEncoder {
    pub fn new(
        loader: DataLoader,
        batch_size: usize,
        shutdown: Arc<AtomicBool>,
        draw_target: f32,
    ) -> Self {
        let (sender, receiver) = sync_channel(BATCH_QUEUE_SIZE);

        let worker = thread::spawn(move || {
            let mut samples = Vec::with_capacity(batch_size);

            for buffer in loader {
                let buffer = match buffer {
                    Ok(buffer) => buffer,
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        return;
                    }
                };

                for sample in buffer {
                    samples.push(encode_sample(&sample, draw_target));

                    if samples.len() == batch_size {
                        if shutdown.load(Ordering::Relaxed) {
                            return;
                        }

                        if sender.send(Ok(Batch::new(&samples))).is_err() {
                            return;
                        }

                        samples.clear();
                    }
                }
            }

            if !samples.is_empty() && !shutdown.load(Ordering::Relaxed) {
                let _ = sender.send(Ok(Batch::new(&samples)));
            }
        });

        Self {
            receiver: Some(receiver),
            worker: Some(worker),
        }
    }
}

impl Iterator for FeatureEncoder {
    type Item = io::Result<Batch>;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.as_ref()?.recv().ok()
    }
}

impl Drop for FeatureEncoder {
    fn drop(&mut self) {
        drop(self.receiver.take());

        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub struct Batch {
    pub stm_features: Vec<u32>,
    pub nstm_features: Vec<u32>,

    pub scores: Vec<f32>,
    pub outcomes: Vec<f32>,
    pub buckets: Vec<usize>,
}

impl Batch {
    fn new(samples: &[EncodedSample]) -> Self {
        // Find the sample with the most active features and pad all samples to this.
        let max_active_features = samples
            .iter()
            .map(|sample| sample.stm_features.len().max(sample.nstm_features.len()))
            .max()
            .unwrap_or(0);

        let mut stm_features = Vec::with_capacity(samples.len() * max_active_features);
        let mut nstm_features = Vec::with_capacity(samples.len() * max_active_features);
        let mut scores = Vec::with_capacity(samples.len());
        let mut outcomes = Vec::with_capacity(samples.len());
        let mut buckets = Vec::with_capacity(samples.len());

        for (row, sample) in samples.iter().enumerate() {
            let row_end = (row + 1) * max_active_features;

            stm_features.extend(&sample.stm_features);
            stm_features.resize(row_end, u32::MAX);

            nstm_features.extend(&sample.nstm_features);
            nstm_features.resize(row_end, u32::MAX);

            scores.push(sample.score);
            outcomes.push(sample.outcome);
            buckets.push(sample.bucket);
        }

        Self {
            stm_features,
            nstm_features,
            scores,
            outcomes,
            buckets,
        }
    }

    pub fn len(&self) -> usize {
        self.scores.len()
    }

    pub fn max_active_features(&self) -> usize {
        // Samples are padded equal, so we can just get the length of each sample by dividing by num batches.
        self.stm_features.len().checked_div(self.len()).unwrap_or(0)
    }
}

struct EncodedSample {
    stm_features: FeatureIndices,
    nstm_features: FeatureIndices,
    score: f32,
    outcome: f32,
    bucket: usize,
}

fn encode_sample(sample: &TrainingSample, draw_target: f32) -> EncodedSample {
    let board = &sample.board;
    let metrics = BoardMetrics::new(board);
    let stm = board.side_to_move();
    let nstm = !stm;

    let stm_features = encode_board_indices(board, &metrics, stm);
    let nstm_features = encode_board_indices(board, &metrics, nstm);

    let white_score = sample.score as f32 / FV_SCALE;
    let score = match stm {
        Color::White => white_score,
        Color::Black => -white_score,
    };
    let outcome = match (sample.outcome, stm) {
        (GameOutcome::Draw, _) => draw_target,
        (GameOutcome::White, Color::White) | (GameOutcome::Black, Color::Black) => 1.0,
        (GameOutcome::White, Color::Black) | (GameOutcome::Black, Color::White) => 0.0,
    };

    EncodedSample {
        stm_features,
        nstm_features,
        score,
        outcome,
        bucket: output_bucket(board),
    }
}
