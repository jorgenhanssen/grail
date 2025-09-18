use chess::{Board, ChessMove, Color, Square, NUM_COLORS, NUM_SQUARES};

const MAX_HISTORY: i32 = 512;
const MAX_DEPTH: usize = 100;
pub const MAX_CONT_PLIES: usize = 3;

#[derive(Clone)]
pub struct ContinuationHistory {
    // 1-ply continuation: indexed by color, prev_to, curr_from, curr_to
    cont1: [[[[i16; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
    // 2-ply continuation: indexed by color, prev2_to (same side two plies ago), curr_from, curr_to
    cont2: [[[[i16; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
    // 3-ply continuation
    cont3: [[[[i16; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
}

impl ContinuationHistory {
    pub fn new() -> Self {
        Self {
            cont1: [[[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
            cont2: [[[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
            cont3: [[[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.cont1 = [[[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS];
        self.cont2 = [[[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS];
        self.cont3 = [[[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS];
    }

    #[inline(always)]
    pub fn get1(&self, color: Color, prev_to: Option<Square>, src: Square, dst: Square) -> i16 {
        if let Some(p_to) = prev_to {
            self.cont1[color.to_index()][p_to.to_index()][src.to_index()][dst.to_index()]
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn get2(&self, color: Color, prev2_to: Option<Square>, src: Square, dst: Square) -> i16 {
        if let Some(p2_to) = prev2_to {
            self.cont2[color.to_index()][p2_to.to_index()][src.to_index()][dst.to_index()]
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn get3(&self, color: Color, prev3_to: Option<Square>, src: Square, dst: Square) -> i16 {
        if let Some(p3_to) = prev3_to {
            self.cont3[color.to_index()][p3_to.to_index()][src.to_index()][dst.to_index()]
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn get(
        &self,
        color: Color,
        prev_to: &[Option<Square>; MAX_CONT_PLIES],
        src: Square,
        dst: Square,
    ) -> i16 {
        self.get1(color, prev_to[0], src, dst)
            + self.get2(color, prev_to[1], src, dst)
            + self.get3(color, prev_to[2], src, dst)
    }

    #[inline(always)]
    pub fn get_bonus(&self, remaining_depth: u8) -> i32 {
        BONUS[remaining_depth.min(MAX_DEPTH as u8) as usize]
    }

    #[inline(always)]
    pub fn get_malus(&self, remaining_depth: u8) -> i32 {
        MALUS[remaining_depth.min(MAX_DEPTH as u8) as usize]
    }

    #[inline(always)]
    fn update_entry(entry: &mut i16, delta: i32) {
        let h = *entry as i32;
        let b = delta.clamp(-MAX_HISTORY, MAX_HISTORY);
        let new = h + b - ((h * b.abs()) / MAX_HISTORY);
        *entry = new.clamp(-MAX_HISTORY, MAX_HISTORY) as i16;
    }

    #[inline(always)]
    fn update_tables(
        &mut self,
        color: Color,
        prev_to: &[Option<Square>; MAX_CONT_PLIES],
        src: Square,
        dst: Square,
        delta: i32,
    ) {
        if let Some(p1) = prev_to[0] {
            let entry =
                &mut self.cont1[color.to_index()][p1.to_index()][src.to_index()][dst.to_index()];
            Self::update_entry(entry, delta);
        }
        if let Some(p2) = prev_to[1] {
            let entry =
                &mut self.cont2[color.to_index()][p2.to_index()][src.to_index()][dst.to_index()];
            Self::update_entry(entry, delta);
        }
        if let Some(p3) = prev_to[2] {
            let entry =
                &mut self.cont3[color.to_index()][p3.to_index()][src.to_index()][dst.to_index()];
            Self::update_entry(entry, delta);
        }
    }

    #[inline(always)]
    pub fn update_quiet_all(
        &mut self,
        board: &Board,
        prev_to: &[Option<Square>; MAX_CONT_PLIES],
        mv: ChessMove,
        delta: i32,
    ) {
        let color = board.side_to_move();
        self.update_tables(color, prev_to, mv.get_source(), mv.get_dest(), delta);
    }
}

const BONUS: [i32; MAX_DEPTH + 1] = {
    let mut table = [0; MAX_DEPTH + 1];
    let mut i = 0;
    while i <= MAX_DEPTH {
        let depth = i as i32;
        table[i] = 9 * depth; // slightly smaller than quiet history bonus
        i += 1;
    }
    table
};

const MALUS: [i32; MAX_DEPTH + 1] = {
    let mut table = [0; MAX_DEPTH + 1];
    let mut i = 0;
    while i <= MAX_DEPTH {
        let depth = i as i32;
        table[i] = -(7 * depth);
        i += 1;
    }
    table
};
