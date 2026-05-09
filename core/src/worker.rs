use std::sync::mpsc::{Receiver, Sender};

use ahash::AHashSet;
use cozy_chess::Board;
use search::{Engine, EngineConfig};
use uci::{NULL_MOVE, UciOutput, move_to_uci};

use crate::display::display_position;
use crate::search_metadata::SearchResultMeta;

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
    /// Display current position.
    Display,
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
    last_search: Option<SearchResultMeta>,
}

impl EngineWorker {
    pub fn new(engine: Engine, rx: Receiver<EngineCommand>, output: Sender<UciOutput>) -> Self {
        Self {
            engine,
            rx,
            output,
            last_search: None,
        }
    }

    /// Main loop: process commands until Quit is received.
    pub fn run(mut self) {
        while let Ok(cmd) = self.rx.recv() {
            match cmd {
                EngineCommand::Go(params) => {
                    let stm = self.engine.board().side_to_move();
                    let result = self.engine.search(&params, Some(&self.output));

                    // UCI requires bestmove for every "go" command, even in checkmate positions
                    let mut uci_move = NULL_MOVE.to_string();

                    if let Some(ref r) = result {
                        let lines = r.lines();
                        if !lines.is_empty() {
                            self.last_search = Some(SearchResultMeta::new(lines.to_vec(), stm));
                        }
                        if let Some(mv) = r.primary().and_then(search::PvLine::best_move) {
                            uci_move = move_to_uci(self.engine.board(), mv);
                        }
                    }

                    let _ = self.output.send(UciOutput::BestMove(uci_move));
                }
                EngineCommand::SetPosition { board, history } => {
                    self.last_search = None;
                    self.engine.set_position(board, Some(history));
                }
                EngineCommand::NewGame => {
                    self.last_search = None;
                    self.engine.new_game();
                }
                EngineCommand::Configure(config) => {
                    self.engine.configure(&config, false);
                }
                EngineCommand::Display => {
                    display_position(self.engine.board(), self.last_search.as_ref())
                }
                EngineCommand::Quit => break,
            }
        }
    }
}
