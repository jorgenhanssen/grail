use std::io::BufRead;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Sender},
};
use std::thread::{self, JoinHandle};

use config::EngineConfig;
use search::Engine;
use uci::{Decoder, UciConnection, UciInput, UciOutput, list_uci_options, set_uci_option};

use crate::worker::{EngineCommand, EngineWorker};

static NNUE_BYTES: &[u8] = include_bytes!("../../nnue/model.safetensors");

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
    uci_tx: Sender<UciOutput>,
    worker_handle: JoinHandle<()>,
}

impl Grail {
    /// Creates a new Grail instance, spawning the engine worker thread.
    pub fn new() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (cmd_tx, cmd_rx) = mpsc::channel();

        let uci = UciConnection::new();
        let uci_tx = uci.output_sender();

        let config = EngineConfig::default();
        let engine = Engine::new(
            &config,
            Arc::clone(&stop),
            nnue::Evaluator::from_safetensors(NNUE_BYTES).expect("Failed to load embedded NNUE"),
        );

        let worker = EngineWorker::new(engine, cmd_rx, uci_tx.clone());
        let worker_handle = thread::spawn(move || worker.run());

        Self {
            config,
            stop,
            cmd_tx,
            uci_tx,
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
                self.send_uci_output(UciOutput::IdName(format!(
                    "{} {}",
                    ENGINE_NAME, ENGINE_VERSION
                )));
                self.send_uci_output(UciOutput::IdAuthor(ENGINE_AUTHOR.to_string()));
                for option in list_uci_options(&self.config) {
                    self.send_uci_output(UciOutput::Option(option));
                }
                self.send_uci_output(UciOutput::UciOk);
            }
            UciInput::IsReady => self.send_uci_output(UciOutput::ReadyOk),
            // TODO: Implement debug mode: send extra info via "info string" when enabled
            UciInput::Debug(_enabled) => {}
            UciInput::SetOption { name, value } => {
                match set_uci_option(&mut self.config, &name, &value) {
                    Ok(()) => self.send_engine_command(EngineCommand::Configure(Box::new(
                        self.config.clone(),
                    ))),
                    Err(error) => self.send_uci_output(UciOutput::InfoString(error)),
                }
            }
            UciInput::UciNewGame => self.send_engine_command(EngineCommand::NewGame),
            UciInput::Position {
                board,
                game_history,
            } => self.send_engine_command(EngineCommand::SetPosition {
                board,
                history: game_history,
            }),
            UciInput::Go(params) => {
                self.stop.store(false, Ordering::Relaxed);
                self.send_engine_command(EngineCommand::Go(params));
            }
            UciInput::Stop => self.stop.store(true, Ordering::Relaxed),
            UciInput::Display => self.send_engine_command(EngineCommand::Display),
            UciInput::Eval => self.send_engine_command(EngineCommand::Eval),
            UciInput::Bench => self.send_engine_command(EngineCommand::Bench),
            UciInput::Quit => return false,
            UciInput::Unknown(_) => {} // Ignore unknown commands per UCI spec
        }
        true
    }

    fn send_uci_output(&self, output: UciOutput) {
        let _ = self.uci_tx.send(output);
    }

    fn send_engine_command(&self, command: EngineCommand) {
        let _ = self.cmd_tx.send(command);
    }

    fn shutdown(self) {
        self.stop.store(true, Ordering::Relaxed);
        self.send_engine_command(EngineCommand::Quit);
        let _ = self.worker_handle.join();
    }
}
