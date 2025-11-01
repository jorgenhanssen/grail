use chess::{Board, ChessMove, Color, Square, NUM_COLORS, NUM_SQUARES};

use crate::EngineConfig;

const MAX_DEPTH: usize = 100;

#[derive(Clone)]
pub struct ContinuationHistory {
    // Flattened: [ply][color][prev_to][curr_from][curr_to]
    continuations: Vec<i16>,

    max_moves: usize,
    max_history: i32,
    bonus_multiplier: i32,
    malus_multiplier: i32,
}

impl ContinuationHistory {
    pub fn new(
        max_moves: usize,
        max_history: i32,
        bonus_multiplier: i32,
        malus_multiplier: i32,
    ) -> Self {
        let size = max_moves * NUM_COLORS * NUM_SQUARES * NUM_SQUARES * NUM_SQUARES;
        Self {
            continuations: vec![0; size],
            max_moves,
            max_history,
            bonus_multiplier,
            malus_multiplier,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.max_moves = config.continuation_max_moves.value;
        self.max_history = config.continuation_max_value.value;
        self.bonus_multiplier = config.continuation_bonus_multiplier.value;
        self.malus_multiplier = config.continuation_malus_multiplier.value;

        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.max_moves == config.continuation_max_moves.value
            && self.max_history == config.continuation_max_value.value
            && self.bonus_multiplier == config.continuation_bonus_multiplier.value
            && self.malus_multiplier == config.continuation_malus_multiplier.value
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        let size = self.max_moves * NUM_COLORS * NUM_SQUARES * NUM_SQUARES * NUM_SQUARES;
        self.continuations = vec![0; size];
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
        if ply >= self.max_moves {
            return 0;
        }
        if let Some(p_to) = prev_to {
            self.continuations[self.index(ply, color, p_to, src, dst)]
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn get(&self, color: Color, prev_to: &[Option<Square>], src: Square, dst: Square) -> i16 {
        let mut score = 0;
        for (ply, p_to) in prev_to.iter().enumerate().take(self.max_moves) {
            score += self.get_ply(ply, color, *p_to, src, dst);
        }
        score
    }

    #[inline(always)]
    pub fn get_bonus(&self, remaining_depth: u8) -> i32 {
        let depth = remaining_depth.min(MAX_DEPTH as u8) as i32;
        self.bonus_multiplier * depth
    }

    #[inline(always)]
    pub fn get_malus(&self, remaining_depth: u8) -> i32 {
        let depth = remaining_depth.min(MAX_DEPTH as u8) as i32;
        -self.malus_multiplier * depth
    }

    #[inline(always)]
    pub fn get_prev_to_squares(
        &self,
        search_stack: &[crate::negamax::search_stack::SearchNode],
    ) -> Vec<Option<Square>> {
        let len = search_stack.len();
        let mut vec = vec![None; self.max_moves];
        for i in 0..self.max_moves {
            if i < len {
                if let Some(mv) = search_stack[len - 1 - i].last_move {
                    vec[i] = Some(mv.get_dest());
                }
            }
        }
        vec
    }

    #[inline(always)]
    fn update_entry(entry: &mut i16, delta: i32, max_history: i32) {
        let h = *entry as i32;
        let b = delta.clamp(-max_history, max_history);
        let new = h + b - ((h * b.abs()) / max_history);
        *entry = new.clamp(-max_history, max_history) as i16;
    }

    #[inline(always)]
    fn update_continuations(
        &mut self,
        color: Color,
        prev_to: &[Option<Square>],
        src: Square,
        dst: Square,
        delta: i32,
    ) {
        for (ply, p_to_opt) in prev_to.iter().enumerate().take(self.max_moves) {
            if let Some(p_to) = *p_to_opt {
                let idx = self.index(ply, color, p_to, src, dst);
                let entry = &mut self.continuations[idx];
                Self::update_entry(entry, delta, self.max_history);
            }
        }
    }

    #[inline(always)]
    pub fn update_quiet_all(
        &mut self,
        board: &Board,
        prev_to: &[Option<Square>],
        mv: ChessMove,
        delta: i32,
    ) {
        let color = board.side_to_move();
        self.update_continuations(color, prev_to, mv.get_source(), mv.get_dest(), delta);
    }
}

impl ContinuationHistory {
    #[inline(always)]
    fn index(&self, ply: usize, color: Color, prev_to: Square, src: Square, dst: Square) -> usize {
        let ply_idx = ply;
        let color_idx = color.to_index();
        let prev_to_idx = prev_to.to_index();
        let src_idx = src.to_index();
        let dst_idx = dst.to_index();

        let ply_stride = NUM_COLORS * NUM_SQUARES * NUM_SQUARES * NUM_SQUARES;
        let color_stride = NUM_SQUARES * NUM_SQUARES * NUM_SQUARES;
        let prev_to_stride = NUM_SQUARES * NUM_SQUARES;
        let src_stride = NUM_SQUARES;

        ply_idx * ply_stride
            + color_idx * color_stride
            + prev_to_idx * prev_to_stride
            + src_idx * src_stride
            + dst_idx
    }
}
