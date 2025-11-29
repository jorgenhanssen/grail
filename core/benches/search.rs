use std::sync::{atomic::AtomicBool, Arc};

use candle_core::safetensors::SliceSafetensors;
use candle_core::Device;
use candle_nn::VarMap;
use cozy_chess::Board;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use search::{Engine, EngineConfig};
use uci::commands::GoParams;

const DEPTH: u8 = 15;
const SAMPLE_SIZE: usize = 10;

/// Perft positions from https://github.com/AndyGrant/Ethereal/blob/master/src/perft/standard.epd
const POSITIONS: &[&str] = &[
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
];

fn create_engine() -> Engine {
    let config = EngineConfig::default();
    let stop = Arc::new(AtomicBool::new(false));

    let hce = Box::new(evaluation::hce::Evaluator::new(
        config.get_piece_values(),
        config.get_hce_config(),
    ));

    // Load embedded NNUE
    static NNUE_BYTES: &[u8] = include_bytes!("../../nnue/model.safetensors");
    let varmap = VarMap::new();
    let mut nnue = nnue::Evaluator::new(&varmap, &Device::Cpu);
    let st = SliceSafetensors::new(NNUE_BYTES).unwrap();
    {
        let mut tensor_data = varmap.data().lock().unwrap();
        for (name, var) in tensor_data.iter_mut() {
            let tensor = st.load(name, var.device()).unwrap();
            var.set(&tensor).unwrap();
        }
    }
    nnue.enable_nnue();

    Engine::new(&config, hce, Some(Box::new(nnue)), stop)
}

fn bench_positions(c: &mut Criterion) {
    let mut engine = create_engine();
    let mut group = c.benchmark_group(format!("search/depth_{}", DEPTH));

    group.sample_size(SAMPLE_SIZE);
    group.sampling_mode(SamplingMode::Flat);

    for fen in POSITIONS {
        let board: Board = fen.parse().unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(fen), &board, |b, board| {
            b.iter(|| {
                engine.new_game();
                engine.set_position(board.clone(), None);
                black_box(engine.search(
                    &GoParams {
                        depth: Some(DEPTH),
                        ..Default::default()
                    },
                    None,
                ))
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_positions);
criterion_main!(benches);
