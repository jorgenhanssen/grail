use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use nnue::encoding::NUM_FEATURES;
use nnue::network::Network;
use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::args::Args;
use crate::dataset::{DataLoader, ShardReader, ShardedDataset};
use crate::training::evaluation::evaluate;
use crate::training::metrics::MetricsTracker;
use crate::training::progress::TrainingProgressBar;
use crate::utils::device::get_device;
use crate::utils::loss::huber;

/// Number of shards to keep loaded for training.
const TRAIN_SHARDS: usize = 10;

/// Number of shards to keep loaded for validation/test.
const EVAL_SHARDS: usize = 4;

pub struct Trainer {
    network: Network,
    optimizer: AdamW,
    varmap: VarMap,
    device: Device,
    batch_size: usize,
    workers: usize,
    epochs: usize,
    lr_decay: f64,
    patience: u64,
    model_path: String,
}

impl Trainer {
    pub fn new(args: &Args, model_path: &str) -> Result<Self, Box<dyn Error>> {
        let device = get_device()?;
        log::info!("Using device: {:?}", device);

        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let network = Network::new(&vs)?;
        let optimizer = AdamW::new(
            varmap.all_vars(),
            ParamsAdamW {
                lr: args.learning_rate,
                ..Default::default()
            },
        )?;

        Ok(Self {
            network,
            optimizer,
            varmap,
            device,
            batch_size: args.batch_size,
            workers: args.workers,
            epochs: args.epochs,
            lr_decay: args.lr_decay,
            patience: args.patience,
            model_path: model_path.to_string(),
        })
    }

    pub fn train(
        &mut self,
        dataset: &ShardedDataset,
        shutdown: Arc<AtomicBool>,
    ) -> Result<(), Box<dyn Error>> {
        let mut metrics = MetricsTracker::new(self.patience);

        for epoch in 1..=self.epochs {
            if shutdown.load(Ordering::Relaxed) {
                log::info!("Training interrupted at epoch {}", epoch);
                break;
            }

            let val_loss = self.train_epoch(dataset, &shutdown)?;

            let Some(val_loss) = val_loss else {
                log::info!("Epoch {} interrupted", epoch);
                break;
            };

            let did_improve = metrics.update(val_loss);
            if did_improve {
                let _ = self.save_model(Path::new(&self.model_path));
            }

            if metrics.should_stop() {
                log::info!("Early stopping after {} epochs", epoch);
                break;
            }

            self.decay_learning_rate();
        }

        if !shutdown.load(Ordering::Relaxed) {
            self.test_model(dataset, &shutdown)?;
        }

        Ok(())
    }

    fn train_epoch(
        &mut self,
        dataset: &ShardedDataset,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<Option<f32>, Box<dyn Error>> {
        let reader = Arc::new(ShardReader::new(dataset.train_path(), TRAIN_SHARDS)?);
        let loader = DataLoader::new(reader, self.batch_size, self.workers, Arc::clone(shutdown));

        let num_batches = dataset.stats.train_samples.div_ceil(self.batch_size);
        let progress = TrainingProgressBar::new(num_batches)?;

        let mut batches_processed = 0;
        let mut total_loss = 0.0;
        let mut train_loss = 0.0;

        for (features, scores) in loader {
            // Check for shutdown
            if shutdown.load(Ordering::Relaxed) {
                return Ok(None);
            }

            let batch_len = scores.len();
            if batch_len == 0 {
                continue;
            }

            let x = Tensor::from_vec(features, (batch_len, NUM_FEATURES), &self.device)?;
            let y = Tensor::from_vec(scores, (batch_len, 1), &self.device)?;

            let preds = self.network.forward(&x)?;
            let loss = huber(&preds, &y)?;

            self.optimizer.backward_step(&loss)?;

            let loss_val = loss.to_vec0::<f32>()?;
            total_loss += loss_val;
            batches_processed += 1;

            train_loss = total_loss / batches_processed as f32;
            progress.update(train_loss);
        }

        let val_reader = Arc::new(ShardReader::new(dataset.val_path(), EVAL_SHARDS)?);
        let val_loader = DataLoader::new(
            val_reader,
            self.batch_size,
            self.workers,
            Arc::clone(shutdown),
        );
        let val_loss = evaluate(&self.network, val_loader, &self.device)?;

        progress.finish(val_loss, train_loss);

        Ok(Some(val_loss))
    }

    fn decay_learning_rate(&mut self) {
        let current_lr = self.optimizer.learning_rate();
        let new_lr = current_lr * self.lr_decay;
        self.optimizer.set_learning_rate(new_lr);
    }

    fn test_model(
        &mut self,
        dataset: &ShardedDataset,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<f32, Box<dyn Error>> {
        log::info!("Running final test set evaluation...");
        let model_path = self.model_path.clone();
        self.load_model(Path::new(&model_path))?;

        let test_reader = Arc::new(ShardReader::new(dataset.test_path(), EVAL_SHARDS)?);
        let test_loader = DataLoader::new(
            test_reader,
            self.batch_size,
            self.workers,
            Arc::clone(shutdown),
        );
        let test_loss = evaluate(&self.network, test_loader, &self.device)?;
        log::info!("Test Loss: {:.6}", test_loss);

        Ok(test_loss)
    }

    fn save_model(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.varmap.save(path)?;
        Ok(())
    }

    fn load_model(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        if path.exists() {
            self.varmap.load(path)?;
        }
        Ok(())
    }
}
