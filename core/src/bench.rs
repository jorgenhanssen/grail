//! Benchmark mode - runs a fixed-depth search for performance profiling.

use std::sync::{
    atomic::AtomicBool,
    mpsc::{self, Receiver},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cozy_chess::{Board, Move};
use search::EngineConfig;
use uci::commands::{GoParams, Info};
use uci::{move_to_uci, Encoder, UciOutput};

use crate::engine::create_engine;

/// Benchmark runner for performance profiling.
pub struct Bench {
    depth: u8,
    engine: search::Engine,
}

impl Bench {
    /// Creates a new benchmark runner with the given search depth.
    pub fn new(depth: u8) -> Self {
        let config = EngineConfig::default();
        let stop = Arc::new(AtomicBool::new(false));

        let engine = create_engine(&config, stop);

        Self { depth, engine }
    }

    /// Runs the benchmark search.
    pub fn run(mut self) {
        self.print_header();

        self.engine.set_position(Board::default(), None);

        let params = GoParams {
            depth: Some(self.depth),
            ..Default::default()
        };

        // Create a channel for the search to send info messages.
        // The printer thread receives and displays them in real-time.
        let (tx, rx) = mpsc::channel();
        let printer = InfoPrinter::spawn(rx);

        let start = Instant::now();
        let result = self.engine.search(&params, Some(&tx));
        let elapsed = start.elapsed();

        // Drop sender so InfoPrinter knows to stop
        drop(tx);

        // Wait for the printer thread to finish and get the last info,
        // which contains the final node count, NPS, and time from the engine.
        let last_info = printer.join();
        self.print_summary(result, elapsed, last_info);
    }

    fn print_header(&self) {
        println!("Running benchmark: depth {}\n", self.depth);
    }

    fn print_summary(
        &self,
        result: Option<(Move, i16)>,
        elapsed: Duration,
        last_info: Option<Info>,
    ) {
        println!("\n=== Benchmark Summary ===");

        if let Some((best_move, score)) = result {
            let uci_move = move_to_uci(self.engine.board(), best_move);
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
            println!("Time: {} ms", elapsed.as_millis());
        }
    }
}

/// Handles printing search info on a separate thread.
///
/// The search engine sends `UciOutput::Info` messages through a channel as it
/// progresses. This thread prints each info line and keeps track of the last
/// one received, which contains the final statistics (nodes, NPS, time).
struct InfoPrinter {
    handle: JoinHandle<Option<Info>>,
}

impl InfoPrinter {
    /// Spawns the printer thread that listens for search info messages.
    fn spawn(rx: Receiver<UciOutput>) -> Self {
        let handle = thread::spawn(move || {
            let encoder = Encoder {};
            let mut last_info = None;

            // Loop until channel is closed (sender dropped)
            for output in rx {
                if let UciOutput::Info(info) = output {
                    println!("{}", encoder.encode(&UciOutput::Info(info.clone())));
                    last_info = Some(info);
                }
            }

            // Return the last info for the summary statistics
            last_info
        });

        Self { handle }
    }

    /// Waits for the printer thread to finish and returns the last info received.
    fn join(self) -> Option<Info> {
        self.handle.join().expect("Info printer thread panicked")
    }
}
