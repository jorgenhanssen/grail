use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};

use ahash::AHashSet;
use chess::{Board, ChessMove};
use evaluation::{hce, PieceValues, HCE, NNUE};
use uci::{commands::Info, UciOutput};

use crate::{
    history::{CaptureHistory, ContinuationHistory, HistoryHeuristic},
    stack::SearchStack,
    transposition::{QSTable, TranspositionTable},
    utils::{convert_centipawn_score, convert_mate_score},
    EngineConfig,
};

mod eval;
mod pruning;
mod quiescence;
mod search;

const MAX_DEPTH: usize = 100;

pub struct Engine {
    config: EngineConfig,
    piece_values: PieceValues,

    hce: Box<dyn HCE>,
    nnue: Option<Box<dyn NNUE>>,

    board: Board,
    game_history: AHashSet<u64>,
    nodes: u32,
    killer_moves: [[Option<ChessMove>; 2]; MAX_DEPTH], // 2 per depth
    current_pv: Vec<ChessMove>,
    max_depth_reached: u8,
    stop: Arc<AtomicBool>,

    tt: TranspositionTable,
    qs_tt: QSTable,

    search_stack: SearchStack,

    history_heuristic: HistoryHeuristic,
    capture_history: CaptureHistory,
    continuation_history: Box<ContinuationHistory>,
}

impl Engine {
    pub fn new(config: &EngineConfig, hce: Box<dyn HCE>, nnue: Option<Box<dyn NNUE>>) -> Self {
        let mut instance = Self {
            config: config.clone(),
            piece_values: config.get_piece_values(),
            stop: Arc::new(AtomicBool::new(false)),

            hce,
            nnue,

            board: Board::default(),
            game_history: AHashSet::new(),
            nodes: 0,
            killer_moves: [[None; 2]; MAX_DEPTH],
            current_pv: Vec::new(),
            max_depth_reached: 1,

            tt: TranspositionTable::new(1),
            qs_tt: QSTable::new(1),

            search_stack: SearchStack::with_capacity(MAX_DEPTH),

            history_heuristic: HistoryHeuristic::new(1, 1, 1, 1, 1, 1),
            capture_history: CaptureHistory::new(1, 1, 1),
            continuation_history: Box::new(ContinuationHistory::new(1, 1, 1, 1)),
        };

        instance.configure(config, true);

        instance
    }

    pub fn configure(&mut self, config: &EngineConfig, init: bool) {
        let old_config = self.config.clone();
        self.config = config.clone();

        // Update the HCE
        // TODO: Find a better way to do this
        self.piece_values = config.get_piece_values();
        self.hce = Box::new(hce::Evaluator::new(
            self.piece_values,
            config.get_hce_config(),
        ));

        if init || old_config.hash_size.value != config.hash_size.value {
            self.configure_transposition_tables();
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

    pub fn set_position(&mut self, board: Board, game_history: AHashSet<u64>) {
        self.board = board;
        self.game_history = game_history;
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    #[inline(always)]
    pub(super) fn init_game(&mut self) {
        self.tt.clear();
        self.qs_tt.clear();
        self.history_heuristic.reset();
        self.capture_history.reset();
        self.continuation_history.reset();
        self.search_stack.clear();
    }

    pub(super) fn send_search_info(
        &self,
        output: &Sender<UciOutput>,
        current_depth: u8,
        best_score: i16,
        elapsed: std::time::Duration,
    ) {
        let found_checkmate = best_score.abs() >= evaluation::scores::MATE_VALUE - MAX_DEPTH as i16;
        let nps = (self.nodes as f32 / elapsed.as_secs_f32()) as u32;

        output
            .send(UciOutput::Info(Info {
                depth: current_depth,
                sel_depth: self.max_depth_reached,
                nodes: self.nodes,
                nodes_per_second: nps,
                time: elapsed.as_millis() as u32,
                score: if found_checkmate {
                    convert_mate_score(best_score)
                } else {
                    convert_centipawn_score(best_score)
                },
                pv: self.current_pv.clone(),
            }))
            .unwrap();
    }

    fn configure_transposition_tables(&mut self) {
        let total_size_mb = self.config.hash_size.value;
        let qs_size_mb = total_size_mb / 3;
        let main_size_mb = total_size_mb - qs_size_mb;

        self.tt = TranspositionTable::new(main_size_mb as usize);
        self.qs_tt = QSTable::new(qs_size_mb as usize);
    }
}
