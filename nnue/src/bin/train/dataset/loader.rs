use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

use nnue::encoding::NUM_FEATURES;

use super::shard_reader::ShardReader;

const CHANNEL_BUFFER_MULTIPLIER: usize = 2;

pub struct Batch {
    pub features: Vec<f32>,
    pub scores: Vec<f32>,
    pub buckets: Vec<usize>,
    pub piece_targets: Vec<usize>,
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
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Batch>(num_workers * CHANNEL_BUFFER_MULTIPLIER);

        let workers: Vec<_> = (0..num_workers)
            .map(|_| {
                Self::spawn_worker(
                    Arc::clone(&reader),
                    sender.clone(),
                    Arc::clone(&shutdown),
                    batch_size,
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
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                let batch = Self::collect_batch(&reader, batch_size, &shutdown);

                if batch.scores.is_empty() || tx.send(batch).is_err() {
                    break;
                }
            }
        })
    }

    fn collect_batch(reader: &ShardReader, batch_size: usize, shutdown: &AtomicBool) -> Batch {
        let mut features = Vec::with_capacity(batch_size * NUM_FEATURES);
        let mut scores = Vec::with_capacity(batch_size);
        let mut buckets = Vec::with_capacity(batch_size);
        let mut piece_targets = Vec::with_capacity(batch_size);

        for _ in 0..batch_size {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            match reader.next() {
                Some(sample) => {
                    if let Some(encoded) = sample.encode() {
                        features.extend_from_slice(&encoded.features);
                        scores.push(encoded.score);
                        buckets.push(encoded.bucket);
                        piece_targets.push(encoded.piece_target);
                    }
                }
                None => break,
            }
        }

        Batch { features, scores, buckets, piece_targets }
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
