use std::sync::{Arc, atomic::AtomicBool, mpsc::Sender};
use std::thread;

use ahash::AHashSet;
use cozy_chess::Board;
use uci::UciOutput;

use pyrrhic_rs::TableBases;

use crate::{
    EngineConfig,
    result::SearchResult,
    searcher::{Searcher, SharedSearcherState},
    tablebase::CozyAdapter,
    transposition::TranspositionTable,
};

pub struct Engine {
    config: EngineConfig,
    shared: Arc<SharedSearcherState>,
    board: Board,
    game_history: AHashSet<u64>,
    searchers: Vec<Searcher>,
    create_evaluator: fn() -> nnue::Evaluator,
}

impl Engine {
    pub fn new(
        config: &EngineConfig,
        stop: Arc<AtomicBool>,
        create_evaluator: fn() -> nnue::Evaluator,
    ) -> Self {
        let shared = Arc::new(SharedSearcherState::new(config, stop));

        let mut engine = Self {
            config: config.clone(),
            shared,
            board: Board::default(),
            game_history: AHashSet::new(),
            searchers: Vec::new(),
            create_evaluator,
        };
        engine.configure(config, true);
        engine
    }

    pub fn configure(&mut self, config: &EngineConfig, init: bool) {
        let old_config = self.config.clone();
        self.config = config.clone();

        if init || old_config.hash_size.value != config.hash_size.value {
            *self.shared.tt() = TranspositionTable::new(config.hash_size.value as usize);
        }

        if init || !self.shared.correction().matches_config(config) {
            self.shared.correction().configure(config);
        }

        if init || old_config.syzygy_path.value != config.syzygy_path.value {
            if config.syzygy_path.value.is_empty() {
                self.shared.clear_tablebases();
            } else {
                self.shared.init_tablebases(&config.syzygy_path.value);
            }
        }

        let new_num_threads = config.threads.value;
        while self.searchers.len() < new_num_threads {
            let thread_id = self.searchers.len();
            let evaluator = (self.create_evaluator)();
            self.searchers.push(Searcher::new(
                Arc::clone(&self.shared),
                thread_id,
                config,
                evaluator,
            ));
        }
        self.searchers.truncate(new_num_threads);

        for searcher in &mut self.searchers {
            searcher.configure(config);
        }
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

    pub fn set_tablebases(&self, tb: TableBases<CozyAdapter>) {
        self.shared.set_tablebases(tb);
    }

    pub fn stop(&self) {
        self.shared.set_stop(true);
    }

    pub fn search(
        &mut self,
        params: &uci::commands::GoParams,
        output: Option<&Sender<UciOutput>>,
    ) -> Option<SearchResult> {
        for searcher in &mut self.searchers {
            searcher.set_position(self.board.clone(), self.game_history.clone());
        }

        self.shared.set_stop(false);
        self.shared.reset_nodes();
        self.shared.tt().age();

        let mut result = None;

        let shared = &self.shared;
        let searchers = &mut self.searchers;

        thread::scope(|s| {
            let (main, helpers) = searchers.split_first_mut().unwrap();

            for helper in helpers.iter_mut() {
                s.spawn(move || {
                    helper.search_auxiliary();
                    helper.sync_nodes();
                });
            }

            result = main.search(params, output);
            main.sync_nodes();

            shared.set_stop(true);
        });

        result
    }
}
