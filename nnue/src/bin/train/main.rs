mod args;

use args::Args;
use candle_core::{DType, Device, Result as CandleResult, Tensor};
use candle_nn::{loss::mse, AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, LevelFilter};
use nnue::{encoding::NUM_FEATURES, samples::Samples, version::VersionManager, NNUE};
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use simplelog::{Config, SimpleLogger};
use std::{error::Error, fs::File};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let manager = VersionManager::new("nnue/versions")?;
    let samples = load_samples(&manager)?;
    let device = Device::Cpu;

    let (x, y) = samples.to_xy(&device)?;

    let (x_train, x_test, y_train, y_test) = train_test_split(&x, &y, 0.1, Some(42))?;

    let varmap = VarMap::new();
    let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let net = NNUE::new(&vs, NUM_FEATURES)?;
    let mut opt = AdamW::new(varmap.all_vars(), ParamsAdamW::default())?;

    train(
        &net,
        &mut opt,
        &x_train,
        &y_train,
        args.batch_size,
        args.epochs,
        0.2,
    )?;

    let test_preds = net.forward(&x_test)?;
    let test_loss = mse(&test_preds, &y_test)?;

    info!("Test loss: {}", f32::try_from(test_loss)?);

    // loop over the first 100 test predictions and print them vs the labels from y test
    for i in 0..200 {
        let pred = f32::try_from(test_preds.get(i)?.squeeze(0)?)?;
        let label = f32::try_from(y_test.get(i)?.squeeze(0)?)?;
        info!("{:.6} {:.6}", label, pred);
    }

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}

fn train(
    net: &NNUE,
    opt: &mut AdamW,
    x: &Tensor,
    y: &Tensor,
    batch_size: usize,
    epochs: usize,
    validation_split: f32,
) -> CandleResult<()> {
    // Split data into train and validation sets
    let (x_train, x_val, y_train, y_val) =
        train_test_split(x, y, validation_split as f64, Some(42))?;
    let num_batches = x_train.dim(0)? / batch_size;

    for epoch in 1..=epochs {
        info!("Epoch {}/{}", epoch, epochs);
        let mut epoch_loss = 0f32;

        let progress_bar = ProgressBar::new(num_batches as u64);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} {pos}/{len} [{wide_bar:.cyan/blue}] {eta_precise} | {msg}",
                )
                .unwrap(),
        );

        // For each batch
        for batch_idx in 0..num_batches {
            let start = batch_idx * batch_size;
            let x_batch = x_train.narrow(0, start, batch_size)?;
            let y_batch = y_train.narrow(0, start, batch_size)?;

            let preds = net.forward(&x_batch)?;
            let loss = mse(&preds, &y_batch)?;
            opt.backward_step(&loss)?;
            epoch_loss += f32::try_from(loss)?;

            let current_loss = epoch_loss / (batch_idx + 1) as f32;
            progress_bar.set_message(format!("loss: {:.6}", current_loss));
            progress_bar.inc(1);
        }

        if x_val.dim(0)? > 0 {
            let val_preds = net.forward(&x_val)?;
            let val_loss = f32::try_from(mse(&val_preds, &y_val)?)?;
            let final_train_loss = epoch_loss / num_batches as f32;

            progress_bar.set_message(format!(
                "val_loss: {:.6}, loss: {:.6}",
                val_loss, final_train_loss
            ));
        }

        progress_bar.finish();
    }

    Ok(())
}

fn load_samples(manager: &VersionManager) -> Result<Samples, Box<dyn Error>> {
    let version = manager.get_latest_version()?.expect("No version found");
    info!("Loading data for version {}", version);

    let path = manager.file_path(version, "data.bin");
    let mut file = File::open(&path)?;
    let samples = Samples::read_from_reader(&mut file)?;

    info!("Read {} samples from {:?}", samples.len(), path);
    Ok(samples)
}

fn train_test_split(
    x: &Tensor,
    y: &Tensor,
    test_ratio: f64,
    random_seed: Option<u64>,
) -> CandleResult<(Tensor, Tensor, Tensor, Tensor)> {
    let num_samples = x.dim(0)?;
    let num_test = (num_samples as f64 * test_ratio) as usize;
    let num_train = num_samples - num_test;

    let mut indices = Vec::with_capacity(num_samples);
    indices.extend(0..num_samples as i64);

    if let Some(seed) = random_seed {
        let mut rng = StdRng::seed_from_u64(seed);
        indices.shuffle(&mut rng);
    }

    let train_idx = Tensor::from_slice(&indices[..num_train], (num_train,), x.device())?;
    let test_idx = Tensor::from_slice(&indices[num_train..], (num_test,), x.device())?;

    let x_train = x.index_select(&train_idx, 0)?;
    let x_test = x.index_select(&test_idx, 0)?;
    let y_train = y.index_select(&train_idx, 0)?;
    let y_test = y.index_select(&test_idx, 0)?;

    Ok((x_train, x_test, y_train, y_test))
}
