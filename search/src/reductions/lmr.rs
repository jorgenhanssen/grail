use crate::MAX_DEPTH;

pub struct LmrTable {
    table: [u8; MAX_DEPTH * MAX_DEPTH],
}

impl LmrTable {
    pub fn new(divisor: f32) -> Self {
        let mut table = [0; MAX_DEPTH * MAX_DEPTH];
        for remaining_depth in 1..MAX_DEPTH {
            for move_index in 1..MAX_DEPTH {
                let depth_factor = (remaining_depth as f32).ln();
                let move_factor = (move_index as f32 / divisor).ln();
                let value = (0.5 + (depth_factor * move_factor)) as u8;
                table[Self::index(remaining_depth as u8, move_index as i32)] = value;
            }
        }

        Self { table }
    }

    pub fn get(&self, remaining_depth: u8, move_index: i32) -> u8 {
        self.table[Self::index(remaining_depth, move_index)]
    }

    fn index(remaining_depth: u8, move_index: i32) -> usize {
        remaining_depth as usize * MAX_DEPTH + move_index as usize
    }
}
