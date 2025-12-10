use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use nnue::encoding::NUM_FEATURES;

use super::shard_reader::ShardReader;

const CHANNEL_BUFFER_MULTIPLIER: usize = 2;

type BatchData = (Vec<f32>, Vec<f32>);

/// Multi-threaded data loader that reads samples from shards.
///
/// Workers read samples from the ShardReader, encode them to features,
/// and send batches through a channel for training.
pub struct DataLoader {
    receiver: mpsc::Receiver<BatchData>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl DataLoader {
    pub fn new(
        reader: Arc<ShardReader>,
        batch_size: usize,
        num_workers: usize,
        shutdown: Arc<AtomicBool>,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(num_workers * CHANNEL_BUFFER_MULTIPLIER);

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
        tx: mpsc::SyncSender<BatchData>,
        shutdown: Arc<AtomicBool>,
        batch_size: usize,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                let (features, scores) = Self::collect_batch(&reader, batch_size, &shutdown);

                if scores.is_empty() || tx.send((features, scores)).is_err() {
                    break;
                }
            }
        })
    }

    fn collect_batch(reader: &ShardReader, batch_size: usize, shutdown: &AtomicBool) -> BatchData {
        let mut features = Vec::with_capacity(batch_size * NUM_FEATURES);
        let mut scores = Vec::with_capacity(batch_size);

        for _ in 0..batch_size {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            match reader.next() {
                Some(sample) => {
                    if let Some((encoded, score)) = sample.encode() {
                        features.extend_from_slice(&encoded);
                        scores.push(score);
                    }
                }
                None => break,
            }
        }

        (features, scores)
    }
}

impl Iterator for DataLoader {
    type Item = BatchData;

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
