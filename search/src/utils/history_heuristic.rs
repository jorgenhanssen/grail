use chess::{Color, Square, NUM_COLORS, NUM_SQUARES};

const MAX_HISTORY: i32 = 16_384;
const MAX_DEPTH: usize = 100;

#[derive(Clone)]
pub struct HistoryHeuristic {
    // history[color][from][to]
    history: [[[i16; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
}

impl HistoryHeuristic {
    pub fn new() -> Self {
        Self {
            history: [[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.history = [[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS];
    }

    #[inline(always)]
    pub fn get(&self, color: Color, source: Square, dest: Square) -> i16 {
        self.history[color.to_index()][source.to_index()][dest.to_index()]
    }

    #[inline(always)]
    pub fn update(&mut self, c: Color, source: Square, dest: Square, bonus: i32) {
        let entry = &mut self.history[c.to_index()][source.to_index()][dest.to_index()];

        let h = *entry as i32;
        let b = bonus.clamp(-(MAX_HISTORY), MAX_HISTORY);

        // Optimized Stockfish formula: h += bonus - (h * |bonus|) / 2¹⁴
        let new = h + b - ((h * b.abs()) >> 14);

        *entry = new.clamp(-(MAX_HISTORY), MAX_HISTORY) as i16;
    }

    #[inline(always)]
    pub fn get_bonus(&self, remaining_depth: u8) -> i32 {
        BONUS[remaining_depth.min(MAX_DEPTH as u8) as usize]
    }

    #[inline(always)]
    pub fn get_malus(&self, remaining_depth: u8) -> i32 {
        MALUS[remaining_depth.min(MAX_DEPTH as u8) as usize]
    }
}

const BONUS: [i32; MAX_DEPTH + 1] = {
    let mut table = [0; MAX_DEPTH + 1];
    let mut i = 0;
    while i <= MAX_DEPTH {
        let depth = i as i32;
        table[i] = 32 * depth * depth + 16 * depth;
        i += 1;
    }
    table
};
const MALUS: [i32; MAX_DEPTH + 1] = {
    let mut table = [0; MAX_DEPTH + 1];
    let mut i = 0;
    while i <= MAX_DEPTH {
        let depth = i as i32;
        table[i] = -(12 * depth * depth + 6 * depth);
        i += 1;
    }
    table
};
