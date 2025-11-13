use chess::{Board, ChessMove, Game, MoveGen};
use rand::Rng;
use search::Engine;
use std::collections::HashMap;
use std::str::FromStr;
use uci::commands::GoParams;
use utils::has_insufficient_material;

const INITIAL_TEMPERATURE: f32 = 3.0;
const TEMPERATURE_DECAY_RATE: f32 = 7.5;
const MIN_TEMPERATURE: f32 = 0.05;
const MATE_THRESHOLD: i16 = 5000;

pub struct SelfPlayGame {
    game: Game,
    game_id: usize,
    ply_count: usize,
    position_counts: HashMap<u64, usize>,
    current_game_samples: Vec<(String, i16)>,
}

impl SelfPlayGame {
    pub fn new(game_id: usize, opening_fen: &str) -> Self {
        let game = if let Ok(board) = Board::from_str(opening_fen) {
            Game::new_with_board(board)
        } else {
            Game::new()
        };

        Self {
            game,
            game_id,
            ply_count: 0,
            position_counts: HashMap::new(),
            current_game_samples: Vec::new(),
        }
    }

    pub fn play(&mut self, engine: &mut Engine, depth: u8) {
        engine.new_game();

        loop {
            if self.is_terminal() {
                break;
            }

            let (best_move, eval) = self.compute_move(engine, depth);

            // Skip near-mate positions
            // Testing showed this improves strength (by freeing capacity for nuanced positions, I guess)
            if eval.abs() >= MATE_THRESHOLD {
                break;
            }

            self.record_eval(eval);
            self.make_move(best_move);
        }
    }

    fn compute_move(&self, engine: &mut Engine, depth: u8) -> (ChessMove, i16) {
        let board = self.current_position();
        let history = self.history();

        engine.set_position(board, history);

        let params = GoParams {
            depth: Some(depth),
            ..Default::default()
        };

        engine.search(&params, None).unwrap()
    }

    fn is_terminal(&mut self) -> bool {
        // 1. Check chess rules (checkmate, stalemate, draw acceptance, etc.)
        if self.game.result().is_some() {
            return true;
        }

        // 2. Check insufficient material (K vs K, K+B vs K, K+N vs K, etc.)
        let board = self.current_position();
        if has_insufficient_material(&board) {
            return true;
        }

        // 3. Check position repetition (abort on first repetition)
        // For training data, we don't need official three-fold rule -
        // any repetition means the game is cycling and won't produce useful data
        let board_hash = board.get_hash();
        *self.position_counts.entry(board_hash).or_insert(0) += 1;
        if self.position_counts[&board_hash] >= 2 {
            return true;
        }

        false
    }

    fn current_position(&self) -> Board {
        self.game.current_position()
    }

    fn record_eval(&mut self, engine_score: i16) {
        let board = self.current_position();

        let white_score = if board.side_to_move() == chess::Color::White {
            engine_score
        } else {
            -engine_score
        };

        self.current_game_samples
            .push((board.to_string(), white_score));
    }

    fn select_move(&self, best_move: ChessMove) -> ChessMove {
        let mut rng = rand::thread_rng();

        // Temperature decays based on full move number (not ply)
        // This ensures both White and Black get equal exploration at each turn
        let move_number = self.ply_count / 2;
        let temp = INITIAL_TEMPERATURE * (-(move_number as f32) / TEMPERATURE_DECAY_RATE).exp();

        if temp < MIN_TEMPERATURE {
            return best_move;
        }

        let board = self.current_position();
        let legal_moves: Vec<ChessMove> = MoveGen::new_legal(&board).collect();

        if legal_moves.len() == 1 {
            return legal_moves[0];
        }

        let probability_of_random_move = (temp / INITIAL_TEMPERATURE).min(1.0);

        if rng.gen::<f32>() < probability_of_random_move {
            let index = rng.gen_range(0..legal_moves.len());
            legal_moves[index]
        } else {
            best_move
        }
    }

    fn make_move(&mut self, best_move: ChessMove) {
        let chosen_move = self.select_move(best_move);
        self.game.make_move(chosen_move);
        self.ply_count += 1;
    }

    fn history(&self) -> ahash::AHashSet<u64> {
        let current_hash = self.current_position().get_hash();
        self.position_counts
            .keys()
            .copied()
            .filter(|&hash| hash != current_hash)
            .collect()
    }

    pub fn drain_samples(&mut self) -> (Vec<(String, i16, usize)>, Vec<i16>) {
        let (samples, scores): (Vec<_>, Vec<_>) = self
            .current_game_samples
            .drain(..)
            .map(|(fen, score)| ((fen, score, self.game_id), score))
            .unzip();
        (samples, scores)
    }
}
