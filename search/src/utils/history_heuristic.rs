use chess::{Board, ChessMove, Color, Square, NUM_COLORS, NUM_SQUARES};

const MAX_HISTORY: i32 = 16_384;
const MAX_DEPTH: usize = 100;
const HISTORY_REDUCE_THRESHOLD: i16 = 0; // reduce quiet late moves if history <= 0
const HISTORY_LEAF_THRESHOLD: i16 = -1000; // prune quiet late moves if history very low
const HISTORY_MOVE_GATE: i32 = 5; // only consider after some moves have been tried

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
    fn update_move(&mut self, c: Color, source: Square, dest: Square, bonus: i32) {
        let entry = &mut self.history[c.to_index()][source.to_index()][dest.to_index()];

        let h = *entry as i32;
        let b = bonus.clamp(-(MAX_HISTORY), MAX_HISTORY);

        // Optimized Stockfish formula: h += bonus - (h * |bonus|) / 2¹⁴
        let new = h + b - ((h * b.abs()) >> 14);

        *entry = new.clamp(-(MAX_HISTORY), MAX_HISTORY) as i16;
    }

    #[inline(always)]
    pub fn update(&mut self, board: &Board, mv: ChessMove, delta: i32) {
        let color = board.side_to_move();
        let source = mv.get_source();
        let dest = mv.get_dest();
        self.update_move(color, source, dest, delta);
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn maybe_reduce_or_prune(
        &self,
        board: &Board,
        mv: ChessMove,
        depth: u8,
        max_depth: u8,
        remaining_depth: u8,
        in_check: bool,
        is_tactical: bool,
        is_pv_move: bool,
        move_index: i32,
        is_improving: bool,
        reduction: &mut u8,
    ) -> bool {
        if !(remaining_depth > 0
            && !in_check
            && !is_tactical
            && !is_pv_move
            && move_index >= HISTORY_MOVE_GATE)
        {
            return false;
        }

        let color = board.side_to_move();
        let source = mv.get_source();
        let dest = mv.get_dest();
        let hist_score = self.get(color, source, dest);

        if hist_score < HISTORY_REDUCE_THRESHOLD {
            // Only apply additional reductions/pruning when position isn't improving
            if !is_improving {
                *reduction = reduction.saturating_add(1);
            }

            let projected_child_max = max_depth.saturating_sub(*reduction);
            if !is_improving
                && hist_score < HISTORY_LEAF_THRESHOLD
                && projected_child_max <= depth + 1
            {
                return true; // prune
            }
        }

        false
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
