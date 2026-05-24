use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use nnue::encoding::NUM_FEATURES;
use nnue::network::Network;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::args::Args;
use crate::dataset::{DataLoader, ShardReader, ShardedDataset};
use crate::state::{EpochRecord, TrainingState};
use crate::training::evaluation::evaluate;
use crate::training::progress::TrainingProgressBar;
use crate::utils::device::get_device;
use crate::utils::loss::wdl_eval_loss;

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
    wdl: f64,
    draw_target: f32,
}

impl Trainer {
    pub fn new(args: &Args, state: &TrainingState) -> Result<Self, Box<dyn Error>> {
        let device = get_device()?;
        let wdl = args.wdl.clamp(0.0, 1.0);
        let draw_target = args.draw_target.clamp(0.0, 1.0) as f32;

        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let network = Network::new(&vs)?;

        let lr = state
            .last_learning_rate()
            .map(|prev| prev * args.lr_decay)
            .unwrap_or(args.learning_rate);

        let optimizer = AdamW::new(
            varmap.all_vars(),
            ParamsAdamW {
                lr,
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
            wdl,
            draw_target,
        })
    }

    pub fn train(
        &mut self,
        dataset: &ShardedDataset,
        state: &mut TrainingState,
        model_path: &Path,
        state_path: &Path,
        shutdown: Arc<AtomicBool>,
    ) -> Result<(), Box<dyn Error>> {
        log::info!("Using device: {:?}", self.device);
        log::info!(
            "WDL blending: {:.0}% WDL / {:.0}% eval, draw target {:.2}",
            self.wdl * 100.0,
            (1.0 - self.wdl) * 100.0,
            self.draw_target,
        );
        log::info!(
            "Starting at epoch {} with LR {:.2e}",
            state.next_epoch_number(),
            self.optimizer.learning_rate(),
        );

        for epoch in state.next_epoch_number()..=self.epochs {
            if shutdown.load(Ordering::Relaxed) {
                log::info!("Training interrupted at epoch {}", epoch);
                break;
            }

            let prev_best_val = state.best_achieved_val_loss();

            let Some((train_loss, val_loss)) = self.train_epoch(dataset, &shutdown)? else {
                log::info!("Epoch {} interrupted", epoch);
                break;
            };

            if val_loss < prev_best_val {
                if let Err(e) = self.save_model(model_path) {
                    log::warn!("Failed to save model: {}", e);
                }
            }

            state.record_epoch(EpochRecord {
                epoch,
                train_loss,
                val_loss,
                learning_rate: self.optimizer.learning_rate(),
            });
            if let Err(e) = state.save(state_path) {
                log::warn!("Failed to save training state: {}", e);
            }

            if state.epochs_no_improve() >= self.patience {
                log::info!("Early stopping after {} epochs", epoch);
                break;
            }

            self.decay_learning_rate();
        }

        if !shutdown.load(Ordering::Relaxed) {
            self.test_model(dataset, model_path, &shutdown)?;
        }

        Ok(())
    }

    fn train_epoch(
        &mut self,
        dataset: &ShardedDataset,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<Option<(f32, f32)>, Box<dyn Error>> {
        let reader = Arc::new(ShardReader::new(dataset.train_path(), TRAIN_SHARDS)?);
        let loader = DataLoader::new(
            reader,
            self.batch_size,
            self.workers,
            Arc::clone(shutdown),
            self.draw_target,
        );

        let num_batches = dataset.stats.train_samples.div_ceil(self.batch_size);
        let progress = TrainingProgressBar::new(num_batches)?;

        let mut batches_processed = 0;
        let mut total_loss = 0.0;
        let mut train_loss = 0.0;

        for batch in loader {
            if shutdown.load(Ordering::Relaxed) {
                return Ok(None);
            }

            let batch_len = batch.scores.len();
            if batch_len == 0 {
                continue;
            }

            let stm =
                Tensor::from_vec(batch.stm_features, (batch_len, NUM_FEATURES), &self.device)?;
            let nstm =
                Tensor::from_vec(batch.nstm_features, (batch_len, NUM_FEATURES), &self.device)?;
            let y_eval = Tensor::from_vec(batch.scores, (batch_len, 1), &self.device)?;
            let y_outcome = Tensor::from_vec(batch.outcomes, (batch_len, 1), &self.device)?;

            let preds = self.network.forward(&stm, &nstm, &batch.buckets)?;
            let loss = wdl_eval_loss(&preds, &y_eval, &y_outcome, self.wdl)?;

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
            self.draw_target,
        );
        let val_loss = evaluate(&self.network, val_loader, &self.device, self.wdl)?;

        progress.finish(val_loss, train_loss);

        Ok(Some((train_loss, val_loss)))
    }

    fn decay_learning_rate(&mut self) {
        let new_lr = self.optimizer.learning_rate() * self.lr_decay;
        self.optimizer.set_learning_rate(new_lr);
    }

    fn test_model(
        &mut self,
        dataset: &ShardedDataset,
        model_path: &Path,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<f32, Box<dyn Error>> {
        log::info!("Running final test set evaluation...");
        self.load_model(model_path)?;

        let test_reader = Arc::new(ShardReader::new(dataset.test_path(), EVAL_SHARDS)?);
        let test_loader = DataLoader::new(
            test_reader,
            self.batch_size,
            self.workers,
            Arc::clone(shutdown),
            self.draw_target,
        );
        let test_loss = evaluate(&self.network, test_loader, &self.device, self.wdl)?;
        log::info!("Test Loss: {:.6}", test_loss);

        Ok(test_loss)
    }

    pub fn save_model(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.varmap.save(path)?;
        Ok(())
    }

    pub fn load_model(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.varmap.load(path)?;
        Ok(())
    }
}
