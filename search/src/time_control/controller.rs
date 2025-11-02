use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chess::Board;
use uci::commands::GoParams;

use crate::time_control::budget::{SearchHistory, TimeBudget};

// To predict the duration of the next iteration based on the previous one.
// Assumes next iteration takes ~2x longer than the previous.
const NEXT_ITERATION_DURATION_FACTOR: f64 = 2.0;

pub struct SearchController {
    start_time: std::time::Instant,
    time_budget: Option<TimeBudget>,
    max_depth: Option<u8>,
    timer_handle: Option<thread::JoinHandle<()>>,
    on_stop_callback: Option<Arc<dyn Fn() + Send + Sync>>,
    last_iteration_duration_ms: Option<u64>,
    current_iteration_start_ms: Option<u64>,
    search_history: SearchHistory,
}

impl SearchController {
    pub fn new(params: &GoParams, board: &Board) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            time_budget: TimeBudget::new(params, board),
            max_depth: params.depth,
            timer_handle: None,
            on_stop_callback: None,
            last_iteration_duration_ms: None,
            current_iteration_start_ms: None,
            search_history: SearchHistory::new(),
        }
    }

    pub fn on_stop<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_stop_callback = Some(Arc::new(callback));
    }

    pub fn start_timer(&mut self) {
        let Some(budget) = self.time_budget else {
            return;
        };
        let Some(callback) = &self.on_stop_callback else {
            return;
        };

        let duration = Duration::from_millis(budget.hard_limit());
        let callback = Arc::clone(callback);

        let handle = thread::spawn(move || {
            thread::sleep(duration);
            callback();
        });

        self.timer_handle = Some(handle);
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
        best_move: Option<chess::ChessMove>,
    ) {
        self.search_history.add_iteration(depth, score, best_move);

        if let Some(ref mut budget) = self.time_budget {
            budget.adjust_for_search_behavior(&self.search_history);
        }
    }

    pub fn on_aspiration_failure(&mut self) {
        self.search_history.add_aspiration_failure();
    }

    pub fn stop_timer(&mut self) {
        if let Some(handle) = self.timer_handle.take() {
            std::mem::drop(handle);
        }
    }
}

impl Drop for SearchController {
    fn drop(&mut self) {
        self.stop_timer();
    }
}
