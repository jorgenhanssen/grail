//! Tracks search behavior for dynamic time adjustments.

use cozy_chess::Move;

// Thresholds for time adjustment decisions
pub const SCORE_DROP_THRESHOLD: i16 = 50;
pub const MIN_DEPTH_FOR_ADJUSTMENTS: u8 = 6;
pub const EASY_MOVE_THRESHOLD: f64 = 0.6;

/// Tracks search iterations to detect patterns like stable best moves or score drops.
/// Used to dynamically adjust time allocation during search.
#[derive(Debug, Clone)]
pub struct TimeControlStats {
    pub scores: Vec<i16>,
    pub depths: Vec<u8>,
    pub best_moves: Vec<Option<Move>>,
    pub aspiration_failures: u32,
}

impl TimeControlStats {
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

    pub fn current_depth(&self) -> u8 {
        self.depths.last().copied().unwrap_or(0)
    }

    /// Returns true if the best move has been stable across recent iterations.
    pub fn best_move_is_stable(&self) -> bool {
        if self.best_moves.len() < 4 {
            return false;
        }

        let recent_moves = &self.best_moves[self.best_moves.len() - 4..];
        let first_move = &recent_moves[0];

        let same_move_count = recent_moves
            .iter()
            .filter(|&mv| mv == first_move && mv.is_some())
            .count();

        (same_move_count as f64) / (recent_moves.len() as f64) >= EASY_MOVE_THRESHOLD
    }

    /// Returns true if the best move changed in the last iteration.
    pub fn best_move_changed_recently(&self) -> bool {
        if self.best_moves.len() < 2 {
            return false;
        }

        let last_two = &self.best_moves[self.best_moves.len() - 2..];
        last_two[0] != last_two[1] && last_two[0].is_some() && last_two[1].is_some()
    }

    /// Returns true if the score dropped significantly in the last iteration.
    pub fn has_score_drop(&self) -> bool {
        if self.scores.len() < 2 {
            return false;
        }

        let last_two = &self.scores[self.scores.len() - 2..];
        (last_two[0] - last_two[1]) >= SCORE_DROP_THRESHOLD
    }
}
