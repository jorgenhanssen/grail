mod args;
mod data;
mod loss;
mod training;

use args::Args;
use candle_core::Device;
use candle_nn::Module;
use candle_nn::{AdamW, VarMap};
use clap::Parser;
use log::LevelFilter;
use loss::huber;
use nnue::{network::Network, samples::Samples};
use simplelog::{Config, SimpleLogger};
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let args = init()?;

    let device = training::get_device()?;

    let samples = data::load_samples()?;
    let (train_idx, test_idx) = samples.train_test_indices(args.test_split);

    let (net, varmap) = training::create_network(&device)?;
    let mut opt = training::create_optimizer(&varmap, args.learning_rate)?;

    train_model(&net, &samples, &train_idx, &mut opt, &device, &args)?;
    test_model(&net, &samples, &test_idx, &device, args.batch_size)?;
    save_model(&varmap)?;

    Ok(())
}

fn init() -> Result<Args, Box<dyn Error>> {
    let args = Args::parse();
    SimpleLogger::init(LevelFilter::Info, Config::default())?;
    Ok(args)
}

fn train_model(
    net: &Network,
    samples: &Samples,
    train_idx: &[usize],
    opt: &mut AdamW,
    device: &Device,
    args: &Args,
) -> Result<(), Box<dyn Error>> {
    log::info!("Starting training...");
    let trainer = training::Trainer::new(args.batch_size, args.epochs, args.lr_decay);
    trainer.fit(
        net,
        samples,
        train_idx,
        opt,
        device,
        args.validation_split,
        args.early_stop_patience,
    )?;
    Ok(())
}

fn test_model(
    net: &Network,
    samples: &Samples,
    test_idx: &[usize],
    device: &Device,
    batch_size: usize,
) -> Result<(), Box<dyn Error>> {
    log::info!("Starting testing...");

    let batched_iter = samples.to_xy_batched_indices(test_idx, batch_size, device);
    let mut total_loss = 0f32;
    let mut batch_count = 0usize;

    for batch_res in batched_iter {
        let (x_batch, y_batch) = batch_res?;
        let preds = net.forward(&x_batch)?;
        let batch_loss = huber(&preds, &y_batch)?;
        total_loss += f32::try_from(batch_loss)?;
        batch_count += 1;
    }

    let avg_loss = total_loss / batch_count.max(1) as f32;
    log::info!("Test loss: {:.5}", avg_loss);

    Ok(())
}

fn save_model(varmap: &VarMap) -> Result<(), Box<dyn Error>> {
    log::info!("Saving model");
    let model_path = PathBuf::from("nnue/model.safetensors");
    varmap.save(&model_path)?;
    log::info!("Model saved to {}", model_path.display());
    Ok(())
}
