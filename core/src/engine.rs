use crate::args::{Args, Engines};
use candle_core::safetensors::SliceSafetensors;
use candle_core::Device;
use candle_core::Result;
use evaluation::TraditionalEvaluator;
use nnue::NNUE;
pub use search::Engine;
pub use search::NegamaxEngine;

use candle_nn::VarMap;

const NNUE_VERSION: u32 = 1;
static NNUE_BYTES: &[u8] = include_bytes!("../../nnue/versions/v1/model.safetensors");

fn load_varmap_from_bytes(varmap: &mut VarMap, data: &[u8]) -> Result<()> {
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
        Engines::Negamax {} => {
            let mut varmap = VarMap::new();
            let mut nnue = NNUE::new(&varmap, &Device::Cpu, NNUE_VERSION);

            load_varmap_from_bytes(&mut varmap, NNUE_BYTES).unwrap();

            nnue.enable_nnue();

            NegamaxEngine::new(Box::new(nnue))
        }
    }
}
