use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use nnue::encoding::NUM_FEATURES;
use nnue::network::Network;
use std::error::Error;
use std::path::Path;

use crate::args::Args;
use crate::dataset::Dataset;
use crate::training::evaluation::evaluate;
use crate::training::metrics::MetricsTracker;
use crate::training::progress::TrainingProgressBar;
use crate::utils::device::get_device;
use crate::utils::loss::huber;

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

    pub fn train(&mut self, dataset: &mut Dataset) -> Result<(), Box<dyn Error>> {
        let mut metrics = MetricsTracker::new(self.patience);

        for epoch in 1..=self.epochs {
            let val_loss = self.train_epoch(dataset)?;

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

        self.test_model(dataset)?;

        Ok(())
    }

    fn train_epoch(&mut self, dataset: &mut Dataset) -> Result<f32, Box<dyn Error>> {
        let loader = dataset.train_loader(self.batch_size, self.workers);
        let num_batches = loader.num_samples().div_ceil(self.batch_size);

        let progress = TrainingProgressBar::new(num_batches)?;

        let mut batches_processed = 0;
        let mut total_loss = 0.0;
        let mut train_loss = 0.0;

        for (features, scores) in loader {
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

        // Evaluate on validation set
        let val_loader = dataset.val_loader(self.batch_size, self.workers);
        let val_loss = evaluate(&self.network, val_loader, &self.device)?;

        progress.finish(val_loss, train_loss);

        Ok(val_loss)
    }

    fn decay_learning_rate(&mut self) {
        let current_lr = self.optimizer.learning_rate();
        let new_lr = current_lr * self.lr_decay;
        self.optimizer.set_learning_rate(new_lr);
    }

    fn test_model(&mut self, dataset: &mut Dataset) -> Result<f32, Box<dyn Error>> {
        log::info!("Running final test set evaluation...");
        let model_path = self.model_path.clone();
        self.load_model(Path::new(&model_path))?;

        let test_loader = dataset.test_loader(self.batch_size, self.workers);
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
