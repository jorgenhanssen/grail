use candle_core::safetensors::SliceSafetensors;
use candle_core::Device;
use candle_nn::VarMap;
use evaluation::NNUE;

pub fn resolve_nnue() -> Result<Box<dyn NNUE>, Box<dyn std::error::Error>> {
    static NNUE_BYTES: &[u8] = include_bytes!("../../nnue/model.safetensors");

    let varmap = VarMap::new();
    let mut nnue = nnue::Evaluator::new(&varmap, &Device::Cpu);

    let st = SliceSafetensors::new(NNUE_BYTES)?;
    let mut tensor_data = varmap.data().lock().unwrap();

    for (name, var) in tensor_data.iter_mut() {
        let tensor = st.load(name, var.device())?;
        var.set(&tensor)?;
    }

    nnue.enable_nnue();

    Ok(Box::new(nnue))
}
