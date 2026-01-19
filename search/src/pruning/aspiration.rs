use evaluation::scores::SCORE_INF;

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
}

impl AspirationWindow {
    pub fn new(start_half: i16, widen: i16, enabled_from: u8) -> Self {
        Self {
            bounds: Bounds::FULL,
            start_half,
            widen,
            enabled_from,
        }
    }

    /// Sets up window for new depth based on previous score.
    pub fn begin_depth(&mut self, depth: u8, prev_score: i16) {
        if depth < self.enabled_from {
            self.bounds = Bounds::FULL;
            return;
        }

        let half = (self.start_half + 10 * depth as i16).min(SCORE_INF);
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
            // fail‑low – widen only the low side
            let span = (self.bounds.beta - score).abs().max(self.start_half) * self.widen;
            self.bounds.alpha = score.saturating_sub(span);
            Pass::FailLow
        } else {
            // fail‑high
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
