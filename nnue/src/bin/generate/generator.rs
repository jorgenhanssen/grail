use std::error::Error;
use std::time::Instant;
use std::{path::PathBuf, sync::Arc};

use ahash::AHashSet;
use chess::{Board, ChessMove, Game, MoveGen};
use evaluation::{Evaluator, TraditionalEvaluator};
use nnue::version::VersionManager;
use nnue::NNUE;
use rand::Rng;
use search::{Engine, MinimaxEngine};
use std::sync::Mutex;

use rayon::iter::*;

pub struct Generator {
    threads: usize,
    nnue_path: Option<PathBuf>,
}

impl Generator {
    pub fn new(threads: usize, manager: &VersionManager) -> Result<Self, Box<dyn Error>> {
        let version = manager.get_latest_version()?;

        let generator = match version {
            Some(version) => Self {
                threads,
                nnue_path: Some(manager.file_path(version, "model.safetensors")),
            },
            _ => Self {
                threads,
                nnue_path: None,
            },
        };

        Ok(generator)
    }

    pub fn run(&self, duration: u64, depth: u64) -> Vec<(Board, f32)> {
        log::info!("Generating NNUE samples using {} threads", self.threads);

        // We only want to evaluate positions once
        let global_evaluated = Arc::new(Mutex::new(AHashSet::new()));

        let evaluations = (0..self.threads).into_par_iter().map(|tid| {
            let evaluator: Box<dyn Evaluator> = match &self.nnue_path {
                Some(path) => Box::new(NNUE::new(path.clone())),
                None => Box::new(TraditionalEvaluator),
            };

            let global_evaluated_ref = Arc::clone(&global_evaluated);
            let mut worker = SelfPlayWorker::new(tid, global_evaluated_ref, depth, evaluator);

            worker.play_games(duration)
        });

        evaluations.flatten().collect()
    }
}

struct SelfPlayWorker {
    tid: usize,
    global_evaluated: Arc<Mutex<AHashSet<u64>>>,
    engine: MinimaxEngine,
    depth: u64,

    // Specific to ongoing game
    game: Game,
    positions_in_current_game: AHashSet<u64>,
}

impl SelfPlayWorker {
    pub fn new(
        tid: usize,
        global_evaluated: Arc<Mutex<AHashSet<u64>>>,
        depth: u64,
        evaluator: Box<dyn Evaluator>,
    ) -> Self {
        Self {
            tid,
            global_evaluated,
            game: Game::new(),
            depth,

            engine: MinimaxEngine::new(evaluator),
            positions_in_current_game: AHashSet::new(),
        }
    }

    pub fn play_games(&mut self, duration: u64) -> Vec<(Board, f32)> {
        let start_time = Instant::now();
        let mut evaluations = Vec::new();

        self.reset_game();

        while start_time.elapsed().as_secs() < duration {
            let terminal = self.play_single_move(&mut evaluations);

            if terminal {
                self.reset_game();
            }
        }

        evaluations
    }

    fn play_single_move(&mut self, evaluations: &mut Vec<(Board, f32)>) -> bool {
        if self.game.result().is_some() {
            log::info!("[{}] Game ended: {:?}", self.tid, self.game.result());
            return true;
        }

        let board = self.game.current_position();
        let board_hash = board.get_hash();

        // If we've seen this position in the *current game*, we have a cycle.
        if !self.positions_in_current_game.insert(board_hash) {
            log::info!("[{}] Cycle detected, resetting game", self.tid);
            return true;
        }

        // Select and make a move, possibly storing the board's evaluation.
        let chosen_move = self.select_move(board, evaluations);
        self.game.make_move(chosen_move);

        false
    }

    fn select_move(
        &mut self,
        board: chess::Board,
        evaluations: &mut Vec<(Board, f32)>,
    ) -> ChessMove {
        let moves: Vec<ChessMove> = MoveGen::new_legal(&board).collect();

        if self.position_has_been_evaluated(&board) {
            return random_move(&moves);
        }

        let (engine_move, engine_score) = self.get_engine_move(&board);

        // tanh to force mate scores to be in [-1, 1]
        evaluations.push((board.clone(), engine_score.tanh()));

        if self.should_use_engine_move(&engine_score) {
            engine_move
        } else {
            random_move(&moves)
        }
    }

    #[inline]
    fn position_has_been_evaluated(&mut self, board: &Board) -> bool {
        let hash = board.get_hash();

        let mut evaluated_positions = self.global_evaluated.lock().unwrap();
        if evaluated_positions.contains(&hash) {
            true
        } else {
            // Mark it seen from now on
            evaluated_positions.insert(hash);
            false
        }
    }

    #[inline]
    fn get_engine_move(&mut self, board: &Board) -> (ChessMove, f32) {
        self.engine.set_position(*board);
        self.engine.init_search();
        self.engine.search_root(self.depth)
    }

    #[inline]
    fn should_use_engine_move(&self, score: &f32) -> bool {
        score.abs() > 0.7
    }

    #[inline]
    fn reset_game(&mut self) {
        self.game = Game::new();
        self.positions_in_current_game.clear();
    }
}

#[inline]
fn random_move(moves: &[ChessMove]) -> ChessMove {
    let idx = rand::thread_rng().gen_range(0..moves.len());
    moves[idx]
}
