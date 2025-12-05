use evaluation::scores::SCORE_INF;

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
    alpha: i16,
    beta: i16,
    start_half: i16,
    widen: i16,
    enabled_from: u8,
}

impl AspirationWindow {
    pub fn new(start_half: i16, widen: i16, enabled_from: u8) -> Self {
        Self {
            alpha: -SCORE_INF,
            beta: SCORE_INF,
            start_half,
            widen,
            enabled_from,
        }
    }

    /// Sets up window for new depth based on previous score.
    pub fn begin_depth(&mut self, depth: u8, prev_score: i16) {
        if depth < self.enabled_from {
            self.alpha = -SCORE_INF;
            self.beta = SCORE_INF;
            return;
        }

        let half = (self.start_half + 10 * depth as i16).min(SCORE_INF);
        self.alpha = prev_score.saturating_sub(half);
        self.beta = prev_score.saturating_add(half);
    }

    pub fn bounds(&self) -> (i16, i16) {
        (self.alpha, self.beta)
    }

    /// Checks score against bounds; widens window on failure.
    pub fn analyse_pass(&mut self, score: i16) -> Pass {
        if score > self.alpha && score < self.beta {
            return Pass::Hit(score);
        }
        if score <= self.alpha {
            // fail‑low – widen only the low side
            let span = (self.beta - score).abs().max(self.start_half) * self.widen;
            self.alpha = score.saturating_sub(span);
            Pass::FailLow
        } else {
            // fail‑high
            let span = (score - self.alpha).abs().max(self.start_half) * self.widen;
            self.beta = score.saturating_add(span);
            Pass::FailHigh
        }
    }

    /// Fully opens the window after too many failures.
    pub fn fully_extend(&mut self) {
        self.alpha = -SCORE_INF;
        self.beta = SCORE_INF;
    }
}
