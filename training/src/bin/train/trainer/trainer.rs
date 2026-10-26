use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use nnue::network::Network;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use training::dataset::DatasetCollection;

use crate::args::Args;
use crate::data_loader::DataLoader;
use crate::feature_encoder::FeatureEncoder;
use crate::state::{EpochRecord, TrainingState};
use crate::trainer::evaluation::evaluate;
use crate::trainer::progress::TrainingProgressBar;
use crate::utils::device::get_device;
use crate::utils::loss::wdl_eval_loss;

pub struct Trainer {
    network: Network,
    optimizer: AdamW,
    varmap: VarMap,
    device: Device,
    batch_size: usize,
    num_batches: usize,
    buffer_size: usize,
    epochs: usize,
    lr_decay: f64,
    patience: u64,
    wdl: f64,
    draw_target: f32,
}

impl Trainer {
    pub fn new(
        args: &Args,
        state: &TrainingState,
        train_samples: usize,
    ) -> Result<Self, Box<dyn Error>> {
        let device = get_device();
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
            num_batches: train_samples.div_ceil(args.batch_size),
            buffer_size: args.buffer_size,
            epochs: args.epochs,
            lr_decay: args.lr_decay,
            patience: args.patience,
            wdl,
            draw_target,
        })
    }

    pub fn train(
        &mut self,
        datasets: &DatasetCollection,
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

            let seed = state.seed.wrapping_add(epoch as u64);
            let Some((train_loss, val_loss)) = self.train_epoch(datasets, seed, &shutdown)? else {
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
            self.test_model(datasets, model_path, &shutdown)?;
        }

        Ok(())
    }

    fn train_epoch(
        &mut self,
        datasets: &DatasetCollection,
        seed: u64,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<Option<(f32, f32)>, Box<dyn Error>> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut games = datasets.train_games()?;
        games.shuffle_files(&mut rng);

        let loader = DataLoader::new(games, self.buffer_size, Some(rng), Arc::clone(shutdown));
        let encoder = FeatureEncoder::new(
            loader,
            self.batch_size,
            Arc::clone(shutdown),
            self.draw_target,
        );

        let progress = TrainingProgressBar::new(self.num_batches)?;

        let mut batches_processed = 0;
        let mut total_loss = 0.0;
        let mut train_loss = 0.0;

        for batch in encoder {
            let batch = batch?;
            if shutdown.load(Ordering::Relaxed) {
                return Ok(None);
            }

            let batch_len = batch.len();
            if batch_len == 0 {
                continue;
            }

            let max_active_features = batch.max_active_features();
            let stm_indices = Tensor::from_vec(
                batch.stm_features,
                (batch_len, max_active_features),
                &self.device,
            )?;
            let nstm_indices = Tensor::from_vec(
                batch.nstm_features,
                (batch_len, max_active_features),
                &self.device,
            )?;
            let y_eval = Tensor::from_vec(batch.scores, (batch_len, 1), &self.device)?;
            let y_outcome = Tensor::from_vec(batch.outcomes, (batch_len, 1), &self.device)?;

            let preds = self
                .network
                .forward(&stm_indices, &nstm_indices, &batch.buckets)?;
            let loss = wdl_eval_loss(&preds, &y_eval, &y_outcome, self.wdl)?;

            self.optimizer.backward_step(&loss)?;

            let loss_val = loss.to_vec0::<f32>()?;
            total_loss += loss_val;
            batches_processed += 1;

            train_loss = total_loss / batches_processed as f32;
            progress.update(train_loss);
        }

        if shutdown.load(Ordering::Relaxed) {
            return Ok(None);
        }

        let val_loader = DataLoader::new(
            datasets.val_games()?,
            self.buffer_size,
            None,
            Arc::clone(shutdown),
        );
        let val_encoder = FeatureEncoder::new(
            val_loader,
            self.batch_size,
            Arc::clone(shutdown),
            self.draw_target,
        );
        let val_loss = evaluate(&self.network, val_encoder, &self.device, self.wdl)?;

        if shutdown.load(Ordering::Relaxed) {
            return Ok(None);
        }

        progress.finish(val_loss, train_loss);

        Ok(Some((train_loss, val_loss)))
    }

    fn decay_learning_rate(&mut self) {
        let new_lr = self.optimizer.learning_rate() * self.lr_decay;
        self.optimizer.set_learning_rate(new_lr);
    }

    fn test_model(
        &mut self,
        datasets: &DatasetCollection,
        model_path: &Path,
        shutdown: &Arc<AtomicBool>,
    ) -> Result<f32, Box<dyn Error>> {
        log::info!("Running final test set evaluation...");
        self.load_model(model_path)?;

        let test_loader = DataLoader::new(
            datasets.test_games()?,
            self.buffer_size,
            None,
            Arc::clone(shutdown),
        );
        let test_encoder = FeatureEncoder::new(
            test_loader,
            self.batch_size,
            Arc::clone(shutdown),
            self.draw_target,
        );
        let test_loss = evaluate(&self.network, test_encoder, &self.device, self.wdl)?;
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
