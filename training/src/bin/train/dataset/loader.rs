use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

use super::shard_reader::ShardReader;

const CHANNEL_BUFFER_MULTIPLIER: usize = 2;

pub struct Batch {
    pub stm_features: Vec<u32>,
    pub nstm_features: Vec<u32>,

    pub scores: Vec<f32>,
    pub outcomes: Vec<f32>,
    pub buckets: Vec<usize>,
}

/// Multi-threaded data loader that reads samples from shards.
///
/// Workers read samples from the ShardReader, encode them to features,
/// and send batches through a channel for training.
pub struct DataLoader {
    receiver: mpsc::Receiver<Batch>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl DataLoader {
    pub fn new(
        reader: Arc<ShardReader>,
        batch_size: usize,
        num_workers: usize,
        shutdown: Arc<AtomicBool>,
        draw_target: f32,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(num_workers * CHANNEL_BUFFER_MULTIPLIER);

        let workers: Vec<_> = (0..num_workers)
            .map(|_| {
                Self::spawn_worker(
                    Arc::clone(&reader),
                    sender.clone(),
                    Arc::clone(&shutdown),
                    batch_size,
                    draw_target,
                )
            })
            .collect();

        // Drop sender so receiver sees EOF when all workers finish
        drop(sender);

        Self { receiver, workers }
    }

    fn spawn_worker(
        reader: Arc<ShardReader>,
        tx: mpsc::SyncSender<Batch>,
        shutdown: Arc<AtomicBool>,
        batch_size: usize,
        draw_target: f32,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                let batch = Self::collect_batch(&reader, batch_size, &shutdown, draw_target);

                if batch.scores.is_empty() || tx.send(batch).is_err() {
                    break;
                }
            }
        })
    }

    fn collect_batch(
        reader: &ShardReader,
        batch_size: usize,
        shutdown: &AtomicBool,
        draw_target: f32,
    ) -> Batch {
        let mut samples = Vec::with_capacity(batch_size);

        for _ in 0..batch_size {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            match reader.next() {
                Some(sample) => {
                    if let Some(encoded) = sample.encode(draw_target) {
                        samples.push(encoded);
                    }
                }
                None => break,
            }
        }

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

        for (row, sample) in samples.into_iter().enumerate() {
            let row_end = (row + 1) * max_active_features;

            stm_features.extend(sample.stm_features);
            stm_features.resize(row_end, u32::MAX);

            nstm_features.extend(sample.nstm_features);
            nstm_features.resize(row_end, u32::MAX);

            scores.push(sample.score);
            outcomes.push(sample.outcome);
            buckets.push(sample.bucket);
        }

        Batch {
            stm_features,
            nstm_features,
            scores,
            outcomes,
            buckets,
        }
    }
}

impl Iterator for DataLoader {
    type Item = Batch;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.recv().ok()
    }
}

impl Drop for DataLoader {
    fn drop(&mut self) {
        while self.receiver.try_recv().is_ok() {}

        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
