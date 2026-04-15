use crate::samples::{GameOutcome, Sample};
use cozy_chess::{Board, Color, Move};
use rand::Rng;
use search::{Engine, PvLine, SearchResult};
use std::collections::HashMap;
use std::str::FromStr;
use uci::commands::GoParams;
use utils::{flip_eval_perspective, has_check, has_insufficient_material, has_legal_moves};

struct RecordedPosition {
    fen: String,
    score: i16,
    best_move: Move,
    ply: u16,
}

/// A self-play game that generates training samples.
///
/// Uses MultiPV search at decision points and teleports along chosen PV lines
/// to reduce sample correlation and increase game diversity.
pub struct SelfPlayGame {
    board: Board,
    game_id: usize,
    position_counts: HashMap<u64, usize>,
    positions: Vec<RecordedPosition>,
    ply: u16,
    depth: u8,
}

impl SelfPlayGame {
    pub fn new(game_id: usize, opening_fen: &str, depth: u8) -> Self {
        let board = Board::from_str(opening_fen).unwrap();

        Self {
            board,
            game_id,
            position_counts: HashMap::new(),
            positions: Vec::new(),
            ply: 0,
            depth,
        }
    }

    /// Play the game using MultiPV search and teleporting.
    ///
    /// At each decision point: search, record sample, select PV via softmax,
    /// then teleport along the chosen PV.
    pub fn play(&mut self, engine: &mut Engine) {
        engine.new_game();

        loop {
            if self.is_terminal() {
                break;
            }

            let result = match self.search(engine) {
                Some(r) => r,
                None => break,
            };

            let pv = match result.primary() {
                Some(pv) => pv,
                None => break,
            };

            if let Some(mv) = pv.best_move() {
                self.record_position(pv.score, mv);
            }

            let chosen_pv = result.select_softmax().expect("has lines");
            self.teleport(chosen_pv);
        }
    }

    fn search(&self, engine: &mut Engine) -> Option<SearchResult> {
        engine.set_position(self.board.clone(), Some(self.history()));

        let params = GoParams {
            depth: Some(self.depth),
            ..Default::default()
        };

        engine.search(&params, None)
    }

    fn record_position(&mut self, eval: i16, best_move: Move) {
        let white_score = flip_eval_perspective(self.board.side_to_move(), eval);
        self.positions.push(RecordedPosition {
            fen: format!("{}", self.board),
            score: white_score,
            best_move,
            ply: self.ply,
        });
    }

    fn outcome(&self) -> GameOutcome {
        if !has_legal_moves(&self.board) && has_check(&self.board) {
            return if self.board.side_to_move() == Color::White {
                GameOutcome::Black
            } else {
                GameOutcome::White
            };
        }

        GameOutcome::Draw
    }

    /// Teleport along a PV line by playing moves without searching.
    ///
    /// Picks a random teleport length from 1 to pv.len() (capped by depth),
    /// then plays that many moves from the PV.
    fn teleport(&mut self, pv: &PvLine) {
        if pv.line.is_empty() {
            return;
        }

        let mut rng = rand::rng();
        let max_len = pv.line.len().min(self.depth as usize);
        let teleport_len = rng.random_range(1..=max_len);

        for mv in pv.line.iter().take(teleport_len) {
            self.play_move(*mv);
            if self.is_terminal() {
                break;
            }
        }
    }

    /// Play a single move, updating all game state.
    fn play_move(&mut self, mv: Move) {
        self.board.play_unchecked(mv);
        self.ply += 1;

        // Track position for repetition detection
        let hash = self.board.hash();
        *self.position_counts.entry(hash).or_insert(0) += 1;
    }

    fn is_terminal(&self) -> bool {
        if !has_legal_moves(&self.board) {
            return true;
        }
        if has_insufficient_material(&self.board) {
            return true;
        }
        if self.board.halfmove_clock() >= 100 {
            return true;
        }
        // Any repetition ends game for training purposes
        let hash = self.board.hash();
        self.position_counts.get(&hash).copied().unwrap_or(0) >= 2
    }

    /// Get position history for repetition detection during search.
    fn history(&self) -> ahash::AHashSet<u64> {
        let current_hash = self.board.hash();
        self.position_counts
            .keys()
            .copied()
            .filter(|&hash| hash != current_hash)
            .collect()
    }

    pub fn game_length(&self) -> u16 {
        self.ply
    }

    pub fn get_samples(&mut self) -> (Vec<Sample>, Vec<i16>) {
        let outcome = self.outcome();
        let end_ply = self.ply;
        let (samples, scores): (Vec<_>, Vec<_>) = self
            .positions
            .drain(..)
            .map(|pos| {
                let distance_to_end = end_ply.saturating_sub(pos.ply);
                let sample = Sample {
                    fen: pos.fen,
                    score: pos.score,
                    game_id: self.game_id,
                    best_move: pos.best_move,
                    outcome,
                    distance_to_end,
                };
                (sample, pos.score)
            })
            .unzip();
        (samples, scores)
    }
}
