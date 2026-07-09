use crate::limit::SearchLimit;
use crate::samples::GameOutcome;
use cozy_chess::{Board, Move};
use rand::{RngExt, rng};
use search::{Engine, SearchResult};
use std::collections::HashMap;
use utils::{flip_eval_perspective, has_check, has_insufficient_material, has_legal_moves};

#[derive(Clone, Copy)]
pub struct GameConfig {
    pub limit: SearchLimit,
    pub max_opening_imbalance: Option<i16>,
    pub max_teleport_plies: usize,
    pub max_teleport_pv_fraction: f64,
    pub max_game_plies: usize,
    pub dense_sampling: bool,
}
impl GameConfig {
    fn max_teleport_len(&self, pv_len: usize) -> usize {
        let pv_cap = ((pv_len as f64) * self.max_teleport_pv_fraction).floor() as usize;
        self.max_teleport_plies.min(pv_cap.max(1))
    }
}

/// A recorded position/sample (outcomes are labelled in the refinery).
pub struct Position {
    pub fen: String,
    pub score: i16,
    pub best_move: Move,
}

/// A self-play game that records positions for training.
///
/// Uses MultiPV search at decision points and teleports along chosen PV lines
/// to produce varied games from the same openings.
pub struct SelfPlayGame {
    board: Board,
    position_counts: HashMap<u64, usize>,
    positions: Vec<Position>,
    plies_played: usize,
    config: GameConfig,
}

impl SelfPlayGame {
    pub fn new(board: Board, config: GameConfig) -> Self {
        Self {
            board,
            position_counts: HashMap::new(),
            positions: Vec::new(),
            plies_played: 0,
            config,
        }
    }

    /// Play the game using MultiPV search and teleporting.
    ///
    /// At each decision point: search, record sample, select PV via softmax,
    /// then teleport along the chosen PV. With sparse sampling, only the
    /// anchor/decision nodes are searched and recorded.
    pub fn play(&mut self, engine: &mut Engine) {
        engine.new_game();

        let mut line: Vec<Move> = Vec::new();

        loop {
            // Cursed long game... no trustworthy outcome, discard the game.
            if self.plies_played >= self.config.max_game_plies {
                self.positions.clear();
                return;
            }

            if self.is_terminal() {
                break;
            }

            let is_anchor = line.is_empty();

            if is_anchor || self.config.dense_sampling {
                let Some(result) = self.search(engine) else {
                    break;
                };
                let Some(pv) = result.primary() else {
                    break;
                };

                if self.opening_is_too_imbalanced(pv.score) {
                    break;
                }

                if let Some(mv) = pv.best_move() {
                    self.record_position(pv.score, mv);
                }

                if is_anchor {
                    let chosen = result.select_softmax().expect("has lines");
                    let max_len = self.config.max_teleport_len(chosen.line.len());
                    let len = rng().random_range(1..=max_len);
                    line = chosen.line.iter().take(len).copied().collect();
                }
            }

            if line.is_empty() {
                break;
            }

            self.play_move(line.remove(0));
        }
    }

    fn search(&self, engine: &mut Engine) -> Option<SearchResult> {
        engine.set_position(self.board.clone(), Some(self.history()));
        engine.search(&self.config.limit.go_params(), None)
    }

    fn record_position(&mut self, eval: i16, best_move: Move) {
        let white_score = flip_eval_perspective(self.board.side_to_move(), eval);
        self.positions.push(Position {
            fen: format!("{}", self.board),
            score: white_score,
            best_move,
        });
    }

    fn opening_is_too_imbalanced(&self, score: i16) -> bool {
        let Some(max) = self.config.max_opening_imbalance else {
            return false;
        };
        self.positions.is_empty() && score.abs() > max
    }

    /// The game outcome. Returns None for speculative draws (50-move rule / repetition)
    pub fn outcome(&self) -> Option<GameOutcome> {
        if !has_legal_moves(&self.board) {
            return Some(if has_check(&self.board) {
                GameOutcome::win(!self.board.side_to_move())
            } else {
                GameOutcome::Draw
            });
        }

        if has_insufficient_material(&self.board) {
            return Some(GameOutcome::Draw);
        }

        // During some testing, I found that a winning game can still end in a draw in datagen.
        // Looked like it could shuffle won endgames into draws somehow.
        // Anyways, in these games it is better to have the tb define the outcome.
        None
    }

    /// Play a single move, updating all game state.
    fn play_move(&mut self, mv: Move) {
        self.board.play_unchecked(mv);
        self.plies_played += 1;

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
        if self.position_counts.get(&hash).copied().unwrap_or(0) >= 2 {
            return true;
        }
        false
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

    pub fn into_positions(self) -> Vec<Position> {
        self.positions
    }
}
