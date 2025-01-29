use crate::args::{Args, Engines};
use candle_core::safetensors::SliceSafetensors;
use candle_core::Device;
use candle_core::Result;
use nnue::NNUE;
pub use search::Engine;
pub use search::MinimaxEngine;

use candle_nn::VarMap;

static NNUE_BYTES: &[u8] = include_bytes!("../../nnue/versions/v0/model.safetensors");

fn load_varmap_from_bytes(varmap: &mut VarMap, data: &[u8]) -> Result<()> {
    // Example using `SliceSafetensors`
    let st = SliceSafetensors::new(data)?;
    let mut tensor_data = varmap.data().lock().unwrap();

    for (name, var) in tensor_data.iter_mut() {
        let tensor = st.load(name, var.device())?;
        var.set(&tensor)?;
    }
    Ok(())
}
pub fn create(args: &Args) -> impl Engine {
    match args.engines {
        Engines::Minimax {} => {
            let mut varmap = VarMap::new();
            let nnue = NNUE::new(&varmap, &Device::Cpu);

            load_varmap_from_bytes(&mut varmap, NNUE_BYTES).unwrap();

            MinimaxEngine::new(Box::new(nnue))
        }
    }
}
