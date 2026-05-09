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
    start_half: i16,
    widen: i16,
    enabled_from: u8,
    score_divisor: i32,
}

impl AspirationWindow {
    pub fn new(start_half: i16, widen: i16, enabled_from: u8, score_divisor: i32) -> Self {
        Self {
            bounds: Bounds::FULL,
            start_half,
            widen,
            enabled_from,
            score_divisor,
        }
    }

    /// Sets up window for new depth based on previous score.
    /// Window size increases with score magnitude - winning positions are more volatile.
    pub fn begin_depth(&mut self, depth: u8, prev_score: i16) {
        if depth < self.enabled_from {
            self.bounds = Bounds::FULL;
            return;
        }
        // Mate scores are too large for the quadratic scaling below (i16 overflow).
        if prev_score.abs() >= MATE_SCORE_BOUND {
            self.bounds = Bounds::FULL;
            return;
        }

        // Score-based adjustment: larger scores get wider windows (inspired by 4ku).
        // Quadratic scaling: with divisor=16384: adds ish 0 at score=0, 5 at score=300, 15 at score=500
        let score_squared = (prev_score as i32) * (prev_score as i32);
        let score_adjustment = (score_squared / self.score_divisor) as i16;

        let half = (self.start_half + 10 * depth as i16 + score_adjustment).min(SCORE_INF);

        self.bounds = Bounds::new(
            prev_score.saturating_sub(half),
            prev_score.saturating_add(half),
        );
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// Checks score against bounds; widens window on failure.
    pub fn analyse_pass(&mut self, score: i16) -> Pass {
        if score > self.bounds.alpha && score < self.bounds.beta {
            return Pass::Hit(score);
        }
        if score <= self.bounds.alpha {
            // fail-low: widen only the low side
            let span = (self.bounds.beta - score).abs().max(self.start_half) * self.widen;
            self.bounds.alpha = score.saturating_sub(span);
            Pass::FailLow
        } else {
            // fail-high
            let span = (score - self.bounds.alpha).abs().max(self.start_half) * self.widen;
            self.bounds.beta = score.saturating_add(span);
            Pass::FailHigh
        }
    }

    /// Fully opens the window after too many failures.
    pub fn fully_extend(&mut self) {
        self.bounds = Bounds::FULL;
    }
}
