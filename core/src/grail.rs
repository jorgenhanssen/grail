use std::io::BufRead;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Sender},
};
use std::thread::{self, JoinHandle};

use config::EngineConfig;
use uci::{Decoder, UciConnection, UciInput, UciOutput};

use crate::engine::create_engine;
use crate::worker::{EngineCommand, EngineWorker};

const ENGINE_NAME: &str = "Grail";
const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
const ENGINE_AUTHOR: &str = "Jørgen Hanssen";

/// The main UCI application.
///
/// Handles the UCI protocol on the main thread and coordinates
/// the engine worker thread via channels.
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
    /// If an initial line is provided, it is executed and the program exits.
    pub fn run(mut self, initial: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let decoder = Decoder::new();

        match initial {
            Some(line) => {
                self.handle(decoder.decode(line.trim()));
            }
            None => {
                let stdin = std::io::stdin();
                for line in stdin.lock().lines() {
                    let line = line?;
                    if !self.handle(decoder.decode(line.trim())) {
                        break;
                    }
                }
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
            // TODO: Implement debug mode: send extra info via "info string" when enabled
            UciInput::Debug(_enabled) => {}
            UciInput::SetOption { name, value } => {
                if let Err(e) = self.config.update_from_uci(&name, &value) {
                    let _ = self.output.send(UciOutput::InfoString(e));
                } else {
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
            UciInput::Display => {
                let _ = self.cmd_tx.send(EngineCommand::Display);
            }
            UciInput::Bench => {
                let _ = self.cmd_tx.send(EngineCommand::Bench);
            }
            UciInput::Quit => return false,
            UciInput::Unknown(_) => {} // Ignore unknown commands per UCI spec
        }
        true
    }

    fn shutdown(self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.cmd_tx.send(EngineCommand::Quit);
        let _ = self.worker_handle.join();
    }
}
