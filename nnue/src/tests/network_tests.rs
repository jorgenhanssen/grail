use candle_core::{Device, Result, Tensor};
use candle_nn::{Module, VarBuilder, VarMap};
use rand::Rng;

use crate::encoding::NUM_FEATURES;
use crate::network::{NNUENetwork, Network};

/// Creates a random input vector filled with 0s and 1s
fn create_random_input() -> [f32; NUM_FEATURES] {
    let mut rng = rand::thread_rng();
    let mut input = [0.0; NUM_FEATURES];
    for i in 0..NUM_FEATURES {
        input[i] = if rng.gen_bool(0.5) { 1.0 } else { 0.0 };
    }
    input
}

#[test]
fn test_nnue_implementation() -> Result<()> {
    // Create a test network using random weights
    let vm = VarMap::new();
    let vb = VarBuilder::from_varmap(&vm, candle_core::DType::F32, &Device::Cpu);
    let candle_network = Network::new(&vb)?;
    let mut nnue_network = NNUENetwork::from_network(&candle_network)?;

    // Create a random test input
    let input = create_random_input();

    // Convert input to Tensor for Candle
    let input_tensor = Tensor::from_slice(&input, (1, input.len()), &Device::Cpu)?;

    // Get Candle output
    let network_result = candle_network
        .forward(&input_tensor)
        .and_then(|t| t.get(0))
        .and_then(|t| t.get(0))
        .and_then(|t| t.to_scalar::<f32>())
        .expect("Failed to evaluate position");

    // Get NNUE output
    let nnue_result = nnue_network.forward(&input);

    // Check if the difference is within an acceptable epsilon
    assert_approx_eq(network_result, nnue_result, "Values");

    Ok(())
}

#[test]
fn benchmark_nnue_vs_candle() -> Result<()> {
    // Create a test network using random weights
    let vm = VarMap::new();
    let vb = VarBuilder::from_varmap(&vm, candle_core::DType::F32, &Device::Cpu);
    let candle_network = Network::new(&vb)?;
    let mut nnue_network = NNUENetwork::from_network(&candle_network)?;

    const ITERATIONS: usize = 100_000;
    let inputs = (0..ITERATIONS)
        .map(|_| create_random_input())
        .collect::<Vec<_>>();

    let mut candle_sum = 0.0;
    let mut nnue_sum = 0.0;

    // Benchmark Candle implementation
    let start_time = std::time::Instant::now();
    for input in &inputs {
        let input_tensor = Tensor::from_slice(input, (1, input.len()), &Device::Cpu)?;

        let result = candle_network
            .forward(&input_tensor)
            .and_then(|t| t.get(0))
            .and_then(|t| t.get(0))
            .and_then(|t| t.to_scalar::<f32>())
            .expect("Failed to evaluate position");

        candle_sum += result;
    }
    let candle_duration = start_time.elapsed();
    println!("Candle result: {}", candle_sum);

    // Benchmark NNUE implementation
    let start_time = std::time::Instant::now();
    for input in &inputs {
        let result = nnue_network.forward(&input.to_vec());
        nnue_sum += result;
    }
    let nnue_duration = start_time.elapsed();
    println!("NNUE result: {}", nnue_sum);

    println!("Benchmark results over {} iterations:", ITERATIONS);
    println!(
        "Candle: {:?} ({:.2} ns/iter)",
        candle_duration,
        candle_duration.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "NNUE: {:?} ({:.2} ns/iter)",
        nnue_duration,
        nnue_duration.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "Speedup: {:.2}x",
        candle_duration.as_nanos() as f64 / nnue_duration.as_nanos() as f64
    );

    assert_approx_eq(candle_sum, nnue_sum, "Sums");

    Ok(())
}

/// Asserts that two floating point values are within EPSILON of each other
fn assert_approx_eq(a: f32, b: f32, description: &str) {
    const EPSILON: f32 = 1e-2;

    assert!(
        (a - b).abs() < EPSILON,
        "{} differ by more than epsilon: |{} - {}| = {} > {}",
        description,
        a,
        b,
        (a - b).abs(),
        EPSILON
    );
}
