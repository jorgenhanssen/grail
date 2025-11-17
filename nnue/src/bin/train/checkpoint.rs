use candle_nn::VarMap;
use std::{error::Error, path::PathBuf};

pub fn path(epoch: usize) -> PathBuf {
    PathBuf::from(format!("nnue/model_epoch_{}.safetensors", epoch))
}

pub fn save(varmap: &VarMap, epoch: usize) -> Result<(), Box<dyn Error>> {
    let checkpoint_path = path(epoch);
    varmap.save(&checkpoint_path)?;
    Ok(())
}

pub fn delete(epoch: usize) -> Result<(), Box<dyn Error>> {
    let checkpoint_path = path(epoch);
    if checkpoint_path.exists() {
        std::fs::remove_file(&checkpoint_path)?;
    }
    Ok(())
}
