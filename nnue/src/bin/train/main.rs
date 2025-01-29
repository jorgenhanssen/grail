mod args;
mod utils;

use args::Args;
use candle_core::{DType, Device, Result as CandleResult, Tensor};
use candle_nn::{loss::mse, AdamW, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use nnue::{network::Network, samples::Samples, version::VersionManager};
use simplelog::{Config, SimpleLogger};
use std::{error::Error, fs::File};
use utils::train_test_split;

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let manager = VersionManager::new()?;
    let version = manager.get_latest_version()?.expect("No version found");
    let samples = load_samples(&manager)?;
    let device = Device::Cpu;

    log::info!("Converting samples to tensors...");

    let (x, y) = samples.to_xy(&device)?;

    log::info!("Splitting samples into train and test...");

    let (x_train, x_test, y_train, y_test) = train_test_split(&x, &y, 0.1, Some(42))?;

    log::info!("Creating network...");

    let varmap = VarMap::new();
    let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let net = Network::new(&vs)?;

    let mut opt = AdamW::new(varmap.all_vars(), ParamsAdamW::default())?;

    log::info!("Training network...");

    fit(&net, &mut opt, &x_train, &y_train, &args, 0.2)?;

    log::info!("Testing network...");

    let test_preds = net.forward(&x_test)?;
    let test_loss = mse(&test_preds, &y_test)?;

    log::info!("Dumping test results...");

    dump_test_results(&test_preds, &y_test, f32::try_from(test_loss)?)?;

    log::info!("Saving model...");
    let path = manager.file_path(version, "model.bin");
    varmap.save(&path)?;

    log::info!("Done!");

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();
    SimpleLogger::init(LevelFilter::Info, Config::default())?;

    Ok(args)
}

fn fit(
    net: &Network,
    opt: &mut AdamW,
    x: &Tensor,
    y: &Tensor,
    args: &Args,
    validation_split: f32,
) -> CandleResult<()> {
    let batch_size = args.batch_size;
    let epochs = args.epochs;

    let (x_train, x_val, y_train, y_val) =
        train_test_split(x, y, validation_split as f64, Some(42))?;
    let num_batches = x_train.dim(0)? / batch_size;

    for epoch in 1..=epochs {
        println!("Epoch {}/{}", epoch, epochs);
        let mut epoch_loss = 0f32;

        let progress_bar = ProgressBar::new(num_batches as u64);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    " {spinner:.cyan} {pos}/{len} [{wide_bar:.cyan/blue}] {eta_precise} | {msg}",
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
                "val: {:.6}, loss: {:.6}",
                val_loss, final_train_loss
            ));
        }

        progress_bar.finish();
    }

    Ok(())
}

fn load_samples(manager: &VersionManager) -> Result<Samples, Box<dyn Error>> {
    let version = manager.get_latest_version()?.expect("No version found");
    log::info!("Loading data for version {}", version);

    let path = manager.file_path(version, "data.bin");
    let mut file = File::open(&path)?;
    let samples = Samples::read_from_reader(&mut file)?;

    log::info!("Read {} samples from {:?}", samples.len(), path);
    Ok(samples)
}

fn dump_test_results(
    predictions: &Tensor,
    labels: &Tensor,
    test_loss: f32,
) -> Result<(), Box<dyn Error>> {
    use std::io::Write;

    log::info!("Test loss: {}", test_loss);

    let mut file = File::create("test_results.txt")?;
    writeln!(file, "Test Loss: {}", test_loss)?;
    writeln!(file, "Label      Prediction")?;
    writeln!(file, "--------------------")?;

    let num_samples = predictions.dim(0)?;
    for i in 0..num_samples {
        let pred = f32::try_from(predictions.get(i)?.squeeze(0)?)?;
        let label = f32::try_from(labels.get(i)?.squeeze(0)?)?;
        writeln!(file, "{:<10.6} {:.6}", label, pred)?;
    }

    log::info!("Test results have been written to test_results.txt");
    Ok(())
}
