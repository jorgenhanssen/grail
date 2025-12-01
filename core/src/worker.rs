use std::sync::mpsc::{Receiver, Sender};

use ahash::AHashSet;
use cozy_chess::Board;
use search::{Engine, EngineConfig};
use uci::{move_to_uci, UciOutput, NULL_MOVE};

/// Commands sent from the UCI thread to the engine worker.
pub enum EngineCommand {
    /// Update engine configuration.
    Configure(Box<EngineConfig>),
    /// Reset engine state for a new game.
    NewGame,
    /// Set the position to search from.
    SetPosition {
        board: Board,
        history: AHashSet<u64>,
    },
    /// Start searching with the given parameters.
    Go(uci::commands::GoParams),
    /// Shut down the worker thread.
    Quit,
}

/// Engine worker that processes commands on a dedicated thread.
///
/// Owns the search engine and receives commands via channel from
/// the main UCI thread. Runs searches and sends results back.
pub struct EngineWorker {
    engine: Engine,
    rx: Receiver<EngineCommand>,
    output: Sender<UciOutput>,
}

impl EngineWorker {
    pub fn new(engine: Engine, rx: Receiver<EngineCommand>, output: Sender<UciOutput>) -> Self {
        Self { engine, rx, output }
    }

    /// Main loop: process commands until Quit is received.
    pub fn run(mut self) {
        while let Ok(cmd) = self.rx.recv() {
            match cmd {
                EngineCommand::Go(params) => {
                    let result = self.engine.search(&params, Some(&self.output));

                    // UCI requires bestmove for every "go" command, even in checkmate positions
                    let uci_move = result
                        .map(|(mv, _)| move_to_uci(self.engine.board(), mv))
                        .unwrap_or_else(|| NULL_MOVE.to_string());

                    let _ = self.output.send(UciOutput::BestMove(uci_move));
                }
                EngineCommand::SetPosition { board, history } => {
                    self.engine.set_position(board, Some(history));
                }
                EngineCommand::NewGame => {
                    self.engine.new_game();
                }
                EngineCommand::Configure(config) => {
                    self.engine.configure(&config, false);
                }
                EngineCommand::Quit => break,
            }
        }
    }
}
