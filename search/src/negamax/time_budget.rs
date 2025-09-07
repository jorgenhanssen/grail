use crate::utils::game_phase;
use chess::{Board, Color, MoveGen};
use uci::commands::GoParams;

// Time management constants
const INCREMENT_USAGE: f64 = 0.8;
const RESERVE_FRACTION: f64 = 0.08;
const MIN_RESERVE_MS: u64 = 1000;
const OVERHEAD_MS: u64 = 20;
const HARD_MULTIPLIER: f64 = 2.5;
const MIN_TIME_PER_MOVE: u64 = 25;

// Soft linear scaling for estimated moves left: opening -> 60, endgame -> 10
const EST_MOVES_OPENING: f64 = 60.0;
const EST_MOVES_ENDGAME: f64 = 1.0;

#[derive(Debug, Clone, Copy)]
pub struct TimeBudget {
    pub target: u64,
    pub hard: u64,
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

        // If there is only one legal move, set the target time to 100ms
        let mut gen = MoveGen::new_legal(board);
        if gen.next().is_none() || gen.next().is_none() {
            return Some(Self {
                target: 100,
                hard: 100,
            });
        }

        let (time_left, increment) = Self::extract_time_params(params, board)?;
        let moves_left = params
            .moves_to_go
            .unwrap_or_else(|| Self::estimated_moves_left(board))
            .max(1);

        let reserve = ((time_left as f64) * RESERVE_FRACTION) as u64;
        let reserve = reserve.max(MIN_RESERVE_MS);
        let available = time_left
            .saturating_sub(reserve)
            .saturating_sub(OVERHEAD_MS);

        // Calculate target time per move
        let base_time = (available as f64) / (moves_left as f64);
        let increment_bonus = (increment as f64) * INCREMENT_USAGE;
        let target = ((base_time + increment_bonus) as u64).max(MIN_TIME_PER_MOVE);

        let hard = ((target as f64) * HARD_MULTIPLIER) as u64;

        Some(Self { target, hard })
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

    #[inline]
    fn estimated_moves_left(board: &Board) -> u64 {
        // Linear blend between opening and endgame estimate
        // phase in [0,1]: 1.0->opening, 0.0->endgame
        let p = game_phase(board) as f64;
        let est = EST_MOVES_ENDGAME + p * (EST_MOVES_OPENING - EST_MOVES_ENDGAME);
        est.round() as u64
    }
}
