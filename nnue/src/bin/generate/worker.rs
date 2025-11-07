use crate::book::Book;
use crate::game::game_is_terminal;
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
const MATE_THRESHOLD: i16 = 5000;
const STABLE_DRAW_MOVES: usize = 40;
const DRAWISH_EVAL: i16 = 20;

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

        // Reduced hash size to reduce RAM usage
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
        }
    }

    pub fn play_games(&mut self, stop_flag: Arc<AtomicBool>) -> Vec<(String, i16, usize)> {
        let mut evaluations = Vec::new();

        while !stop_flag.load(Ordering::Relaxed) {
            self.play_game(&mut evaluations);
        }

        evaluations
    }

    fn play_game(&mut self, evaluations: &mut Vec<(String, i16, usize)>) {
        self.init_game();

        loop {
            if self.is_game_over() {
                break;
            }

            let (best_move, eval) = self.compute_move();

            // Skip near-mate positions
            // Testing showed this improves strength (by freeing capacity for nuanced positions, I guess)
            if eval.abs() >= MATE_THRESHOLD {
                break;
            }
            if self.is_stable_draw() {
                break;
            }

            self.record_sample(eval);

            // Choose and execute move (with temperature-based exploration)
            let chosen_move = self.select_move(best_move);
            self.game.make_move(chosen_move);
        }

        self.flush_game(evaluations);
    }

    // Checks terminal by chess rules / repetition
    fn is_game_over(&mut self) -> bool {
        let board = self.game.current_position();
        game_is_terminal(&self.game, &board, &mut self.position_counts)
    }

    // Checks stable draw condition (40+ moves near zero eval)
    fn is_stable_draw(&self) -> bool {
        if self.current_game_positions.len() < STABLE_DRAW_MOVES {
            return false;
        }
        let start = self.current_game_positions.len() - STABLE_DRAW_MOVES;
        self.current_game_positions[start..]
            .iter()
            .all(|(_, eval)| eval.abs() < DRAWISH_EVAL)
    }

    // Runs engine to get best move + score
    fn compute_move(&mut self) -> (ChessMove, i16) {
        let board = self.game.current_position();
        self.get_engine_best_move(&board)
    }

    // Records training sample (position + eval from white's perspective)
    fn record_sample(&mut self, engine_score: i16) {
        let board = self.game.current_position();
        let white_score = if board.side_to_move() == chess::Color::White {
            engine_score
        } else {
            -engine_score
        };
        self.current_game_positions
            .push((board.to_string(), white_score));
    }

    // Flushes completed game to output
    fn flush_game(&mut self, evaluations: &mut Vec<(String, i16, usize)>) {
        let (positions, scores): (Vec<_>, Vec<_>) = self
            .current_game_positions
            .drain(..)
            .map(|(fen, score)| ((fen, score, self.game_id), score))
            .unzip();

        let num_positions = positions.len();
        evaluations.extend(positions);
        self.histogram.record_scores(&scores);
        self.sample_counter
            .fetch_add(num_positions, Ordering::Relaxed);
    }

    // Selects move with temperature-based exploration (decays over game turns)
    fn select_move(&mut self, best_move: ChessMove) -> ChessMove {
        let mut rng = rand::thread_rng();

        let current_move = self.current_game_positions.len() / 2;

        // Temperature is used to balance exploration and exploitation
        // Temperature is higher in the beginning of the game and decays over time.
        // = More random moves in the beginning of the game => optimal play near the end.
        // Scaled by move and not turn to ensure both sides have equal exploration (else first player would be more random = worse play)
        let temp = INITIAL_TEMPERATURE * (-(current_move as f32) / TEMPERATURE_DECAY_RATE).exp();

        // With very low temperature, just play the best move
        if temp < MIN_TEMPERATURE {
            return best_move;
        }

        let board = self.game.current_position();

        let legal_moves: Vec<ChessMove> = MoveGen::new_legal(&board).collect();
        if legal_moves.len() == 1 {
            return legal_moves[0];
        }

        let probability_of_random_move = (temp / INITIAL_TEMPERATURE).min(1.0);

        if rng.gen::<f32>() < probability_of_random_move {
            // Pick a truly random legal move
            let index = rng.gen_range(0..legal_moves.len());
            legal_moves[index]
        } else {
            // Play the best move
            best_move
        }
    }

    #[inline]
    fn get_engine_best_move(&mut self, board: &Board) -> (ChessMove, i16) {
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

    fn init_game(&mut self) {
        self.game_id = self.game_id_counter.fetch_add(1, Ordering::Relaxed);

        let fen = self.opening_book.random_position();
        self.game = if let Ok(board) = Board::from_str(fen) {
            Game::new_with_board(board)
        } else {
            Game::new()
        };

        self.position_counts.clear();
        self.current_game_positions.clear();
        self.engine.new_game();
    }
}
