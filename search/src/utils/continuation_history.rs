use chess::{Board, ChessMove, Color, Square, NUM_COLORS, NUM_SQUARES};

const MAX_HISTORY: i32 = 512;
const MAX_DEPTH: usize = 100;
pub const MAX_CONT_PLIES: usize = 4;

#[derive(Clone)]
pub struct ContinuationHistory {
    // [ply][color][prev_to][curr_from][curr_to]
    continuations: [[[[[i16; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS]; MAX_CONT_PLIES],
}

impl ContinuationHistory {
    pub fn new() -> Self {
        Self {
            continuations: [[[[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS];
                MAX_CONT_PLIES],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.continuations =
            [[[[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS]; MAX_CONT_PLIES];
    }

    #[inline(always)]
    pub fn get_ply(
        &self,
        ply: usize,
        color: Color,
        prev_to: Option<Square>,
        src: Square,
        dst: Square,
    ) -> i16 {
        if ply >= MAX_CONT_PLIES {
            return 0;
        }
        if let Some(p_to) = prev_to {
            self.continuations[ply][color.to_index()][p_to.to_index()][src.to_index()]
                [dst.to_index()]
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
        let mut score = 0;
        for ply in 0..MAX_CONT_PLIES {
            score += self.get_ply(ply, color, prev_to[ply], src, dst);
        }
        score
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
    fn update_continuations(
        &mut self,
        color: Color,
        prev_to: &[Option<Square>; MAX_CONT_PLIES],
        src: Square,
        dst: Square,
        delta: i32,
    ) {
        for ply in 0..MAX_CONT_PLIES {
            if let Some(p_to) = prev_to[ply] {
                let entry = &mut self.continuations[ply][color.to_index()][p_to.to_index()]
                    [src.to_index()][dst.to_index()];
                Self::update_entry(entry, delta);
            }
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
        self.update_continuations(color, prev_to, mv.get_source(), mv.get_dest(), delta);
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
