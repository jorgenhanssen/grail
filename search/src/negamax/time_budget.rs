use chess::Color;
use uci::commands::GoParams;

// Time management constants
const DEFAULT_MOVES_LEFT: u64 = 25;
const INCREMENT_USAGE: f64 = 0.8;
const RESERVE_FRACTION: f64 = 0.08;
const MIN_RESERVE_MS: u64 = 300;
const OVERHEAD_MS: u64 = 20;
const HARD_MULTIPLIER: f64 = 2.5;
const MIN_TIME_PER_MOVE: u64 = 25;

#[derive(Debug, Clone, Copy)]
pub struct TimeBudget {
    pub target: u64,
    pub hard: u64,
}

impl TimeBudget {
    pub fn new(params: &GoParams, side_to_move: Color) -> Option<Self> {
        // Time is provided, so let's use that as a hard limit
        if let Some(move_time) = params.move_time {
            return Some(Self {
                target: move_time,
                hard: move_time,
            });
        }

        let (time_left, increment) = Self::extract_time_params(params, side_to_move)?;
        let moves_left = params.moves_to_go.unwrap_or(DEFAULT_MOVES_LEFT).max(1);

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
    fn extract_time_params(params: &GoParams, side_to_move: Color) -> Option<(u64, u64)> {
        let (time_left, increment) = match side_to_move {
            Color::White => (params.wtime?, params.winc.unwrap_or(0)),
            Color::Black => (params.btime?, params.binc.unwrap_or(0)),
        };
        Some((time_left, increment))
    }
}
