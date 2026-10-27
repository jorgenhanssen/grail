use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, sync_channel};
use std::thread::{self, JoinHandle};

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use training::dataset::{GameReader, TrainingSample};

pub struct DataLoader {
    receiver: Option<Receiver<io::Result<Vec<TrainingSample>>>>,
    worker: Option<JoinHandle<()>>,
}

impl DataLoader {
    pub fn new(
        mut games: GameReader,
        buffer_size: usize,
        mut rng: Option<StdRng>,
        shutdown: Arc<AtomicBool>,
    ) -> Self {
        let (sender, receiver) = sync_channel(0);

        let worker = thread::spawn(move || {
            loop {
                let mut buffer = Vec::with_capacity(buffer_size);

                while buffer.len() < buffer_size {
                    if shutdown.load(Ordering::Relaxed) {
                        return;
                    }

                    match games.next() {
                        Some(Ok(game)) => buffer.extend(game.into_samples()),
                        Some(Err(error)) => {
                            let _ = sender.send(Err(error));
                            return;
                        }
                        None => break,
                    }
                }

                if buffer.is_empty() {
                    break;
                }
                if let Some(rng) = &mut rng {
                    buffer.shuffle(rng);
                }
                if sender.send(Ok(buffer)).is_err() {
                    break;
                }
            }
        });

        Self {
            receiver: Some(receiver),
            worker: Some(worker),
        }
    }
}

impl Iterator for DataLoader {
    type Item = io::Result<Vec<TrainingSample>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.as_ref()?.recv().ok()
    }
}

impl Drop for DataLoader {
    fn drop(&mut self) {
        drop(self.receiver.take());

        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
