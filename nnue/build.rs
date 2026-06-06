// For custom cuda kernels.
// https://github.com/huggingface/candle/blob/main/candle-examples/build.rs

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    #[cfg(feature = "cuda")]
    {
        use std::env;
        use std::path::PathBuf;

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let target = out_dir.join("cuda_kernels.rs");

        let bindings = cudaforge::KernelBuilder::new()
            .source_glob("kernels/*.cu")
            .build_ptx()
            .expect("Failed to build ptx");

        bindings
            .write(target)
            .expect("Failed to write ptx bindings");
    }
}
