const TREND_DELTA: i16 = 100;
const TREND_LOOKBACK: usize = 2;
const TREND_MAX_STRENGTH: i16 = 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Trend {
    Improving(u8),
    Neutral,
    Worsening(u8),
}

impl Trend {
    #[inline(always)]
    pub fn new(eval: i16, eval_stack: &[i16], in_check: bool, remaining_depth: u8) -> Self {
        if in_check || remaining_depth < 3 || eval_stack.len() < TREND_LOOKBACK {
            return Trend::Neutral;
        }

        // Find delta between current eval and eval x moves before
        let delta = eval - eval_stack[eval_stack.len() - TREND_LOOKBACK];
        let abs_delta = delta.abs();

        if abs_delta < TREND_DELTA {
            return Trend::Neutral;
        }

        // Faster than strength.min(TREND_MAX_STRENGTH)
        let strength = if abs_delta >= TREND_MAX_STRENGTH * TREND_DELTA {
            TREND_MAX_STRENGTH as u8
        } else {
            (abs_delta / TREND_DELTA) as u8
        };

        if delta > 0 {
            Trend::Improving(strength)
        } else {
            Trend::Worsening(strength)
        }
    }
}
