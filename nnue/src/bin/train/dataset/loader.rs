use ahash::AHashMap;
use std::collections::hash_map::Entry;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{mpsc, Arc};
use std::thread;

use chess::{Board, Color};
use nnue::encoding::{encode_board, NUM_FEATURES};
use utils::board_metrics::BoardMetrics;

use super::indexer::SampleIndex;

pub const FV_SCALE: f32 = 400.0;

// Holds x items in the channel per worker
const CHANNEL_BUFFER_MULTIPLIER: usize = 2;

type BatchData = (Vec<f32>, Vec<f32>); // (x, y) - (features, scores)

pub struct DataLoader {
    receiver: mpsc::Receiver<BatchData>,
    workers: Vec<thread::JoinHandle<()>>,
    num_samples: usize,
}

impl DataLoader {
    pub fn new(
        indices: &[SampleIndex],
        files: &[PathBuf],
        batch_size: usize,
        num_workers: usize,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(num_workers * CHANNEL_BUFFER_MULTIPLIER);

        let files = Arc::new(files.to_vec());
        let shared_indices = Arc::new(indices.to_vec());

        let (work_sender, work_receiver) =
            mpsc::sync_channel::<Vec<SampleIndex>>(num_workers * CHANNEL_BUFFER_MULTIPLIER);
        let work_receiver = Arc::new(std::sync::Mutex::new(work_receiver));

        let workers = Self::spawn_workers(num_workers, work_receiver, sender.clone(), files);

        // Distribute batches to workers
        thread::spawn(move || {
            for chunk in shared_indices.chunks(batch_size) {
                if work_sender.send(chunk.to_vec()).is_err() {
                    break;
                }
            }
        });

        Self {
            receiver,
            workers,
            num_samples: indices.len(),
        }
    }

    pub fn num_samples(&self) -> usize {
        self.num_samples
    }

    fn spawn_workers(
        num_workers: usize,
        work_receiver: Arc<std::sync::Mutex<mpsc::Receiver<Vec<SampleIndex>>>>,
        sender: mpsc::SyncSender<BatchData>,
        files: Arc<Vec<PathBuf>>,
    ) -> Vec<thread::JoinHandle<()>> {
        (0..num_workers)
            .map(|_| {
                let rx = Arc::clone(&work_receiver);
                let tx = sender.clone();
                let paths = Arc::clone(&files);

                thread::spawn(move || {
                    let mut file_cache: AHashMap<u8, File> = AHashMap::new();

                    loop {
                        let batch_indices: Vec<SampleIndex> = {
                            match rx.lock().unwrap().recv() {
                                Ok(b) => b,
                                Err(_) => break,
                            }
                        };

                        let batch_size = batch_indices.len();

                        let mut features = Vec::with_capacity(batch_size * NUM_FEATURES);
                        let mut scores = Vec::with_capacity(batch_size);

                        for sample in batch_indices {
                            if let Err(e) = Self::process_sample(
                                sample,
                                &mut file_cache,
                                &paths,
                                &mut features,
                                &mut scores,
                            ) {
                                log::debug!("Failed to process sample: {}", e);
                            }
                        }

                        if tx.send((features, scores)).is_err() {
                            break;
                        }
                    }
                })
            })
            .collect()
    }

    fn process_sample(
        sample: SampleIndex,
        file_cache: &mut AHashMap<u8, File>,
        paths: &[PathBuf],
        features: &mut Vec<f32>,
        scores: &mut Vec<f32>,
    ) -> Result<(), String> {
        // Get or open file
        let file = match file_cache.entry(sample.file_id) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let path = paths
                    .get(sample.file_id as usize)
                    .ok_or_else(|| format!("Invalid file_id: {}", sample.file_id))?;
                let f = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
                e.insert(f)
            }
        };

        file.seek(SeekFrom::Start(sample.byte_offset))
            .map_err(|e| format!("Failed to seek: {}", e))?;

        let mut fen_bytes = vec![0u8; sample.fen_len as usize];

        file.read_exact(&mut fen_bytes)
            .map_err(|e| format!("Failed to read FEN: {}", e))?;

        let fen =
            std::str::from_utf8(&fen_bytes).map_err(|e| format!("Invalid UTF-8 in FEN: {}", e))?;
        let board =
            Board::from_str(fen).map_err(|e| format!("Failed to parse FEN '{}': {}", fen, e))?;

        let metrics = BoardMetrics::new(&board);
        let encoded_features = encode_board(
            &board,
            metrics.attacks[Color::White.to_index()],
            metrics.attacks[Color::Black.to_index()],
            metrics.support[Color::White.to_index()],
            metrics.support[Color::Black.to_index()],
            metrics.threats[Color::White.to_index()],
            metrics.threats[Color::Black.to_index()],
        );

        features.extend_from_slice(&encoded_features);
        scores.push(sample.score as f32 / FV_SCALE);

        Ok(())
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
        // Join all workers on drop to ensure clean shutdown
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
