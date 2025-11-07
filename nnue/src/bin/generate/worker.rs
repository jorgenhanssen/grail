use crate::book::Book;
use crate::game::{check_draw, flush_game_to_evaluations, should_abort_game, GameEndReason};
use crate::histogram::HistogramHandle;
use chess::{Board, ChessMove, Game, MoveGen};
use evaluation::{hce, NNUE};
use rand::Rng;
use search::{Engine, EngineConfig};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use uci::commands::GoParams;

const WORKER_HASH_SIZE_MB: i32 = 384;
const INITIAL_TEMPERATURE: f32 = 3.0;
const TEMPERATURE_DECAY_RATE: f32 = 7.5;
const MIN_TEMPERATURE: f32 = 0.05;

pub struct SelfPlayWorker {
    _tid: usize,
    sample_counter: Arc<AtomicUsize>,
    game_id_counter: Arc<AtomicUsize>,
    engine: Engine,
    depth: u8,
    opening_book: Arc<Book>,
    histogram: HistogramHandle,

    // Game-specific state
    game: Game,
    game_id: usize,
    position_counts: HashMap<u64, usize>,
    current_game_positions: Vec<(String, i16)>,
    game_end_reason: Option<GameEndReason>,
}

impl SelfPlayWorker {
    pub fn new(
        tid: usize,
        sample_counter: Arc<AtomicUsize>,
        game_id_counter: Arc<AtomicUsize>,
        depth: u8,
        nnue: Option<Box<dyn NNUE>>,
        opening_book: Arc<Book>,
        histogram: HistogramHandle,
    ) -> Self {
        let mut config = EngineConfig::default();
        // Reduce hash size for data generation (384 MB instead of 1024 MB)
        // With 32 threads, this reduces RAM from 32GB to ~12GB
        config.hash_size.value = WORKER_HASH_SIZE_MB;

        let hce = Box::new(hce::Evaluator::new(
            config.get_piece_values(),
            config.get_hce_config(),
        ));

        Self {
            _tid: tid,
            sample_counter,
            game_id_counter,
            game: Game::new(),
            game_id: 0,
            depth,
            engine: Engine::new(&config, hce, nnue),
            position_counts: HashMap::new(),
            current_game_positions: Vec::new(),
            opening_book,
            histogram,
            game_end_reason: None,
        }
    }

    pub fn play_games(&mut self, stop_flag: Arc<AtomicBool>) -> Vec<(String, i16, usize)> {
        let mut evaluations = Vec::new();
        self.reset_game();

        while !stop_flag.load(Ordering::Relaxed) {
            if self.play_single_move() {
                flush_game_to_evaluations(
                    self.game_id,
                    &mut self.current_game_positions,
                    &mut evaluations,
                    &self.histogram,
                    &self.sample_counter,
                );
                self.reset_game();
            }
        }

        evaluations
    }

    fn play_single_move(&mut self) -> bool {
        let board = self.game.current_position();

        // Check if game should end (consolidated draw detection)
        if check_draw(
            &self.game,
            &board,
            &mut self.position_counts,
            &mut self.game_end_reason,
        ) {
            return true;
        }

        // Select move and get evaluation
        let (chosen_move, score) = self.select_move(board);

        // Check if game should be aborted (stable draw or mate detected)
        if should_abort_game(
            &score,
            &self.current_game_positions,
            &mut self.game_end_reason,
        ) {
            // If we're aborting due to mate score, remove the last position
            // since it won't be useful for training (will be filtered out anyway)
            if matches!(self.game_end_reason, Some(GameEndReason::MateScore)) {
                self.current_game_positions.pop();
            }
            return true;
        }

        self.game.make_move(chosen_move);

        false
    }

    fn select_move(&mut self, board: chess::Board) -> (ChessMove, i16) {
        let num_moves = self.current_game_positions.len();

        // Do single deep search to get best move + evaluation
        let (best_move, engine_score) = self.get_engine_move(&board);

        // Store position with evaluation (from white's perspective)
        let white_score = if board.side_to_move() == chess::Color::White {
            engine_score
        } else {
            -engine_score
        };
        self.current_game_positions
            .push((board.to_string(), white_score));

        // Apply temperature-based move selection
        // Use full turns (ply pairs) so both sides get equal temperature
        let full_turns = num_moves / 2;
        let chosen_move = self.select_move_with_temperature(&board, best_move, full_turns);

        (chosen_move, engine_score)
    }

    fn select_move_with_temperature(
        &mut self,
        board: &Board,
        best_move: ChessMove,
        full_turns: usize,
    ) -> ChessMove {
        let mut rng = rand::thread_rng();

        // Turn-based temperature decay (both White and Black get same temp per turn)
        // Formula: temp = INITIAL_TEMPERATURE * exp(-full_turns / TEMPERATURE_DECAY_RATE)
        // At turn 0: temp ≈ 3.0 (high randomness)
        // At turn 7-8: temp ≈ 1.1 (moderate randomness)
        // At turn 15: temp ≈ 0.40 (low randomness)
        // At turn 25: temp ≈ 0.10 (nearly optimal)
        // At turn 30+: temp < 0.05 (essentially optimal)
        let temperature =
            INITIAL_TEMPERATURE * (-(full_turns as f32) / TEMPERATURE_DECAY_RATE).exp();

        // With very low temperature, just play the best move
        if temperature < MIN_TEMPERATURE {
            return best_move;
        }

        // Generate all legal moves
        let legal_moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
        if legal_moves.len() == 1 {
            return legal_moves[0];
        }

        // Use random move probability: play random move with probability = temperature / INITIAL_TEMPERATURE
        // This ensures both sides have equal exploration
        let random_prob = (temperature / INITIAL_TEMPERATURE).min(1.0);

        if rng.gen::<f32>() < random_prob {
            // Pick a truly random legal move
            let index = rng.gen_range(0..legal_moves.len());
            legal_moves[index]
        } else {
            // Play the best move
            best_move
        }
    }

    #[inline]
    fn get_engine_move(&mut self, board: &Board) -> (ChessMove, i16) {
        let current_hash = board.get_hash();
        let history: ahash::AHashSet<u64> = self
            .position_counts
            .keys()
            .copied()
            .filter(|&hash| hash != current_hash)
            .collect();

        self.engine.set_position(*board, history);

        let params = GoParams {
            depth: Some(self.depth),
            ..Default::default()
        };

        self.engine.search(&params, None).unwrap()
    }

    #[inline]
    fn reset_game(&mut self) {
        // Get unique game ID
        self.game_id = self.game_id_counter.fetch_add(1, Ordering::Relaxed);

        // Get a random position from opening book
        let fen = self.opening_book.random_position();
        if let Ok(board) = Board::from_str(fen) {
            self.game = Game::new_with_board(board);
        } else {
            self.game = Game::new();
        }

        self.position_counts.clear();
        self.current_game_positions.clear();
        self.game_end_reason = None;
        self.engine.new_game();
    }
}
