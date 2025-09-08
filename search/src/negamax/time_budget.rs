use chess::{Board, Color};
use uci::commands::GoParams;

use crate::negamax::utils::only_move;

// Time management constants
const DEFAULT_MOVES_LEFT: u64 = 20;
const INCREMENT_USAGE: f64 = 0.8;
const RESERVE_FRACTION: f64 = 0.08;
const MIN_RESERVE_MS: u64 = 300;
const OVERHEAD_MS: u64 = 20;
const MIN_TIME_PER_MOVE: u64 = 25;
const INITIAL_TARGET_FACTOR: f64 = 0.7; // Start with 70% of available time per move
const MAX_TARGET_FACTOR: f64 = 0.95; // Never use more than 95% of hard limit
const MIN_TARGET_FACTOR: f64 = 0.3; // Never go below 30% of hard limit

const ONLY_MOVE_TIME_MS: u64 = 100;

// Stockfish-style time management constants
const BEST_MOVE_STABILITY_BONUS: f64 = 0.8; // 20% time reduction when best move is stable
const BEST_MOVE_INSTABILITY_PENALTY: f64 = 1.4; // 40% time increase when best move changes
const SCORE_DROP_THRESHOLD: i16 = 50; // Significant score drop in centipawns
const SCORE_DROP_PENALTY: f64 = 1.3; // 30% time increase on score drop
const MIN_DEPTH_FOR_ADJUSTMENTS: u8 = 6; // Don't adjust before this depth
const EASY_MOVE_THRESHOLD: f64 = 0.6; // If same best move for 60% of iterations = easy

#[derive(Debug, Clone, Copy)]
pub struct TimeBudget {
    pub target: u64,
    pub hard: u64,
}

#[derive(Debug, Clone)]
pub struct SearchHistory {
    pub scores: Vec<i16>,
    pub depths: Vec<u8>,
    pub best_moves: Vec<Option<chess::ChessMove>>,
    pub aspiration_failures: u32,
}

impl SearchHistory {
    pub fn new() -> Self {
        Self {
            scores: Vec::new(),
            depths: Vec::new(),
            best_moves: Vec::new(),
            aspiration_failures: 0,
        }
    }

    pub fn add_iteration(&mut self, depth: u8, score: i16, best_move: Option<chess::ChessMove>) {
        self.depths.push(depth);
        self.scores.push(score);
        self.best_moves.push(best_move);
    }

    pub fn add_aspiration_failure(&mut self) {
        self.aspiration_failures += 1;
    }

    fn current_depth(&self) -> u8 {
        self.depths.last().copied().unwrap_or(0)
    }

    fn best_move_is_stable(&self) -> bool {
        if self.best_moves.len() < 4 {
            return false;
        }

        let recent_moves = &self.best_moves[self.best_moves.len() - 4..];
        let first_move = &recent_moves[0];

        // Count how many recent iterations have the same best move
        let same_move_count = recent_moves
            .iter()
            .filter(|&mv| mv == first_move && mv.is_some())
            .count();

        (same_move_count as f64) / (recent_moves.len() as f64) >= EASY_MOVE_THRESHOLD
    }

    fn best_move_changed_recently(&self) -> bool {
        if self.best_moves.len() < 2 {
            return false;
        }

        let last_two = &self.best_moves[self.best_moves.len() - 2..];
        last_two[0] != last_two[1] && last_two[0].is_some() && last_two[1].is_some()
    }

    fn has_score_drop(&self) -> bool {
        if self.scores.len() < 2 {
            return false;
        }

        let last_two = &self.scores[self.scores.len() - 2..];
        (last_two[0] - last_two[1]) >= SCORE_DROP_THRESHOLD
    }
}

impl TimeBudget {
    pub fn new(params: &GoParams, board: &Board) -> Option<Self> {
        // Time is provided, so let's use that as a hard limit
        if let Some(move_time) = params.move_time {
            return Some(Self {
                target: move_time,
                hard: move_time,
            });
        }

        // No need spending much time searching if there is only one legal move, but a little bit for score.
        if only_move(board) {
            return Some(Self {
                target: ONLY_MOVE_TIME_MS,
                hard: ONLY_MOVE_TIME_MS,
            });
        }

        let (time_left, increment) = Self::extract_time_params(params, board)?;
        let moves_left = params.moves_to_go.unwrap_or(DEFAULT_MOVES_LEFT);

        let reserve = ((time_left as f64) * RESERVE_FRACTION) as u64;
        let reserve = reserve.max(MIN_RESERVE_MS);
        let available = time_left
            .saturating_sub(reserve)
            .saturating_sub(OVERHEAD_MS);

        // Actual time the engine can afford per move (including increment)
        let base_time = (available as f64) / (moves_left as f64);
        let increment_bonus = (increment as f64) * INCREMENT_USAGE;
        let hard = ((base_time + increment_bonus) as u64).max(MIN_TIME_PER_MOVE);

        let target = ((hard as f64) * INITIAL_TARGET_FACTOR) as u64;

        Some(Self { target, hard })
    }

    // Stockfish-style time adjustment based on search behavior
    pub fn adjust_for_search_behavior(&mut self, search_history: &SearchHistory) {
        if search_history.current_depth() < MIN_DEPTH_FOR_ADJUSTMENTS {
            return;
        }

        let mut target_factor = INITIAL_TARGET_FACTOR;

        if search_history.best_move_is_stable() {
            target_factor *= BEST_MOVE_STABILITY_BONUS; // -20% time
        } else if search_history.best_move_changed_recently() {
            target_factor *= BEST_MOVE_INSTABILITY_PENALTY; // +40% time
        }

        if search_history.has_score_drop() {
            // Score has dropped, so verify
            target_factor *= SCORE_DROP_PENALTY; // +30% time
        }

        if search_history.aspiration_failures > 2 {
            // Position is complex, so verify
            target_factor *= 1.2; // +20% time
        }

        target_factor = target_factor.clamp(MIN_TARGET_FACTOR, MAX_TARGET_FACTOR);

        let new_target = ((self.hard as f64) * target_factor) as u64;
        self.target = new_target.max(MIN_TIME_PER_MOVE);
    }

    #[inline]
    fn extract_time_params(params: &GoParams, board: &Board) -> Option<(u64, u64)> {
        let side_to_move = board.side_to_move();
        let (time_left, increment) = match side_to_move {
            Color::White => (params.wtime?, params.winc.unwrap_or(0)),
            Color::Black => (params.btime?, params.binc.unwrap_or(0)),
        };
        Some((time_left, increment))
    }
}
