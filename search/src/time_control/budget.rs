//! Time budget management for chess search.
//!
//! Uses a two-tier system:
//! - **target**: soft limit, aim to stop here (can be adjusted during search)
//! - **hard**: absolute maximum, never exceed

use cozy_chess::{Board, Color, Move};
use uci::commands::GoParams;

use utils::only_move;

// Time management constants
// Estimated moves remaining - intentionally conservative since the target
// often gets reduced when the best move is stable (see adjust_for_search_behavior)
const MOVE_MARGIN_START: u64 = 20;
const MOVE_MARGIN_END: u64 = 10;
const INCREMENT_USAGE: f64 = 0.8;
const RESERVE_FRACTION: f64 = 0.08;
const MIN_RESERVE_MS: u64 = 300;
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
pub enum TimeBudget {
    // Spend approximately this exact amount (e.g., UCI movetime, only move)
    Exact { millis: u64 },
    // Managed time where target may be adjusted during search, capped by hard
    Managed { target: u64, hard: u64 },
}

#[derive(Debug, Clone)]
pub struct SearchHistory {
    pub scores: Vec<i16>,
    pub depths: Vec<u8>,
    pub best_moves: Vec<Option<Move>>,
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

    pub fn add_iteration(&mut self, depth: u8, score: i16, best_move: Option<Move>) {
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
    pub fn new(params: &GoParams, board: &Board, move_overhead_ms: u64) -> Option<Self> {
        // UCI movetime: spend exactly this exact amount
        if let Some(move_time) = params.move_time {
            return Some(Self::Exact { millis: move_time });
        }

        // If a depth is set then we do not want this to be timed.
        if params.depth.is_some() {
            return None;
        }

        // No need spending much time searching if there is only one legal move, but a little bit for score.
        if only_move(board) {
            return Some(Self::Exact {
                millis: ONLY_MOVE_TIME_MS,
            });
        }

        let side_to_move = board.side_to_move();

        let time_left = get_time_left(params, side_to_move)?;
        let increment = get_increment(params, side_to_move);
        let opponent_time = get_time_left(params, !side_to_move);

        let moves_left = params.moves_to_go.unwrap_or(move_margin(board));

        let reserve = if params.moves_to_go.is_some() {
            // Skip reserving time when we know how many moves to the next refill.
            0
        } else {
            // Reserve time as safety buffer since we don't know when the game ends.
            let r = ((time_left as f64) * RESERVE_FRACTION) as u64;
            r.max(MIN_RESERVE_MS)
        };
        let available = time_left
            .saturating_sub(reserve)
            .saturating_sub(move_overhead_ms);

        // If we have more time than opponent, we can afford to spend a bit more
        let time_advantage = opponent_time
            .map(|opp| time_left.saturating_sub(opp))
            .unwrap_or(0);

        let total_available = available.saturating_add(time_advantage);

        // Actual time the engine can afford per move (including increment and advantage)
        let base_time = (total_available as f64) / (moves_left as f64);
        let increment_bonus = (increment as f64) * INCREMENT_USAGE;
        let hard = ((base_time + increment_bonus) as u64).max(MIN_TIME_PER_MOVE);

        let target = ((hard as f64) * INITIAL_TARGET_FACTOR) as u64;

        Some(Self::Managed { target, hard })
    }

    pub fn hard_limit(&self) -> u64 {
        match *self {
            TimeBudget::Exact { millis } => millis,
            TimeBudget::Managed { hard, .. } => hard,
        }
    }

    pub fn target_limit(&self) -> u64 {
        match *self {
            TimeBudget::Exact { millis } => millis,
            TimeBudget::Managed { target, .. } => target,
        }
    }

    // Stockfish-style time adjustment based on search behavior (Managed only)
    pub fn adjust_for_search_behavior(&mut self, search_history: &SearchHistory) {
        match self {
            TimeBudget::Exact { .. } => {
                // Do not adjust in exact mode
            }
            TimeBudget::Managed { target, hard } => {
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

                let new_target = ((*hard as f64) * target_factor) as u64;
                *target = new_target.max(MIN_TIME_PER_MOVE);
            }
        }
    }
}

#[inline]
fn get_time_left(params: &GoParams, color: Color) -> Option<u64> {
    match color {
        Color::White => params.wtime,
        Color::Black => params.btime,
    }
}

#[inline]
fn get_increment(params: &GoParams, color: Color) -> u64 {
    match color {
        Color::White => params.winc.unwrap_or(0),
        Color::Black => params.binc.unwrap_or(0),
    }
}

/// Estimates moves remaining based on game phase.
/// More pieces = earlier in game = more moves expected.
fn move_margin(board: &Board) -> u64 {
    const TOTAL_PIECES: f32 = 32.0;

    let num_pieces = board.occupied().len() as f32;
    let phase = num_pieces / TOTAL_PIECES;

    (phase * MOVE_MARGIN_START as f32 + (1.0 - phase) * MOVE_MARGIN_END as f32) as u64
}
