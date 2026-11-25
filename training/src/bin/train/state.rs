use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct EpochRecord {
    pub epoch: usize,
    pub train_loss: f32,
    pub val_loss: f32,
    pub learning_rate: f64,
}

/// Saved after every epoch so a training run can pick up where it left off.
#[derive(Debug, Serialize, Deserialize)]
pub struct TrainingState {
    pub seed: u64,
    pub history: Vec<EpochRecord>,
}

impl TrainingState {
    pub fn new(seed: Option<u64>) -> Self {
        Self {
            seed: seed.unwrap_or_else(|| rand::rng().random()),
            history: Vec::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        Ok(serde_json::from_reader(BufReader::new(file))?)
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let file = File::create(path)?;
        serde_json::to_writer_pretty(BufWriter::new(file), self)?;
        Ok(())
    }

    pub fn record_epoch(&mut self, record: EpochRecord) {
        self.history.push(record);
    }

    pub fn has_history(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn next_epoch_number(&self) -> usize {
        self.history.last().map(|e| e.epoch + 1).unwrap_or(1)
    }

    pub fn last_learning_rate(&self) -> Option<f64> {
        self.history.last().map(|e| e.learning_rate)
    }

    pub fn best_achieved_val_loss(&self) -> f32 {
        self.history
            .iter()
            .map(|e| e.val_loss)
            .fold(f32::INFINITY, f32::min)
    }

    pub fn epochs_no_improve(&self) -> u64 {
        let mut min_so_far = f32::INFINITY;
        let mut count = 0u64;
        for record in &self.history {
            if record.val_loss < min_so_far {
                min_so_far = record.val_loss;
                count = 0;
            } else {
                count += 1;
            }
        }
        count
    }
}
