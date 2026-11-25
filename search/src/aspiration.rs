use config::EngineConfig;

use crate::scores::{MATE_SCORE_BOUND, SCORE_INF};

use crate::utils::Bounds;

#[derive(PartialEq, Debug)]
pub enum Pass {
    Hit(i16),
    FailLow,
    FailHigh,
}

/// Manages aspiration window bounds across search iterations.
/// Starts with a narrow window around the previous score, widens on fail-low/fail-high.
///
/// <https://www.chessprogramming.org/Aspiration_Windows>
#[derive(Copy, Clone)]
pub struct AspirationWindow {
    bounds: Bounds,
    start_half: i32,
    widen: i32,
    enabled_from: u8,
    score_divisor: i32,
}

impl AspirationWindow {
    pub fn new(config: &EngineConfig) -> Self {
        Self {
            bounds: Bounds::FULL,
            start_half: config.aspiration_window_size,
            widen: config.aspiration_window_widen,
            enabled_from: config.aspiration_window_depth,
            score_divisor: config.aspiration_score_divisor,
        }
    }

    /// Sets up window for new depth based on previous score.
    /// Window size increases with score magnitude (winning positions are more volatile).
    pub fn begin_depth(&mut self, depth: u8, prev_score: i16) {
        if depth < self.enabled_from {
            self.bounds = Bounds::FULL;
            return;
        }
        // Use the full window for mate scores.
        if prev_score.unsigned_abs() >= MATE_SCORE_BOUND as u16 {
            self.bounds = Bounds::FULL;
            return;
        }

        // Score-based adjustment: larger scores get wider windows (inspired by 4ku).
        // Quadratic scaling: with divisor=16384: adds ish 0 at score=0, 5 at score=300, 15 at score=500
        let score_squared = (prev_score as i32) * (prev_score as i32);
        let score_adjustment = score_squared / self.score_divisor;
        let half = self.start_half + score_adjustment;
        let half = half.min(SCORE_INF as i32) as i16;

        self.bounds = Bounds::new(
            prev_score.saturating_sub(half).max(-SCORE_INF),
            prev_score.saturating_add(half).min(SCORE_INF),
        );
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// Checks score against bounds and widens the window on failure.
    pub fn analyse_pass(&mut self, score: i16) -> Pass {
        if score > self.bounds.alpha && score < self.bounds.beta {
            return Pass::Hit(score);
        }
        if score <= self.bounds.alpha {
            // fail-low: widen only the low side
            let distance = self.bounds.beta as i32 - score as i32;
            let span = distance.max(self.start_half) * self.widen / 100;
            self.bounds.alpha = (score as i32 - span).max(-SCORE_INF as i32) as i16;

            Pass::FailLow
        } else {
            // fail-high
            let distance = score as i32 - self.bounds.alpha as i32;
            let span = distance.max(self.start_half) * self.widen / 100;
            self.bounds.beta = (score as i32 + span).min(SCORE_INF as i32) as i16;

            Pass::FailHigh
        }
    }

    /// Fully opens the window after too many failures.
    pub fn fully_extend(&mut self) {
        self.bounds = Bounds::FULL;
    }
}
