use crate::engine;
use cozy_chess::{Board, Move};
use search::EngineConfig;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use uci::commands::{GoParams, Info};
use uci::{move_to_uci, Encoder, UciOutput};

pub fn run(depth: u8) {
    let config = EngineConfig::default();
    let engine = engine::create_engine(&config);

    let benchmark = Benchmark::new(engine, depth);
    benchmark.run();
}

struct Benchmark {
    depth: u8,
    engine: search::Engine,
}

impl Benchmark {
    fn new(engine: search::Engine, depth: u8) -> Self {
        Self { depth, engine }
    }

    fn run(mut self) {
        self.print_header();

        let board = Board::default();
        // TODO: Consider making history optional and defaulting to empty
        let history = ahash::AHashSet::new();
        self.engine.set_position(board, history);

        let params = self.create_search_params();
        let (tx, rx) = mpsc::channel();

        let printer = InfoPrinter::spawn(rx);
        let result = self.execute_search(&params, tx);

        let last_info = printer.join();
        self.print_summary(&result, last_info);
    }

    fn print_header(&self) {
        println!("Running benchmark: depth {}\n", self.depth);
    }

    fn create_search_params(&self) -> GoParams {
        GoParams {
            depth: Some(self.depth),
            ..Default::default()
        }
    }

    fn execute_search(&mut self, params: &GoParams, tx: Sender<UciOutput>) -> SearchResult {
        let start = Instant::now();
        let result = self.engine.search(params, Some(&tx));
        let elapsed = start.elapsed();

        SearchResult { result, elapsed }
    }

    fn print_summary(&self, search_result: &SearchResult, last_info: Option<Info>) {
        println!("\n=== Benchmark Summary ===");

        if let Some((best_move, score)) = &search_result.result {
            let uci_move = move_to_uci(self.engine.board(), *best_move);
            println!("Best move: {}", uci_move);
            println!("Score: {}", score);
        } else {
            println!("Benchmark failed to complete");
            return;
        }

        if let Some(info) = last_info {
            println!("Nodes: {}", info.nodes);
            println!("NPS: {}", info.nodes_per_second);
            println!("Time: {} ms", info.time);
        } else {
            println!("Time: {} ms", search_result.elapsed.as_millis());
        }
    }
}

struct SearchResult {
    result: Option<(Move, i16)>,
    elapsed: Duration,
}

struct InfoPrinter {
    handle: JoinHandle<Option<Info>>,
}

impl InfoPrinter {
    fn spawn(rx: Receiver<UciOutput>) -> Self {
        let handle = thread::spawn(move || {
            let encoder = Encoder {};
            let mut last_info = None;

            for output in rx {
                if let UciOutput::Info(info) = output {
                    println!("{}", encoder.encode(&UciOutput::Info(info.clone())));
                    last_info = Some(info);
                }
            }

            last_info
        });

        Self { handle }
    }

    fn join(self) -> Option<Info> {
        self.handle.join().expect("Info printer thread panicked")
    }
}
