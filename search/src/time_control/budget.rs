use cozy_chess::{Board, Color};
use uci::commands::GoParams;
use utils::only_move;

use super::stats::{TimeControlStats, MIN_DEPTH_FOR_ADJUSTMENTS};

// Time management constants
const MOVE_MARGIN_START: u64 = 20;
const MOVE_MARGIN_END: u64 = 10;
const INCREMENT_USAGE: f64 = 0.8;
const RESERVE_FRACTION: f64 = 0.08;
const MIN_RESERVE_MS: u64 = 300;
const OVERHEAD_MS: u64 = 20;
const MIN_TIME_PER_MOVE: u64 = 25;
const INITIAL_TARGET_FACTOR: f64 = 0.7; // Start with 70% of available time per move
const MAX_TARGET_FACTOR: f64 = 0.95; // Never use more than 95% of hard limit
const MIN_TARGET_FACTOR: f64 = 0.3; // Never go below 30% of hard limit

const ONLY_MOVE_TIME_MS: u64 = 100;

// Time adjustment factors based on search behavior
const BEST_MOVE_STABILITY_BONUS: f64 = 0.8; // -20% time when best move is stable
const BEST_MOVE_INSTABILITY_PENALTY: f64 = 1.4; // +40% time when best move changes
const SCORE_DROP_PENALTY: f64 = 1.3; // +30% time on score drop

#[derive(Debug, Clone, Copy)]
pub enum TimeBudget {
    /// Spend approximately this exact amount (e.g., UCI movetime, only move)
    Exact { millis: u64 },
    /// Managed time where target may be adjusted during search, capped by hard
    Managed { target: u64, hard: u64 },
}

impl TimeBudget {
    pub fn new(params: &GoParams, board: &Board) -> Option<Self> {
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

        // TODO: skip reserve when moves_to_go is set (time control refills after those moves)
        let reserve = ((time_left as f64) * RESERVE_FRACTION) as u64;
        let reserve = reserve.max(MIN_RESERVE_MS);
        let available = time_left
            .saturating_sub(reserve)
            .saturating_sub(OVERHEAD_MS);

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
    pub fn adjust_for_search_behavior(&mut self, stats: &TimeControlStats) {
        match self {
            TimeBudget::Exact { .. } => {
                // Do not adjust in exact mode
            }
            TimeBudget::Managed { target, hard } => {
                if stats.current_depth() < MIN_DEPTH_FOR_ADJUSTMENTS {
                    return;
                }

                let mut target_factor = INITIAL_TARGET_FACTOR;

                if stats.best_move_is_stable() {
                    target_factor *= BEST_MOVE_STABILITY_BONUS; // -20% time
                } else if stats.best_move_changed_recently() {
                    target_factor *= BEST_MOVE_INSTABILITY_PENALTY; // +40% time
                }

                if stats.has_score_drop() {
                    // Score has dropped, so verify
                    target_factor *= SCORE_DROP_PENALTY; // +30% time
                }

                if stats.aspiration_failures > 2 {
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

fn move_margin(board: &Board) -> u64 {
    const TOTAL_PIECES: f32 = 32.0;

    let num_pieces = board.occupied().len() as f32;
    let phase = num_pieces / TOTAL_PIECES;

    (phase * MOVE_MARGIN_START as f32 + (1.0 - phase) * MOVE_MARGIN_END as f32) as u64
}
