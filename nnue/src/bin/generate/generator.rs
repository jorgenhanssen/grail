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
use rand::Rng;
use search::{Engine, NegamaxEngine};
use std::sync::Mutex;
use uci::commands::GoParams;

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

        let global_evaluated = Arc::new(Mutex::new(AHashSet::new()));

        let handles: Vec<_> = (0..self.threads)
            .map(|tid| {
                let nnue_path = self.nnue_path.clone();
                let version = self.version;
                let global_evaluated = Arc::clone(&global_evaluated);
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

                    let mut worker = SelfPlayWorker::new(tid, global_evaluated, depth, evaluator);
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
    global_evaluated: Arc<Mutex<AHashSet<u64>>>,
    engine: NegamaxEngine,
    depth: u8,

    // Specific to ongoing game
    game: Game,
    positions_in_current_game: AHashSet<u64>,
}

impl SelfPlayWorker {
    pub fn new(
        tid: usize,
        global_evaluated: Arc<Mutex<AHashSet<u64>>>,
        depth: u8,
        evaluator: Box<dyn Evaluator>,
    ) -> Self {
        Self {
            tid,
            global_evaluated,
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
                let global_count = self.global_evaluated.lock().unwrap().len();
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
        let moves: Vec<ChessMove> = MoveGen::new_legal(&board).collect();

        if self.position_has_been_evaluated(&board) {
            return (random_move(&moves), 0);
        }

        let (engine_move, engine_score) = self.get_engine_move(&board);

        // Convert to white's perspective and tanh to force mate scores to be in [-1, 1]
        let white_score = if board.side_to_move() == chess::Color::White {
            engine_score
        } else {
            -engine_score
        };
        evaluations.push((board.to_string(), white_score));

        if should_use_engine_move() {
            (engine_move, engine_score)
        } else {
            (random_move(&moves), engine_score)
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

#[inline]
fn should_use_engine_move() -> bool {
    rand::thread_rng().gen::<f32>() < 0.3
}

#[inline]
fn random_move(moves: &[ChessMove]) -> ChessMove {
    let idx = rand::thread_rng().gen_range(0..moves.len());
    moves[idx]
}
