use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
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
    pub val_ratio: f64,
    pub test_ratio: f64,
    pub history: Vec<EpochRecord>,
}

impl TrainingState {
    /// Load state from disk if present, otherwise build a fresh one with a random/provided seed.
    pub fn new(
        path: &Path,
        val_ratio: f64,
        test_ratio: f64,
        seed: Option<u64>,
    ) -> Result<Self, Box<dyn Error>> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self {
                seed: seed.unwrap_or_else(|| rand::rng().random()),
                val_ratio,
                test_ratio,
                history: Vec::new(),
            })
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

    /// Removes the state file if it exists. Returns true if something was deleted.
    pub fn destroy(path: &Path) -> io::Result<bool> {
        if path.exists() {
            fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 12345;
    const VAL_RATIO: f64 = 0.05;
    const TEST_RATIO: f64 = 0.01;
    const LR: f64 = 0.001;

    fn record(epoch: usize, val_loss: f32) -> EpochRecord {
        EpochRecord {
            epoch,
            train_loss: val_loss,
            val_loss,
            learning_rate: LR,
        }
    }

    fn fresh() -> TrainingState {
        TrainingState {
            seed: SEED,
            val_ratio: VAL_RATIO,
            test_ratio: TEST_RATIO,
            history: Vec::new(),
        }
    }

    #[test]
    fn roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("training.json");

        let mut original = fresh();
        original.record_epoch(record(1, 0.5));
        original.record_epoch(record(2, 0.4));
        original.save(&path).unwrap();

        let loaded = TrainingState::load(&path).unwrap();
        assert_eq!(loaded.seed, SEED);
        assert_eq!(loaded.val_ratio, VAL_RATIO);
        assert_eq!(loaded.test_ratio, TEST_RATIO);
        assert_eq!(loaded.next_epoch_number(), 3);
        assert_eq!(loaded.best_achieved_val_loss(), 0.4);
    }

    #[test]
    fn load_missing_file_errors() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(TrainingState::load(&tmp.path().join("missing.json")).is_err());
    }

    #[test]
    fn new_returns_fresh_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("missing.json");
        let s = TrainingState::new(&path, VAL_RATIO, TEST_RATIO, None).unwrap();
        assert_eq!(s.val_ratio, VAL_RATIO);
        assert_eq!(s.test_ratio, TEST_RATIO);
        assert!(!s.has_history());
    }

    #[test]
    fn new_loads_existing_and_ignores_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("training.json");
        let mut original = fresh();
        original.record_epoch(record(1, 0.5));
        original.save(&path).unwrap();

        let s = TrainingState::new(&path, 0.0, 0.0, Some(99999)).unwrap();
        assert_eq!(s.seed, SEED);
        assert!(s.has_history());
    }

    #[test]
    fn derives_empty() {
        let s = fresh();
        assert!(!s.has_history());
        assert_eq!(s.next_epoch_number(), 1);
        assert_eq!(s.best_achieved_val_loss(), f32::INFINITY);
        assert_eq!(s.epochs_no_improve(), 0);
        assert_eq!(s.last_learning_rate(), None);
    }

    #[test]
    fn derives_with_history() {
        let mut s = fresh();
        s.record_epoch(record(1, 0.5));
        s.record_epoch(record(2, 0.4));
        s.record_epoch(record(3, 0.5));
        s.record_epoch(record(4, 0.3));
        s.record_epoch(record(5, 0.4));
        s.record_epoch(record(6, 0.5));

        assert!(s.has_history());
        assert_eq!(s.next_epoch_number(), 7);
        assert_eq!(s.best_achieved_val_loss(), 0.3);
        assert_eq!(s.epochs_no_improve(), 2);
        assert_eq!(s.last_learning_rate(), Some(LR));
    }
}
