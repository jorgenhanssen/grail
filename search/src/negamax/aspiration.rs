// aspiration.rs
use evaluation::scores::MATE_VALUE;

pub const ASP_HALF_START: i16 = 50; // ±0.50 pawn
pub const ASP_WIDEN: i16 = 2; // ×2 each miss
pub const ASP_ENABLED_FROM: u8 = 4; // start at depth 4
pub const ASP_MAX_RETRIES: u8 = 2; // bail to full window after this many retries

#[derive(PartialEq, Debug)]
pub enum Pass {
    Hit(i16),
    FailLow,
    FailHigh,
}

#[derive(Copy, Clone)]
pub struct AspirationWindow {
    alpha: i16,
    beta: i16,
    start_half: i16,  // ±50 cp at depth 1
    widen: i16,       // *2 each miss
    enabled_from: u8, // start at depth 4
}

impl AspirationWindow {
    pub fn new(start_half: i16, widen: i16, enabled_from: u8) -> Self {
        Self {
            alpha: -MATE_VALUE - 1,
            beta: MATE_VALUE + 1,
            start_half,
            widen,
            enabled_from,
        }
    }

    /// Call once at the top of each depth
    pub fn begin_depth(&mut self, depth: u8, prev_score: i16) {
        if depth < self.enabled_from {
            self.alpha = -MATE_VALUE - 1;
            self.beta = MATE_VALUE + 1;
            return;
        }

        let half = (self.start_half + 10 * depth as i16).min(MATE_VALUE);
        self.alpha = prev_score.saturating_sub(half);
        self.beta = prev_score.saturating_add(half);
    }

    #[inline(always)]
    pub fn bounds(&self) -> (i16, i16) {
        (self.alpha, self.beta)
    }

    /// Update window after each pass
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

    /// Force a full range on the next pass
    pub fn fallback_to_full(&mut self) {
        self.alpha = -MATE_VALUE - 1;
        self.beta = MATE_VALUE + 1;
    }
}
