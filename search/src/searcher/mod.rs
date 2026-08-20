use std::sync::{Arc, mpsc::Sender};
use std::time::Instant;

use ahash::AHashSet;
use cozy_chess::Board;
use uci::{UciOutput, commands::Info, pv_to_uci};

use config::EngineConfig;

use crate::pv::{MultiPvSearchContext, PvLine, PvTable};
use crate::{
    history::{CaptureHistory, ContinuationHistory, HistoryHeuristic},
    lmr::LmrTable,
    scores::MATE_VALUE,
    stack::SearchStack,
    utils::{convert_centipawn_score, convert_mate_score},
};

pub(crate) use shared_state::SharedSearcherState;

mod eval;
mod extension;
mod pruning;
mod quiescence;
mod reduction;
mod search;
mod shared_state;
mod singular;

use crate::MAX_DEPTH;

pub struct Searcher {
    /// Shared state across all searchers (TT, correction history, stop etc)
    shared: Arc<SharedSearcherState>,

    /// ID/index for this searcher (0 = main, 1.. = helpers)
    thread_id: usize,

    /// Configuration for the engine
    config: EngineConfig,

    /// Neural network evaluation
    evaluator: nnue::Evaluator,

    /// Multi-PV search context (exclusions, PVs, etc)
    multi_pv: MultiPvSearchContext,

    /// The position we are finding the best move for (root position)
    board: Board,
    /// Position hashes for repetition detection - all positions up until the search.
    game_history: AHashSet<u64>,

    /// Number of nodes searched
    nodes: u64,

    /// Selective depth (max ply reached including quiescence - deepest we have gotten)
    max_ply_reached: u8,

    /// Current iterative deepening depth
    root_depth: u8,

    /// Tracks active search path - used for repetition, improving, etc.
    search_stack: SearchStack,

    /// Scores quiet moves by search success
    history_heuristic: HistoryHeuristic,
    /// Scores captures by search success
    capture_history: CaptureHistory,
    /// Scores based on move sequences
    continuation_history: ContinuationHistory,

    /// Late Move Reductions table
    lmr: LmrTable,

    /// Triangular PV table for the principal variation
    pv_table: PvTable,

    /// Hard time deadline for the search (main searcher).
    deadline: Option<Instant>,

    /// Hard node-count limit for the search. When the cumulative node count
    /// (across all threads) reaches this, the search is stopped.
    node_limit: Option<u64>,

    /// Disable NMP until this ply.
    /// Only used during nmp verification (so set to 0 in normal search)
    nmp_min_ply: u8,
}

impl Searcher {
    /// How often (in nodes) to sync the local node count to the shared atomic counter.
    const NODE_SYNC_INTERVAL: u64 = 1024;

    /// How often (in nodes) to check if the hard time deadline has been reached.
    const TIME_CHECK_INTERVAL: u64 = 1024;

    pub fn new(
        shared: Arc<SharedSearcherState>,
        thread_id: usize,
        config: &EngineConfig,
        evaluator: nnue::Evaluator,
    ) -> Self {
        let mut instance = Self {
            shared,
            thread_id,
            config: config.clone(),

            evaluator,

            multi_pv: MultiPvSearchContext::new(),

            board: Board::default(),
            game_history: AHashSet::new(),
            nodes: 0,
            max_ply_reached: 1,
            root_depth: 0,

            search_stack: SearchStack::with_capacity(MAX_DEPTH),

            history_heuristic: HistoryHeuristic::new(1, 1, 1),
            capture_history: CaptureHistory::new(1, 1, 1),
            continuation_history: ContinuationHistory::new(1, 1, 1, 1),

            lmr: LmrTable::new(config.lmr_divisor),

            pv_table: PvTable::new(),

            deadline: None,
            node_limit: None,

            nmp_min_ply: 0,
        };

        instance.configure(config);
        instance
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.config = config.clone();

        if !self.history_heuristic.matches_config(config) {
            self.history_heuristic.configure(config);
        }
        if !self.capture_history.matches_config(config) {
            self.capture_history.configure(config);
        }
        if !self.continuation_history.matches_config(config) {
            self.continuation_history.configure(config);
        }
        if !self.lmr.matches_config(config) {
            self.lmr.configure(config);
        }
    }

    pub fn set_position(&mut self, board: Board, game_history: AHashSet<u64>) {
        self.board = board;
        self.game_history = game_history;
    }

    pub fn new_game(&mut self) {
        self.history_heuristic.reset();
        self.capture_history.reset();
        self.continuation_history.reset();
        self.search_stack.clear();
    }

    pub fn sync_nodes(&mut self) {
        if self.nodes > 0 {
            self.shared.add_nodes(self.nodes);
            self.nodes = 0;
        }
    }

    fn increment_nodes(&mut self) {
        self.nodes = self.nodes.wrapping_add(1);
        if self.nodes >= Self::NODE_SYNC_INTERVAL {
            self.sync_nodes();
        }
    }

    /// Checks if any hard search limit has been reached (time deadline or
    /// total node count). Sets the shared stop flag when triggered.
    fn check_limits(&self) {
        if !self.nodes.is_multiple_of(Self::TIME_CHECK_INTERVAL) {
            return;
        }

        if let Some(deadline) = self.deadline {
            if Instant::now() >= deadline {
                self.shared.set_stop(true);
                return;
            }
        }

        if let Some(node_limit) = self.node_limit {
            // shared.total_nodes() lags by up to NODE_SYNC_INTERVAL per worker,
            // so add the local count for an accurate-enough estimate.
            let total = self.shared.total_nodes() + self.nodes;
            if total >= node_limit {
                self.shared.set_stop(true);
            }
        }
    }

    fn total_nodes(&self) -> u64 {
        self.shared.total_nodes()
    }

    fn send_search_info(
        &self,
        output: &Sender<UciOutput>,
        current_depth: u8,
        pv: &PvLine,
        elapsed: std::time::Duration,
    ) {
        let found_checkmate = pv.score.abs() >= MATE_VALUE - MAX_DEPTH as i16;
        let total = self.total_nodes();
        let secs = elapsed.as_secs_f64();
        let nps = if secs > 0.0 {
            (total as f64 / secs) as u64
        } else {
            0
        };

        output
            .send(UciOutput::Info(Info {
                depth: current_depth,
                sel_depth: self.max_ply_reached,
                multipv: (pv.pv_index + 1) as u8, // UCI uses 1-based indexing
                nodes: total,
                nodes_per_second: nps,
                hashfull: self.shared.tt().hashfull(),
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
