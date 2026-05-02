use utils::FracPly;

use crate::EngineConfig;
use crate::MAX_DEPTH;

/// <https://www.chessprogramming.org/Late_Move_Reductions>
pub struct LmrTable {
    table: [FracPly; MAX_DEPTH * MAX_DEPTH],
    divisor: i32,
}

impl LmrTable {
    pub fn new(divisor: i32) -> Self {
        let mut table = [FracPly(0); MAX_DEPTH * MAX_DEPTH];
        let divisor_float = divisor as f32 / 100.0;
        for depth in 1..MAX_DEPTH {
            for move_index in 1..MAX_DEPTH {
                let depth_factor = (depth as f32).ln();
                let move_factor = (move_index as f32 / divisor_float).ln();
                let value = 0.5 + depth_factor * move_factor;
                table[Self::index(depth as u8, move_index as i32)] =
                    FracPly((value * FracPly::ONE as f32) as u16);
            }
        }

        Self { table, divisor }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        *self = Self::new(config.lmr_divisor.value);
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.divisor == config.lmr_divisor.value
    }

    pub fn get(&self, depth: u8, move_index: i32) -> FracPly {
        self.table[Self::index(depth, move_index)]
    }

    fn index(depth: u8, move_index: i32) -> usize {
        let rd = (depth as usize).min(MAX_DEPTH - 1);
        let mi = (move_index as usize).min(MAX_DEPTH - 1);
        rd * MAX_DEPTH + mi
    }
}
