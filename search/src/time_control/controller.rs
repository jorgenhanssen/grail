use std::time::Instant;

use cozy_chess::{Board, Move};
use uci::commands::GoParams;

use crate::time_control::budget::TimeBudget;
use crate::time_control::stats::TimeControlStats;

// To predict the duration of the next iteration based on the previous one.
// Assumes next iteration takes ~2x longer than the previous.
const NEXT_ITERATION_DURATION_FACTOR: f64 = 2.0;

pub struct SearchController {
    start_time: Instant,
    time_budget: Option<TimeBudget>,
    max_depth: Option<u8>,
    last_iteration_duration_ms: Option<u64>,
    current_iteration_start_ms: Option<u64>,
    stats: TimeControlStats,
}

impl SearchController {
    pub fn new(params: &GoParams, board: &Board, move_overhead_ms: u64) -> Self {
        Self {
            start_time: Instant::now(),
            time_budget: TimeBudget::new(params, board, move_overhead_ms),
            max_depth: params.depth,
            last_iteration_duration_ms: None,
            current_iteration_start_ms: None,
            stats: TimeControlStats::new(),
        }
    }

    /// Returns the hard deadline as an `Instant`, if time-controlled.
    pub fn deadline(&self) -> Option<Instant> {
        self.time_budget
            .map(|b| self.start_time + std::time::Duration::from_millis(b.hard_limit()))
    }

    pub fn should_continue_to_next_depth(&self, next_depth: u8) -> bool {
        // Depth check (if specified)
        if let Some(max_depth) = self.max_depth {
            return next_depth <= max_depth;
        }

        // Always allow the first iterations regardless of time gates.
        // Ensures we can produce at least one best move under extreme low time.
        if next_depth <= 2 {
            return true;
        }

        // Time check (if specified)
        if let Some(budget) = self.time_budget {
            let elapsed = self.elapsed().as_millis() as u64;

            match budget {
                // Exact (movetime): stop exactly at hard limit.
                TimeBudget::Exact { .. } => {
                    if elapsed >= budget.hard_limit() {
                        return false;
                    }
                }
                // Managed: stop at target and avoid starting an iteration that would exceed hard
                TimeBudget::Managed { .. } => {
                    // Stop at target
                    if elapsed >= budget.target_limit() {
                        return false;
                    }
                    // If still under target but estimate that the next iteration would exceed hard, stop early to save time.
                    if let Some(estimate) = self.estimate_next_iteration_duration() {
                        if elapsed.saturating_add(estimate) > budget.hard_limit() {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    fn estimate_next_iteration_duration(&self) -> Option<u64> {
        let last_duration = self.last_iteration_duration_ms?;

        if last_duration > 0 {
            return Some(((last_duration as f64) * NEXT_ITERATION_DURATION_FACTOR) as u64);
        }

        None
    }

    /// Returns the total elapsed time since search started.
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn on_iteration_start(&mut self) {
        let now_ms = self.elapsed().as_millis() as u64;

        // Calculate duration of the previous iteration
        if let Some(start_ms) = self.current_iteration_start_ms {
            let duration = now_ms.saturating_sub(start_ms);
            self.last_iteration_duration_ms = Some(duration);
        }

        self.current_iteration_start_ms = Some(now_ms);
    }

    pub fn on_iteration_complete(
        &mut self,
        depth: u8,
        score: i16,
        best_move: Option<Move>,
        pv_count: u8,
    ) {
        self.stats.add_iteration(depth, score, best_move);

        if let Some(ref mut budget) = self.time_budget {
            budget.adjust_for_search_behavior(&self.stats, pv_count);
        }
    }

    pub fn add_aspiration_failures(&mut self, count: u32) {
        self.stats.add_aspiration_failures(count);
    }
}
