use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};

use ahash::AHashSet;
use cozy_chess::Board;
use evaluation::{HCE, NNUE};
use uci::{UciOutput, commands::Info, pv_to_uci};

use crate::pv::{MultiPvSearchContext, PvLine, PvTable};

use crate::{
    EngineConfig,
    history::{CaptureHistory, ContinuationHistory, CorrectionHistory, HistoryHeuristic},
    lmr::LmrTable,
    stack::SearchStack,
    transposition::TranspositionTable,
    utils::{convert_centipawn_score, convert_mate_score},
};

mod eval;
mod extension;
mod pruning;
mod quiescence;
mod reduction;
mod search;
mod singular;

use crate::MAX_DEPTH;

pub struct Engine {
    /// Configuration for the engine
    config: EngineConfig,

    /// Signal to terminate search (time control or UCI stop)
    stop: Arc<AtomicBool>,

    /// Hand-crafted evaluation
    hce: Box<dyn HCE>,
    /// Neural network evaluation
    nnue: Option<Box<dyn NNUE>>,

    /// Multi-PV search context (exclusions, PVs, etc)
    multi_pv: MultiPvSearchContext,

    /// The position we are finding the best move for (root position)
    board: Board,
    /// Position hashes for repetition detection - all positions up until the search.
    game_history: AHashSet<u64>,

    /// Number of nodes searched
    nodes: u32,

    /// Selective depth (max ply reached including quiescence - deepest we have gotten)
    max_ply_reached: u8,

    /// Transposition table
    tt: TranspositionTable,

    /// Tracks active search path - used for repetition, improving, etc.
    search_stack: SearchStack,

    /// Scores quiet moves by search success
    history_heuristic: HistoryHeuristic,
    /// Scores captures by search success
    capture_history: CaptureHistory,
    /// Scores based on move sequences
    continuation_history: Box<ContinuationHistory>,
    /// Correction history for static eval adjustment
    correction_history: CorrectionHistory,

    /// Late Move Reductions table
    lmr: LmrTable,

    /// Triangular PV table for the principal variation
    pv_table: PvTable,
}

impl Engine {
    pub fn new(
        config: &EngineConfig,
        hce: Box<dyn HCE>,
        nnue: Option<Box<dyn NNUE>>,
        stop: Arc<AtomicBool>,
    ) -> Self {
        let mut instance = Self {
            config: config.clone(),
            stop,

            hce,
            nnue,

            multi_pv: MultiPvSearchContext::new(),

            board: Board::default(),
            game_history: AHashSet::new(),
            nodes: 0,
            max_ply_reached: 1,

            tt: TranspositionTable::new(1),

            search_stack: SearchStack::with_capacity(MAX_DEPTH),

            history_heuristic: HistoryHeuristic::new(1, 1, 1, 1, 1, 1),
            capture_history: CaptureHistory::new(1, 1, 1),
            continuation_history: Box::new(ContinuationHistory::new(1, 1, 1, 1)),
            correction_history: CorrectionHistory::new(1, 1, 1, 1, 1, 1, 1, 1),

            lmr: LmrTable::new(config.lmr_divisor.value as f32 / 100.0),

            pv_table: PvTable::new(),
        };

        instance.configure(config, true);

        instance
    }

    pub fn configure(&mut self, config: &EngineConfig, init: bool) {
        let old_config = self.config.clone();
        self.config = config.clone();

        // Update the HCE
        self.hce = Box::new(hce::Evaluator::new(config.get_hce_config()));

        if init || old_config.hash_size.value != config.hash_size.value {
            self.tt = TranspositionTable::new(config.hash_size.value as usize);
        }

        if init || !self.history_heuristic.matches_config(config) {
            self.history_heuristic.configure(config);
        }

        if init || !self.capture_history.matches_config(config) {
            self.capture_history.configure(config);
        }

        if init || !self.continuation_history.matches_config(config) {
            self.continuation_history.configure(config);
        }

        if init || !self.correction_history.matches_config(config) {
            self.correction_history.configure(config);
        }
    }

    pub fn name(&self) -> String {
        if let Some(nnue) = &self.nnue {
            format!("Negamax ({})", nnue.name())
        } else {
            format!("Negamax ({})", self.hce.name())
        }
    }

    pub fn new_game(&mut self) {
        self.init_game();
    }

    pub fn set_position(&mut self, board: Board, game_history: Option<AHashSet<u64>>) {
        self.board = board;
        self.game_history = game_history.unwrap_or_default();
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub(super) fn init_game(&mut self) {
        self.tt.clear();
        self.history_heuristic.reset();
        self.capture_history.reset();
        self.continuation_history.reset();
        self.correction_history.reset();
        self.search_stack.clear();
    }

    pub(super) fn send_search_info(
        &self,
        output: &Sender<UciOutput>,
        current_depth: u8,
        pv: &PvLine,
        elapsed: std::time::Duration,
    ) {
        let found_checkmate = pv.score.abs() >= evaluation::scores::MATE_VALUE - MAX_DEPTH as i16;
        let nps = (self.nodes as f32 / elapsed.as_secs_f32()) as u32;

        output
            .send(UciOutput::Info(Info {
                depth: current_depth,
                sel_depth: self.max_ply_reached,
                multipv: (pv.pv_index + 1) as u8, // UCI uses 1-based indexing
                nodes: self.nodes,
                nodes_per_second: nps,
                hashfull: self.tt.hashfull(),
                time: elapsed.as_millis() as u32,
                score: if found_checkmate {
                    convert_mate_score(pv.score)
                } else {
                    convert_centipawn_score(pv.score)
                },
                pv: pv_to_uci(&self.board, &pv.line),
            }))
            .unwrap();
    }
}
