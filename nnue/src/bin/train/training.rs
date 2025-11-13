use std::error::Error;

use crate::loss::huber;
use candle_core::{DType, Device, Result as CandleResult};
use candle_nn::{AdamW, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use indicatif::{ProgressBar, ProgressStyle};
use nnue::{network::Network, samples::Samples};
use rand::{seq::SliceRandom, thread_rng};

pub struct Trainer {
    batch_size: usize,
    epochs: usize,
    lr_decay: f64,
}

impl Trainer {
    pub fn new(batch_size: usize, epochs: usize, lr_decay: f64) -> Self {
        Self {
            batch_size,
            epochs,
            lr_decay,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fit(
        &self,
        net: &Network,
        samples: &Samples,
        train_idx: &[usize],
        opt: &mut AdamW,
        device: &Device,
        validation_split: f32,
        early_stop_patience: u64,
    ) -> CandleResult<()> {
        let split = (validation_split as f64).clamp(0.0, 0.9);
        let val_len = (train_idx.len() as f64 * split) as usize;
        let (val_idx, train_idx_vec) = train_idx.split_at(val_len);
        let mut train_idx_vec = train_idx_vec.to_vec();

        let num_samples = train_idx_vec.len();
        let mut best_val_loss = f32::MAX;
        let mut epochs_no_improve = 0;

        for epoch in 1..=self.epochs {
            let total_batches = (num_samples + self.batch_size - 1) / self.batch_size;
            let progress_bar = ProgressBar::new(total_batches as u64);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template(
                        " {spinner:.cyan} {pos}/{len} [{wide_bar:.cyan/blue}] {eta_precise} | {msg}",
                    )
                    .unwrap(),
            );

            train_idx_vec.shuffle(&mut thread_rng());

            let train_loss =
                self.train_epoch(net, opt, samples, &train_idx_vec, device, &progress_bar)?;
            let val_loss = if !val_idx.is_empty() {
                self.validate(net, samples, val_idx, device)?
            } else {
                0.0
            };

            progress_bar.set_message(format!("val: {:.5}, loss: {:.5}", val_loss, train_loss));
            progress_bar.finish();

            // Learning rate decay
            if self.lr_decay < 1.0 {
                let new_lr = opt.learning_rate() * self.lr_decay;
                opt.set_learning_rate(new_lr);
            }

            // Early stopping
            if val_loss < best_val_loss {
                best_val_loss = val_loss;
                epochs_no_improve = 0;
            } else {
                epochs_no_improve += 1;
                if epochs_no_improve >= early_stop_patience {
                    log::info!("Early stopping after {} epochs", epoch);
                    break;
                }
            }
        }
        Ok(())
    }

    fn train_epoch(
        &self,
        net: &Network,
        opt: &mut AdamW,
        samples: &Samples,
        train_idx: &[usize],
        device: &Device,
        progress_bar: &ProgressBar,
    ) -> CandleResult<f32> {
        let mut epoch_loss_sum = 0f32;
        let mut batch_count = 0usize;

        let batched_iter = samples.to_xy_batched_indices(train_idx, self.batch_size, device);

        for batch_res in batched_iter {
            let (x_batch, y_batch) = batch_res?;
            let preds = net.forward(&x_batch)?;
            let loss = huber(&preds, &y_batch)?;
            opt.backward_step(&loss)?;

            epoch_loss_sum += f32::try_from(loss)?;
            batch_count += 1;

            let current_loss = epoch_loss_sum / batch_count as f32;
            progress_bar.set_message(format!("loss: {:.5}", current_loss));
            progress_bar.inc(1);
        }

        Ok(epoch_loss_sum / (batch_count.max(1) as f32))
    }

    fn validate(
        &self,
        net: &Network,
        samples: &Samples,
        val_idx: &[usize],
        device: &Device,
    ) -> CandleResult<f32> {
        let mut total_loss = 0f32;
        let mut batch_count = 0usize;

        let batched_iter = samples.to_xy_batched_indices(val_idx, self.batch_size, device);
        for batch_res in batched_iter {
            let (x_val, y_val) = batch_res?;
            let preds = net.forward(&x_val)?;
            let loss = huber(&preds, &y_val)?;
            total_loss += f32::try_from(loss)?;
            batch_count += 1;
        }

        Ok(total_loss / (batch_count.max(1) as f32))
    }
}

pub fn get_device() -> Result<Device, Box<dyn Error>> {
    #[cfg(feature = "cuda")]
    if let Ok(device) = Device::cuda_if_available(0) {
        if device.is_cuda() {
            log::info!("Using CUDA device");
            return Ok(device);
        }
    }

    #[cfg(feature = "metal")]
    if let Ok(device) = Device::new_metal(0) {
        if device.is_metal() {
            log::info!("Using Metal device");
            return Ok(device);
        }
    }

    log::info!("Using CPU device");
    Ok(Device::Cpu)
}

pub fn create_network(device: &Device) -> Result<(Network, VarMap), Box<dyn Error>> {
    let varmap = VarMap::new();
    let vs = VarBuilder::from_varmap(&varmap, DType::F32, device);
    let net = Network::new(&vs)?;
    Ok((net, varmap))
}

pub fn create_optimizer(varmap: &VarMap, learning_rate: f64) -> Result<AdamW, Box<dyn Error>> {
    let opt = AdamW::new(
        varmap.all_vars(),
        ParamsAdamW {
            lr: learning_rate,
            ..ParamsAdamW::default()
        },
    )?;
    Ok(opt)
}
