use std::time::Duration;

use cozy_chess::Color;
use uci::commands::GoParams;

pub(super) struct Clock {
    remaining: [Duration; Color::NUM],
    increment: Duration,
}

impl Clock {
    pub fn new(initial: Duration) -> Self {
        Self {
            remaining: [initial; Color::NUM],
            increment: initial / 100,
        }
    }

    pub fn get_go_params(&self) -> GoParams {
        GoParams {
            wtime: Some(self.remaining[Color::White as usize].as_millis() as u64),
            btime: Some(self.remaining[Color::Black as usize].as_millis() as u64),
            winc: Some(self.increment.as_millis() as u64),
            binc: Some(self.increment.as_millis() as u64),
            ..Default::default()
        }
    }

    pub fn finish_move(&mut self, color: Color, elapsed: Duration) -> bool {
        let remaining = &mut self.remaining[color as usize];
        if elapsed >= *remaining {
            eprintln!(
                "Time forfeit: {color:?} used {:.3}s of {:.3}s remaining",
                elapsed.as_secs_f64(),
                remaining.as_secs_f64(),
            );
            *remaining = Duration::ZERO;
            return false;
        }

        *remaining = *remaining - elapsed + self.increment;

        true
    }
}
