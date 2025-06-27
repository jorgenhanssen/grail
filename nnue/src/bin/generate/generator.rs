use std::error::Error;
use std::time::Instant;
use std::{path::PathBuf, sync::Arc};

use ahash::{AHashMap, AHashSet};
use candle_core::Device;
use candle_nn::VarMap;
use chess::{Board, ChessMove, Game, MoveGen};
use evaluation::{Evaluator, TraditionalEvaluator};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use nnue::version::VersionManager;
use nnue::NNUE;
use rand::Rng;
use search::{Engine, NegamaxEngine};
use std::sync::Mutex;

struct SharedData {
    // Map of position hash to move evaluations (for move selection)
    move_evals: AHashMap<u64, Vec<(ChessMove, f32)>>,

    // Map of board FEN to position score (for training data)
    position_scores: AHashMap<String, f32>,
}
impl SharedData {
    fn new() -> Self {
        Self {
            move_evals: AHashMap::new(),
            position_scores: AHashMap::new(),
        }
    }
}

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

    pub fn run(&self, duration: u64, depth: u64) -> Vec<(String, f32)> {
        let eval_name = match &self.nnue_path {
            Some(path) => path.display().to_string(),
            None => "traditional evaluator".to_string(),
        };

        log::info!(
            "Generating samples using {} threads ({})",
            self.threads,
            eval_name
        );

        // Create a single shared progress bar instead of multiple ones
        let pb = ProgressBar::new(duration);
        pb.set_style(
            ProgressStyle::with_template(
                " {spinner:.cyan} {wide_bar:.cyan/blue} {eta_precise} | {msg}",
            )
            .unwrap(),
        );

        // Wrap in Arc to share across threads
        let pb = Arc::new(pb);

        let shared_data = Arc::new(Mutex::new(SharedData::new()));

        let handles: Vec<_> = (0..self.threads)
            .map(|tid| {
                let nnue_path = self.nnue_path.clone();
                let version = self.version;
                let shared_data = Arc::clone(&shared_data);
                let pb = Arc::clone(&pb); // Share the same progress bar

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

                    let mut worker = SelfPlayWorker::new(tid, shared_data, depth, evaluator);
                    worker.play_games(duration, &pb)
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let shared_data = shared_data.lock().unwrap();
        let position_count = shared_data.position_scores.len();

        // Update the progress bar one final time
        pb.finish_with_message(format!("Evaluated {} positions", position_count));

        // Convert the position scores to the expected output format
        let evaluations: Vec<(String, f32)> = shared_data
            .position_scores
            .iter()
            .map(|(fen, score)| (fen.clone(), *score))
            .collect();

        evaluations
    }
}

struct SelfPlayWorker {
    tid: usize,
    shared_data: Arc<Mutex<SharedData>>,
    engine: NegamaxEngine,
    depth: u64,

    // Specific to ongoing game
    game: Game,
    positions_in_current_game: AHashSet<u64>,

    // Configuration for move selection
    temperature: f32,
    pure_random_chance: f32,
}

impl SelfPlayWorker {
    pub fn new(
        tid: usize,
        shared_data: Arc<Mutex<SharedData>>,
        depth: u64,
        evaluator: Box<dyn Evaluator>,
    ) -> Self {
        Self {
            tid,
            shared_data,
            game: Game::new(),
            depth,

            engine: NegamaxEngine::new(evaluator),
            positions_in_current_game: AHashSet::new(),
            temperature: 1.5, // higher = more random
            pure_random_chance: 0.15,
        }
    }

    pub fn play_games(&mut self, duration: u64, pb: &Arc<ProgressBar>) {
        let start_time = Instant::now();
        self.reset_game();

        loop {
            let current_elapsed = start_time.elapsed().as_secs();
            if current_elapsed >= duration {
                break;
            }

            // Update progress with position count
            let position_count = {
                let data = self.shared_data.lock().unwrap();
                data.position_scores.len()
            };

            if self.tid == 0 {
                pb.set_message(format!("{} positions", position_count));
                pb.set_position(current_elapsed);
            }

            let terminal = self.play_single_move();

            if terminal {
                self.reset_game();
            }
        }
    }

    fn play_single_move(&mut self) -> bool {
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
        let (chosen_move, score) = self.select_move(board);

        if self.should_abort_game(&score) {
            return true;
        }

        self.game.make_move(chosen_move);

        false
    }

    fn select_move(&mut self, board: chess::Board) -> (ChessMove, f32) {
        let moves: Vec<ChessMove> = MoveGen::new_legal(&board).collect();
        if moves.is_empty() {
            return (ChessMove::default(), 0.0);
        }

        let board_hash = board.get_hash();
        let fen = board.to_string();

        // First check if position has been globally evaluated
        if let Some(cached_evals) = self.shared_data.lock().unwrap().move_evals.get(&board_hash) {
            // If position is already in cache, we can use it directly
            return self.pick_move(cached_evals);
        }

        // Position has not been evaluated yet
        let (_, engine_score, moves_with_scores) = self.get_engine_move(&board);

        // When using negamax, the engine_score is already from the current player's perspective
        // For training data, we want scores from white's perspective
        // If black is to move, we need to negate the score for the training data
        let white_score = if board.side_to_move() == chess::Color::Black {
            -engine_score
        } else {
            engine_score
        };

        // Apply tanh to normalize scores
        let normalized_score = white_score.tanh();

        // Update shared data
        {
            let mut data = self.shared_data.lock().unwrap();

            // Add move evaluations if not already present
            if !data.move_evals.contains_key(&board_hash) {
                data.move_evals
                    .insert(board_hash, moves_with_scores.clone());
            }

            // Add position score if not already present
            if !data.position_scores.contains_key(&fen) {
                data.position_scores.insert(fen, normalized_score);
            }
        }

        self.pick_move(&moves_with_scores)
    }

    // Helper function to pick a move based on temperature or random chance
    fn pick_move(&self, moves_with_scores: &Vec<(ChessMove, f32)>) -> (ChessMove, f32) {
        // Get current board FEN
        let fen = self.game.current_position().to_string();

        // Decide if we should use pure random selection
        if rand::thread_rng().gen::<f32>() < self.pure_random_chance {
            let idx = rand::thread_rng().gen_range(0..moves_with_scores.len());
            let (selected_move, score) = moves_with_scores[idx].clone();
            println!(
                "[Worker {}] Position: {} | Random move selected: {} (score: {})",
                self.tid, fen, selected_move, score
            );
            return (selected_move, score);
        }

        // Otherwise, select move based on temperature
        self.select_move_by_temperature(moves_with_scores, &fen)
    }

    fn select_move_by_temperature(
        &self,
        move_scores: &Vec<(ChessMove, f32)>,
        fen: &str,
    ) -> (ChessMove, f32) {
        // Handle empty or singleton move lists
        if move_scores.is_empty() {
            return (ChessMove::default(), 0.0);
        }
        if move_scores.len() == 1 {
            let (mv, score) = move_scores[0].clone();
            println!(
                "[Worker {}] Position: {} | Only one move available: {} (score: {})",
                self.tid, fen, mv, score
            );
            return (mv, score);
        }

        // Calculate softmax probabilities
        let mut sum_exp = 0.0;
        let mut probabilities = Vec::with_capacity(move_scores.len());

        // Important: With negamax, scores are already from the current player's perspective
        // Higher scores are always better regardless of who is to move

        // Find max score for numerical stability
        let max_score = move_scores
            .iter()
            .map(|(_, s)| *s)
            .fold(f32::NEG_INFINITY, f32::max);

        for (_, score) in move_scores {
            // No need to adjust for black - negamax already did that
            let exp_value = ((*score - max_score) / self.temperature).exp();
            sum_exp += exp_value;
            probabilities.push(exp_value);
        }

        // Normalize probabilities
        for prob in &mut probabilities {
            *prob /= sum_exp;
        }

        // Select a move based on the calculated probabilities
        let mut rng = rand::thread_rng();
        let sample = rng.gen::<f32>();
        let mut cumulative_prob = 0.0;

        println!(
            "[Worker {}] Position: {} | Temperature: {}, Move options:",
            self.tid, fen, self.temperature
        );
        for (i, ((mv, score), prob)) in move_scores.iter().zip(probabilities.iter()).enumerate() {
            println!("  [{}] {} (score: {}, prob: {:.4})", i, mv, score, prob);
        }

        for (i, prob) in probabilities.iter().enumerate() {
            cumulative_prob += prob;
            if sample < cumulative_prob {
                let (selected_move, score) = move_scores[i].clone();
                println!("[Worker {}] Position: {} | Temperature-based move selected: {} (score: {}, prob: {:.4}, sample: {:.4})", 
                    self.tid, fen, selected_move, score, prob, sample);
                return (selected_move, score);
            }
        }

        // Should rarely happen due to floating point precision issues
        let (selected_move, score) = move_scores.last().unwrap().clone();
        println!(
            "[Worker {}] Position: {} | Fallback move selected: {} (score: {})",
            self.tid, fen, selected_move, score
        );
        move_scores.last().unwrap().clone()
    }

    #[inline]
    fn get_engine_move(&mut self, board: &Board) -> (ChessMove, f32, Vec<(ChessMove, f32)>) {
        self.engine.set_position(*board);
        self.engine.init_search();
        self.engine.search_root(self.depth)
    }

    fn should_abort_game(&self, score: &f32) -> bool {
        let num_moves = self.positions_in_current_game.len();

        // Safety net - avoid extremely long games
        if num_moves > 500 {
            return true;
        }

        // Long drawish game
        if num_moves > 200 && score.abs() < 0.1 {
            return true;
        }

        // Very long slightly imbalanced game
        if num_moves > 300 && score.abs() < 0.5 {
            return true;
        }

        false
    }
    #[inline]
    fn reset_game(&mut self) {
        self.game = Game::new();
        self.positions_in_current_game.clear();
    }
}
