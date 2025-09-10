use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;
use std::{path::PathBuf, sync::Arc};

use ahash::AHashSet;
use candle_core::Device;
use candle_nn::VarMap;
use chess::{Board, ChessMove, Game, MoveGen};
use evaluation::{Evaluator, TraditionalEvaluator};
use indicatif::{ProgressBar, ProgressStyle};
use nnue::version::VersionManager;
use nnue::NNUE;
use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::Rng;
use search::{Engine, NegamaxEngine};
use std::sync::Mutex;
use uci::commands::GoParams;

#[derive(Clone)]
pub struct MoveDistribution {
    pub moves: Vec<ChessMove>,
    pub evaluations: Vec<i16>,
}

impl MoveDistribution {
    pub fn new(moves: Vec<ChessMove>, evaluations: Vec<i16>) -> Self {
        Self { moves, evaluations }
    }
}

/// Global storage for move evaluation distributions, keyed by position hash
type MoveDistributions = Arc<Mutex<HashMap<u64, MoveDistribution>>>;

pub struct Generator {
    threads: usize,
    nnue_path: Option<PathBuf>,
    version: u32,
}

impl Generator {
    pub fn new(threads: usize, manager: &VersionManager) -> Result<Self, Box<dyn Error>> {
        let version = manager.get_latest_version()?;

        let generator = match version {
            Some(version) => Self {
                threads,
                nnue_path: Some(manager.file_path(version, "model.safetensors")),
                version,
            },
            _ => Self {
                threads,
                nnue_path: None,
                version: 0,
            },
        };

        Ok(generator)
    }

    pub fn run(&self, duration: u64, depth: u8) -> Vec<(String, i16)> {
        let eval_name = match &self.nnue_path {
            Some(path) => path.display().to_string(),
            None => "traditional evaluator".to_string(),
        };

        log::info!(
            "Generating samples using {} threads ({})",
            self.threads,
            eval_name
        );

        let pb = ProgressBar::new(duration);
        pb.set_style(
            ProgressStyle::with_template(
                " {spinner:.cyan} {wide_bar:.cyan/blue} {eta_precise} | {msg}",
            )
            .unwrap(),
        );
        let pb = Arc::new(pb);

        let global_distributions: MoveDistributions = Arc::new(Mutex::new(HashMap::new()));

        let handles: Vec<_> = (0..self.threads)
            .map(|tid| {
                let nnue_path = self.nnue_path.clone();
                let version = self.version;
                let global_distributions = Arc::clone(&global_distributions);
                let pb = Arc::clone(&pb);

                std::thread::spawn(move || {
                    let evaluator: Box<dyn Evaluator> = match &nnue_path {
                        Some(path) => {
                            let mut varmap = VarMap::new();
                            let mut nnue = Box::new(NNUE::new(&varmap, &Device::Cpu, version));
                            varmap.load(path).unwrap();
                            nnue.enable_nnue();
                            nnue
                        }
                        None => Box::new(TraditionalEvaluator),
                    };

                    let mut worker =
                        SelfPlayWorker::new(tid, global_distributions, depth, evaluator);
                    worker.play_games(duration, &pb)
                })
            })
            .collect();

        let evaluations: Vec<_> = handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect();

        pb.finish_with_message(format!("Evaluated {} positions", evaluations.len()));

        evaluations
    }
}

struct SelfPlayWorker {
    tid: usize,
    global_distributions: MoveDistributions,
    engine: NegamaxEngine,
    depth: u8,

    // Specific to ongoing game
    game: Game,
    positions_in_current_game: AHashSet<u64>,
}

impl SelfPlayWorker {
    pub fn new(
        tid: usize,
        global_distributions: MoveDistributions,
        depth: u8,
        evaluator: Box<dyn Evaluator>,
    ) -> Self {
        Self {
            tid,
            global_distributions,
            game: Game::new(),
            depth,

            engine: NegamaxEngine::new(evaluator),
            positions_in_current_game: AHashSet::new(),
        }
    }

    pub fn play_games(&mut self, duration: u64, pb: &ProgressBar) -> Vec<(String, i16)> {
        let start_time = Instant::now();
        let mut evaluations = Vec::new();

        self.reset_game();

        loop {
            let current_elapsed = start_time.elapsed().as_secs();
            if current_elapsed >= duration {
                break;
            }

            if self.tid == 0 {
                let global_count = self.global_distributions.lock().unwrap().len();
                pb.set_message(format!("{} positions", global_count));
                pb.set_position(current_elapsed);
            }

            let terminal = self.play_single_move(&mut evaluations);

            if terminal {
                self.reset_game();
            }
        }

        pb.finish_with_message("waiting...");

        evaluations
    }

    fn play_single_move(&mut self, evaluations: &mut Vec<(String, i16)>) -> bool {
        if self.game.result().is_some() {
            return true;
        }

        let board = self.game.current_position();
        let board_hash = board.get_hash();

        // If we've seen this position in the *current game*, we have a cycle.
        if !self.positions_in_current_game.insert(board_hash) {
            return true;
        }

        // Select and make a move, possibly storing the board's evaluation.
        let (chosen_move, score) = self.select_move(board, evaluations);

        if self.should_abort_game(&score) {
            return true;
        }

        self.game.make_move(chosen_move);

        false
    }

    fn select_move(
        &mut self,
        board: chess::Board,
        evaluations: &mut Vec<(String, i16)>,
    ) -> (ChessMove, i16) {
        let board_hash = board.get_hash();

        // Check if we have a distribution for this position
        let distribution = {
            let distributions = self.global_distributions.lock().unwrap();
            distributions.get(&board_hash).cloned()
        };

        match distribution {
            Some(dist) => {
                // Distribution exists - skip full evaluation, use weighted random
                let chosen_move = weighted_random_move(&dist);
                (chosen_move, 0) // Return 0 as score since we're not doing full eval
            }
            None => {
                // No distribution exists - create one, do full eval, then use weighted random

                // Create and store the distribution
                let new_distribution = create_move_distribution(&mut self.engine, &board);
                {
                    let mut distributions = self.global_distributions.lock().unwrap();
                    distributions.insert(board_hash, new_distribution.clone());
                }

                // Do full evaluation for training sample
                let (_, engine_score) = self.get_engine_move(&board);

                // Convert to white's perspective
                let white_score = if board.side_to_move() == chess::Color::White {
                    engine_score
                } else {
                    -engine_score
                };
                evaluations.push((board.to_string(), white_score));

                // Select move using weighted random
                let chosen_move = weighted_random_move(&new_distribution);
                (chosen_move, engine_score)
            }
        }
    }

    #[inline]
    fn get_engine_move(&mut self, board: &Board) -> (ChessMove, i16) {
        self.engine.set_position(*board);

        let params = GoParams {
            depth: Some(self.depth),
            ..Default::default()
        };

        self.engine.search(&params, None).unwrap()
    }

    fn should_abort_game(&self, score: &i16) -> bool {
        let num_moves: usize = self.positions_in_current_game.len();

        // safety net
        if num_moves > 500 {
            return true;
        }

        // Abort if moderately long and drawish
        if num_moves > 200 && score.abs() < 20 {
            return true;
        }

        false
    }

    #[inline]
    fn reset_game(&mut self) {
        self.game = Game::new();
        self.positions_in_current_game.clear();
        self.engine.new_game();
    }
}

// Perform shallow search for all legal moves to create evaluation distribution
#[inline]
fn create_move_distribution(engine: &mut NegamaxEngine, board: &Board) -> MoveDistribution {
    let moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
    let mut evaluations = Vec::with_capacity(moves.len());

    // Perform shallow search for each move
    const SHALLOW_DEPTH: u8 = 2;
    for &chess_move in &moves {
        let mut board_copy = *board;
        board_copy = board_copy.make_move_new(chess_move);

        engine.set_position(board_copy);
        let params = GoParams {
            depth: Some(SHALLOW_DEPTH),
            ..Default::default()
        };

        match engine.search(&params, None) {
            Some((_, eval)) => {
                evaluations.push(-eval);
            }
            None => {
                let eval = match board_copy.status() {
                    chess::BoardStatus::Checkmate => -29_000,
                    chess::BoardStatus::Stalemate => 0,
                    _ => 0,
                };
                evaluations.push(eval);
            }
        }
    }

    MoveDistribution::new(moves, evaluations)
}

// Select a move using weighted random based on evaluations
#[inline]
fn weighted_random_move(distribution: &MoveDistribution) -> ChessMove {
    if distribution.moves.is_empty() {
        panic!("No legal moves available");
    }

    let min_eval = *distribution.evaluations.iter().min().unwrap();
    let shift = if min_eval < 0 { -min_eval + 100 } else { 100 };

    let weights: Vec<u32> = distribution
        .evaluations
        .iter()
        .map(|&eval| (eval + shift).max(1) as u32) // Ensure positive weights
        .collect();

    match WeightedIndex::new(&weights) {
        Ok(dist) => {
            let mut rng = rand::thread_rng();
            let index = dist.sample(&mut rng);
            distribution.moves[index]
        }
        Err(_) => {
            // Fallback to random selection if weighted selection fails
            let mut rng = rand::thread_rng();
            let index = rng.gen_range(0..distribution.moves.len());
            distribution.moves[index]
        }
    }
}
