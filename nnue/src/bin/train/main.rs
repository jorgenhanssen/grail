mod args;

use args::Args;
use candle_core::{DType, Device, Result as CandleResult, Tensor};
use candle_nn::{loss::mse, AdamW, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use nnue::{network::Network, samples::Samples, version::VersionManager};
use rand::{seq::SliceRandom, thread_rng};
use simplelog::{Config, SimpleLogger};
use std::{error::Error, fs::File, io::BufReader};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;
    let manager = VersionManager::new()?;
    let version = manager.get_latest_version()?.expect("No version found");
    let samples = load_samples(&manager)?;

    log::info!("Splitting samples into train and test");
    let (train_samples, test_samples) = samples.train_test_split(0.01, Some(42));

    log::info!("Creating network");
    let device = Device::Cpu;
    let (net, varmap) = create_network(&device)?;
    let mut opt = AdamW::new(varmap.all_vars(), ParamsAdamW::default())?;

    log::info!("Training network");
    let trainer = Trainer::new(args.batch_size, args.epochs);
    trainer.fit(&net, &train_samples, &mut opt, &device, 0.1, 2)?;

    evaluate(&net, &test_samples, &device, &manager)?;

    log::info!("Saving model");
    let path = manager.file_path(version, "model.safetensors");
    varmap.save(&path)?;

    log::info!("Done!");
    Ok(())
}

struct Trainer {
    batch_size: usize,
    epochs: usize,
}

impl Trainer {
    fn new(batch_size: usize, epochs: usize) -> Self {
        Self { batch_size, epochs }
    }

    pub fn fit(
        &self,
        net: &Network,
        train_samples: &Samples,
        opt: &mut AdamW,
        device: &Device,
        validation_split: f32,
        early_stop_patience: u64,
    ) -> CandleResult<()> {
        // Split train_samples -> (train_only, val_only)
        let (train_only, val_only) =
            train_samples.train_test_split(validation_split as f64, Some(42));

        let mut best_val_loss = f32::MAX;
        let mut epochs_no_improve = 0;

        for epoch in 1..=self.epochs {
            // Create a progress bar
            let total_batches = (train_only.len() + self.batch_size - 1) / self.batch_size;
            let progress_bar = ProgressBar::new(total_batches as u64);
            progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    " {spinner:.cyan} {pos}/{len} [{wide_bar:.cyan/blue}] {eta_precise} | {msg}",
                )
                .unwrap(),
        );

            // Optionally shuffle your training samples each epoch
            let mut epoch_train = train_only.clone();
            epoch_train.samples.shuffle(&mut thread_rng());

            let train_loss = self.train_epoch(net, opt, &epoch_train, device, &progress_bar)?;

            let val_loss = if val_only.len() > 0 {
                self.validate(net, &val_only, device)?
            } else {
                0.0
            };

            progress_bar.set_message(format!("val: {:.6}, loss: {:.6}", val_loss, train_loss));

            progress_bar.finish();

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
        train_samples: &Samples,
        device: &Device,
        progress_bar: &ProgressBar,
    ) -> CandleResult<f32> {
        let mut epoch_loss_sum = 0f32;
        let mut batch_count = 0usize;

        // Create the batched iterator
        let mut batched_iter = train_samples.to_xy_batched(self.batch_size, device);

        // Loop over each batch
        while let Some(batch_res) = batched_iter.next() {
            let (x_batch, y_batch) = batch_res?;
            let loss = self.train_step(net, opt, &x_batch, &y_batch)?;
            epoch_loss_sum += loss;
            batch_count += 1;

            // Update the progress bar message
            let current_loss = epoch_loss_sum / batch_count as f32;
            progress_bar.set_message(format!("loss: {:.6}", current_loss));
            progress_bar.inc(1);
        }

        // Average loss for the epoch
        Ok(epoch_loss_sum / (batch_count.max(1) as f32))
    }

    #[inline]
    fn train_step(
        &self,
        net: &Network,
        opt: &mut AdamW,
        x_batch: &Tensor,
        y_batch: &Tensor,
    ) -> CandleResult<f32> {
        let preds = net.forward(x_batch)?;
        let loss = mse(&preds, y_batch)?;
        opt.backward_step(&loss)?;
        Ok(f32::try_from(loss)?)
    }

    fn validate(&self, net: &Network, val_samples: &Samples, device: &Device) -> CandleResult<f32> {
        let mut total_loss = 0f32;
        let mut batch_count = 0usize;

        let mut batched_iter = val_samples.to_xy_batched(self.batch_size, device);
        while let Some(batch_res) = batched_iter.next() {
            let (x_val, y_val) = batch_res?;
            let preds = net.forward(&x_val)?;
            let loss = candle_nn::loss::mse(&preds, &y_val)?;
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

fn load_samples(manager: &VersionManager) -> Result<Samples, Box<dyn Error>> {
    let version = manager.get_latest_version()?.expect("No version found");
    log::info!("Loading data for version {}", version);

    let path = manager.file_path(version, "data.csv");
    let file = File::open(&path)?;
    let samples = Samples::read(BufReader::new(file))?;

    log::info!("Read {} samples from {:?}", samples.len(), path);
    Ok(samples)
}

fn evaluate(
    net: &Network,
    samples: &Samples,
    device: &Device,
    manager: &VersionManager,
) -> Result<(), Box<dyn Error>> {
    log::info!("Evaluating model (batched).");

    let mut batched_iter = samples.to_xy_batched(64 /* or whatever */, device);
    let mut total_loss = 0f32;
    let mut total_count = 0usize;

    // Optionally, if you want to save predictions and labels:
    let mut all_labels = Vec::new();
    let mut all_preds = Vec::new();

    while let Some(batch_res) = batched_iter.next() {
        let (x_batch, y_batch) = batch_res?;
        let preds = net.forward(&x_batch)?;
        let batch_loss = candle_nn::loss::mse(&preds, &y_batch)?;
        total_loss += f32::try_from(batch_loss)?;
        total_count += 1;

        // Optional: store predictions for analysis
        let batch_size = x_batch.dim(0)?;
        for i in 0..batch_size {
            let label = f32::try_from(y_batch.get(i)?.squeeze(0)?)?;
            let pred = f32::try_from(preds.get(i)?.squeeze(0)?)?;
            all_labels.push(label);
            all_preds.push(pred);
        }
    }

    let avg_loss = total_loss / total_count.max(1) as f32;
    log::info!("Test loss: {}", avg_loss);

    // Save the results if desired:
    let version = manager.get_latest_version()?.expect("No version found");
    let file_path = manager.file_path(version, "evaluation.txt");
    let mut file = std::fs::File::create(&file_path)?;
    use std::io::Write;
    writeln!(file, "Test Loss: {}", avg_loss)?;
    writeln!(file, "Label      Prediction")?;
    writeln!(file, "--------------------")?;
    for (label, pred) in all_labels.iter().zip(all_preds.iter()) {
        writeln!(file, "{:<10.6} {:.6}", label, pred)?;
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
