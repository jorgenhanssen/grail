//! Dumps a weight analysis of the trained NNUE to a text file.
//! Useful during training to eyeball how the layers are actually behaving.

mod math;
mod report;
mod stats;

use candle_core::{DType, Device};
use candle_nn::{VarBuilder, VarMap};
use nnue::network::Network;
use std::error::Error;
use std::fs;
use std::path::Path;

const MODEL_PATH: &str = "nnue/model.safetensors";
const ANALYSIS_PATH: &str = "nnue/model.analysis.txt";

fn main() -> Result<(), Box<dyn Error>> {
    let mut varmap = VarMap::new();
    let vs = VarBuilder::from_varmap(&varmap, DType::F32, &Device::Cpu);
    let network = Network::new(&vs)?;
    varmap.load(Path::new(MODEL_PATH))?;

    let analysis = report::create(&network)?;

    fs::write(ANALYSIS_PATH, &analysis)?;
    println!("Analysis saved to {ANALYSIS_PATH}");

    Ok(())
}
