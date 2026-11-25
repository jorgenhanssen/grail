use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};
use std::thread;

use ahash::AHashSet;
use config::EngineConfig;
use cozy_chess::Board;
use pyrrhic_rs::TableBases;
use uci::UciOutput;
use utils::Node;

use crate::{
    result::SearchResult,
    searcher::{Searcher, SharedSearcherState},
    tablebase::CozyAdapter,
    transposition::TranspositionTable,
};

#[derive(Clone, Default)]
pub struct SearchHandle {
    stop_requested: Arc<AtomicBool>,
}

impl SearchHandle {
    pub fn stop(&self) {
        self.stop_requested.store(true, Ordering::Relaxed);
    }

    pub fn is_stopped(&self) -> bool {
        self.stop_requested.load(Ordering::Relaxed)
    }
}

pub struct Engine {
    config: EngineConfig,
    shared: Arc<SharedSearcherState>,
    board: Board,
    game_history: AHashSet<u64>,
    searchers: Vec<Searcher>,
    evaluator: nnue::Evaluator,
}

impl Engine {
    pub fn new(config: &EngineConfig, evaluator: nnue::Evaluator) -> Self {
        let shared = Arc::new(SharedSearcherState::new(config));

        if !config.syzygy_path.is_empty() {
            shared.init_tablebases(&config.syzygy_path);
        }

        let searchers = (0..config.threads)
            .map(|thread_id| {
                Searcher::new(Arc::clone(&shared), thread_id, config, evaluator.clone())
            })
            .collect();

        Self {
            config: config.clone(),
            shared,
            board: Board::default(),
            game_history: AHashSet::new(),
            searchers,
            evaluator,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        if self.config.hash_size != config.hash_size {
            *self.shared.tt() = TranspositionTable::new(config.hash_size as usize);
        }

        if !self.shared.correction().matches_config(config) {
            self.shared.correction().configure(config);
        }

        if self.config.syzygy_path != config.syzygy_path {
            if config.syzygy_path.is_empty() {
                self.shared.clear_tablebases();
            } else {
                self.shared.init_tablebases(&config.syzygy_path);
            }
        }

        self.searchers.truncate(config.threads);
        for searcher in &mut self.searchers {
            searcher.configure(config);
        }

        while self.searchers.len() < config.threads {
            let thread_id = self.searchers.len();
            self.searchers.push(Searcher::new(
                Arc::clone(&self.shared),
                thread_id,
                config,
                self.evaluator.clone(),
            ));
        }

        self.config.clone_from(config);
    }

    pub fn new_game(&mut self) {
        self.shared.tt().clear();
        self.shared.correction().reset();

        for searcher in &mut self.searchers {
            searcher.new_game();
        }
    }

    pub fn set_position(&mut self, board: Board, game_history: Option<AHashSet<u64>>) {
        self.board = board;
        self.game_history = game_history.unwrap_or_default();
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub fn get_static_eval(&mut self, node: &Node) -> i16 {
        self.evaluator.evaluate(node)
    }

    pub fn set_tablebases(&self, tb: TableBases<CozyAdapter>) {
        self.shared.set_tablebases(tb);
    }

    pub fn search(
        &mut self,
        params: &uci::commands::GoParams,
        output: Option<&Sender<UciOutput>>,
        handle: Option<SearchHandle>,
    ) -> Option<SearchResult> {
        let handle = handle.unwrap_or_default();

        for searcher in &mut self.searchers {
            searcher.prepare_new_search(
                self.board.clone(),
                self.game_history.clone(),
                handle.clone(),
            );
        }

        self.shared.reset_nodes();
        self.shared.tt().age();

        thread::scope(|s| {
            let (main, helpers) = self.searchers.split_first_mut().unwrap();

            for helper in helpers.iter_mut() {
                s.spawn(move || {
                    helper.search_auxiliary();
                    helper.sync_nodes();
                });
            }

            let result = main.search(params, output);
            main.sync_nodes();

            handle.stop();
            result
        })
    }
}
