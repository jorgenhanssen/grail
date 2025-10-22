mod args;

use args::Args;
use candle_core::{DType, Device, Result as CandleResult, Tensor};
use candle_nn::{AdamW, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use nnue::{network::Network, samples::Samples, version::VersionManager};
use rand::seq::SliceRandom;
use rand::thread_rng;
use simplelog::{Config, SimpleLogger};
use std::{error::Error, fs::File, io::BufReader};

// Huber loss on normalized centipawn targets
fn eval_loss(pred: &Tensor, eval_target: &Tensor) -> CandleResult<Tensor> {
    let diff = (pred - eval_target)?;
    let abs_diff = diff.abs()?;

    // Huber delta = 1.0 in normalized space (equivalent to TRAINING_SCALE cp)
    let huber_delta = 1.0;
    let is_small = abs_diff.lt(huber_delta)?;

    let quadratic = (diff.sqr()? * 0.5)?;
    let linear = ((abs_diff - 0.5 * huber_delta)? * huber_delta)?;

    let loss = is_small.where_cond(&quadratic, &linear)?;
    loss.mean_all()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;
    let manager = VersionManager::new()?;
    let version = manager.get_latest_version()?.expect("No version found");

    let (samples, train_idx, test_idx) = {
        let (samples, train_idx, test_idx) = load_samples(&manager)?;
        log::info!("Splitting samples into train and test");
        (samples, train_idx, test_idx)
    };

    log::info!("Creating network");
    let device = Device::cuda_if_available(0)?;

    if device.is_cuda() {
        log::info!("Using CUDA");
    } else {
        log::info!("Using CPU");
    }

    let (net, varmap) = create_network(&device)?;

    let mut opt = AdamW::new(
        varmap.all_vars(),
        ParamsAdamW {
            lr: args.learning_rate,
            ..ParamsAdamW::default()
        },
    )?;

    log::info!("Training network");
    let trainer = Trainer::new(args.batch_size, args.epochs, args.lr_decay);
    trainer.fit(
        &net,
        &samples,
        &train_idx,
        &mut opt,
        &device,
        0.1,
        args.early_stop_patience,
    )?;

    evaluate(&net, &samples, &test_idx, &device, &manager)?;

    log::info!("Saving model");
    let path = manager.file_path(version, "model.safetensors");
    varmap.save(&path)?;

    log::info!("Done!");
    Ok(())
}

struct Trainer {
    batch_size: usize,
    epochs: usize,
    lr_decay: f64,
}

impl Trainer {
    fn new(batch_size: usize, epochs: usize, lr_decay: f64) -> Self {
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
            let (x_batch, y_batch, wdl_batch) = batch_res?;
            let loss = self.train_step(net, opt, &x_batch, &y_batch, &wdl_batch)?;
            epoch_loss_sum += loss;
            batch_count += 1;

            let current_loss = epoch_loss_sum / batch_count as f32;
            progress_bar.set_message(format!("loss: {:.5}", current_loss));
            progress_bar.inc(1);
        }

        Ok(epoch_loss_sum / (batch_count.max(1) as f32))
    }

    #[inline]
    fn train_step(
        &self,
        net: &Network,
        opt: &mut AdamW,
        x_batch: &Tensor,
        y_batch: &Tensor,
        _wdl_batch: &Tensor,
    ) -> CandleResult<f32> {
        let preds = net.forward(x_batch)?;
        let loss = eval_loss(&preds, y_batch)?;
        opt.backward_step(&loss)?;
        f32::try_from(loss)
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
            let (x_val, y_val, _wdl_val) = batch_res?;
            let preds = net.forward(&x_val)?;
            let loss = eval_loss(&preds, &y_val)?;
            total_loss += f32::try_from(loss)?;
            batch_count += 1;
        }

        Ok(total_loss / (batch_count.max(1) as f32))
    }
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();
    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}

fn load_samples(
    manager: &VersionManager,
) -> Result<(Samples, Vec<usize>, Vec<usize>), Box<dyn Error>> {
    let mut samples = Samples::new();

    let versions = manager.get_all_versions().expect("No versions found");
    for version in versions.iter().rev() {
        let path = manager.file_path(*version, "data.csv");
        let file = File::open(&path)?;
        let version_samples = Samples::read(BufReader::new(file))?;
        log::info!("Loaded {} samples from {:?}", version_samples.len(), path);

        samples.extend(version_samples);
    }

    log::info!("Loaded {} total samples", samples.len());
    let (train_idx, test_idx) = samples.train_test_indices(0.01, Some(42));
    Ok((samples, train_idx, test_idx))
}

fn evaluate(
    net: &Network,
    samples: &Samples,
    test_idx: &[usize],
    device: &Device,
    manager: &VersionManager,
) -> Result<(), Box<dyn Error>> {
    log::info!("Evaluating model (batched).");

    let batched_iter = samples.to_xy_batched_indices(test_idx, 64, device);
    let mut total_loss = 0f32;
    let mut total_count = 0usize;

    let mut all_labels = Vec::new();
    let mut all_preds = Vec::new();

    for batch_res in batched_iter {
        let (x_batch, y_batch, _wdl_batch) = batch_res?;
        let preds = net.forward(&x_batch)?;
        let batch_loss = eval_loss(&preds, &y_batch)?;
        total_loss += f32::try_from(batch_loss)?;
        total_count += 1;

        let batch_size = x_batch.dim(0)?;
        for i in 0..batch_size {
            let target = f32::try_from(y_batch.get(i)?.squeeze(0)?)?;
            let pred = f32::try_from(preds.get(i)?.squeeze(0)?)?;

            all_labels.push(target);
            all_preds.push(pred);
        }
    }

    let avg_loss = total_loss / total_count.max(1) as f32;
    log::info!("Test loss: {}", avg_loss);

    let version = manager.get_latest_version()?.expect("No version found");
    let file_path = manager.file_path(version, "evaluation.txt");
    let mut file = std::fs::File::create(&file_path)?;
    use std::io::Write;
    writeln!(file, "Test Loss: {}", avg_loss)?;
    writeln!(file, "Target (normalized)  Prediction  (eval/400)")?;
    writeln!(file, "--------------------------------------------")?;
    for (label, pred) in all_labels.iter().zip(all_preds.iter()) {
        writeln!(file, "{:<20.5} {:.5}", label, pred)?;
    }

    log::info!("Evaluation written to {}", file_path.display());
    Ok(())
}

fn create_network(device: &Device) -> CandleResult<(Network, VarMap)> {
    let varmap = VarMap::new();
    let vs = VarBuilder::from_varmap(&varmap, DType::F32, device);
    let net = Network::new(&vs)?;
    Ok((net, varmap))
}
