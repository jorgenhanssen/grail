use super::utils::RAZOR_NEAR_MATE;

const TREND_DELTA: i16 = 120;
const TREND_LOOKBACK: usize = 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Trend {
    Improving(u8),
    Neutral,
    Worsening(u8),
}

impl Trend {
    #[inline(always)]
    pub fn from_eval_stack(
        eval_stack: &[i16],
        current_eval: i16,
        in_check: bool,
        remaining_depth: u8,
    ) -> Self {
        if in_check || remaining_depth < 3 || eval_stack.len() < TREND_LOOKBACK {
            return Trend::Neutral;
        }

        let prev2 = eval_stack[eval_stack.len() - TREND_LOOKBACK];
        if current_eval.abs() >= RAZOR_NEAR_MATE || prev2.abs() >= RAZOR_NEAR_MATE {
            return Trend::Neutral;
        }

        let delta = current_eval - prev2;

        if delta >= TREND_DELTA {
            let s = delta / TREND_DELTA;
            Trend::Improving(s as u8)
        } else if delta <= -TREND_DELTA {
            let s = delta / TREND_DELTA;
            Trend::Worsening(s as u8)
        } else {
            Trend::Neutral
        }
    }
}
