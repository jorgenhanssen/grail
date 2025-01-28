use candle_core::{Result, Tensor};
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

pub fn train_test_split(
    x: &Tensor,
    y: &Tensor,
    split: f64,
    random_seed: Option<u64>,
) -> Result<(Tensor, Tensor, Tensor, Tensor)> {
    let num_samples = x.dim(0)?;
    let num_test = (num_samples as f64 * split) as usize;
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
