//! UCI application - handles the UCI protocol and coordinates the engine worker.

use std::io::BufRead;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Sender},
    Arc,
};
use std::thread::{self, JoinHandle};

use log::debug;
use search::EngineConfig;
use uci::{Decoder, UciConnection, UciInput, UciOutput};

use crate::engine::create_engine;
use crate::worker::{EngineCommand, EngineWorker};

const ENGINE_NAME: &str = "Grail";
const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
const ENGINE_AUTHOR: &str = "Jørgen Hanssen";

/// The main UCI application.
pub struct Grail {
    config: EngineConfig,
    stop: Arc<AtomicBool>,
    cmd_tx: Sender<EngineCommand>,
    output: Sender<UciOutput>,
    worker_handle: JoinHandle<()>,
}

impl Grail {
    /// Creates a new Grail instance, spawning the engine worker thread.
    pub fn new() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (cmd_tx, cmd_rx) = mpsc::channel();

        let uci = UciConnection::new();
        let output = uci.output_sender();

        let config = EngineConfig::default();
        let engine = create_engine(&config, Arc::clone(&stop));

        let worker = EngineWorker::new(engine, cmd_rx, output.clone());
        let worker_handle = thread::spawn(move || worker.run());

        Self {
            config,
            stop,
            cmd_tx,
            output,
            worker_handle,
        }
    }

    /// Runs the UCI protocol loop until quit.
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_greeting();

        let decoder = Decoder::new();
        let stdin = std::io::stdin();

        for line in stdin.lock().lines() {
            let line = line?;
            debug!("Input: {:?}", line.trim());

            let input = decoder.decode(line.trim());
            if !self.handle(input) {
                break;
            }
        }

        self.shutdown();
        Ok(())
    }

    /// Handles a single UCI input. Returns false if we should quit.
    fn handle(&mut self, input: UciInput) -> bool {
        match input {
            UciInput::Uci => {
                let _ = self.output.send(UciOutput::IdName(format!(
                    "{} {}",
                    ENGINE_NAME, ENGINE_VERSION
                )));
                let _ = self
                    .output
                    .send(UciOutput::IdAuthor(ENGINE_AUTHOR.to_string()));
                let _ = self.config.to_uci(&self.output);
                let _ = self.output.send(UciOutput::UciOk);
            }
            UciInput::IsReady => {
                let _ = self.output.send(UciOutput::ReadyOk);
            }
            UciInput::SetOption { name, value } => {
                if let Err(e) = self.config.update_from_uci(&name, &value) {
                    debug!("Option setting failed: {}", e);
                } else {
                    debug!("Set option '{}' to '{}'", name, value);
                    let _ = self
                        .cmd_tx
                        .send(EngineCommand::Configure(Box::new(self.config.clone())));
                }
            }
            UciInput::UciNewGame => {
                let _ = self.cmd_tx.send(EngineCommand::NewGame);
            }
            UciInput::Position {
                board,
                game_history,
            } => {
                let _ = self.cmd_tx.send(EngineCommand::SetPosition {
                    board,
                    history: game_history,
                });
            }
            UciInput::Go(params) => {
                self.stop.store(false, Ordering::Relaxed);
                let _ = self.cmd_tx.send(EngineCommand::Go(params));
            }
            UciInput::Stop => {
                self.stop.store(true, Ordering::Relaxed);
            }
            UciInput::Quit => return false,
            UciInput::Unknown(line) => debug!("Unknown command: {}", line),
        }
        true
    }

    fn send_greeting(&self) {
        let _ = self.output.send(UciOutput::Raw(format!(
            "{} {} by {}",
            ENGINE_NAME, ENGINE_VERSION, ENGINE_AUTHOR
        )));
    }

    fn shutdown(self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.cmd_tx.send(EngineCommand::Quit);
        let _ = self.worker_handle.join();
    }
}
