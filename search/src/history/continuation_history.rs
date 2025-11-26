use cozy_chess::{Board, Color, Move, Square};

use crate::stack::SearchNode;
use crate::EngineConfig;

const CONT_HISTORY_SIZE: usize = Color::NUM * Square::NUM * Square::NUM;

#[derive(Clone)]
pub struct ContinuationHistory {
    // Index: [prev_color][prev_to][current_from][current_to]
    history: Vec<i16>,
    max_value: i32,
    bonus_multiplier: i32,
    malus_multiplier: i32,
}

impl ContinuationHistory {
    pub fn new(max_value: i32, bonus_multiplier: i32, malus_multiplier: i32) -> Self {
        Self {
            history: vec![0; Color::NUM * CONT_HISTORY_SIZE],
            max_value,
            bonus_multiplier,
            malus_multiplier,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.max_value = config.continuation_max_value.value;
        self.bonus_multiplier = config.continuation_bonus_multiplier.value;
        self.malus_multiplier = config.continuation_malus_multiplier.value;
        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.max_value == config.continuation_max_value.value
            && self.bonus_multiplier == config.continuation_bonus_multiplier.value
            && self.malus_multiplier == config.continuation_malus_multiplier.value
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.history.fill(0);
    }

    #[inline(always)]
    pub fn get(&self, color: Color, prev_to: &[Option<Square>], from: Square, to: Square) -> i16 {
        let mut total = 0i16;
        for &maybe_sq in prev_to {
            if let Some(prev) = maybe_sq {
                total += self.history[Self::index(color, prev, from, to)];
            }
        }
        total
    }

    #[inline(always)]
    pub fn update_quiet_all(
        &mut self,
        board: &Board,
        prev_to: &[Option<Square>],
        mv: Move,
        delta: i32,
    ) {
        let color = board.side_to_move();
        let from = mv.from;
        let to = mv.to;
        for &maybe_sq in prev_to {
            if let Some(prev) = maybe_sq {
                self.update_entry(color, prev, from, to, delta);
            }
        }
    }

    #[inline(always)]
    fn update_entry(
        &mut self,
        color: Color,
        prev_to: Square,
        from: Square,
        to: Square,
        delta: i32,
    ) {
        let idx = Self::index(color, prev_to, from, to);
        let entry = &mut self.history[idx];
        let h = *entry as i32;
        let b = delta.clamp(-self.max_value, self.max_value);
        let new = h + b - ((h * b.abs()) / self.max_value);
        *entry = new.clamp(-self.max_value, self.max_value) as i16;
    }

    #[inline(always)]
    fn index(color: Color, prev_to: Square, from: Square, to: Square) -> usize {
        let color_idx = color as usize;
        let prev_idx = prev_to as usize;
        let from_idx = from as usize;
        let to_idx = to as usize;

        color_idx * CONT_HISTORY_SIZE + prev_idx * Square::NUM + from_idx * Square::NUM + to_idx
    }

    /// Extract prev_to squares from the search stack for continuation history lookups
    #[inline(always)]
    pub fn get_prev_to_squares(&self, stack: &[SearchNode]) -> Vec<Option<Square>> {
        let len = stack.len();
        let mut prev_to = Vec::with_capacity(2);

        // 1 ply back (opponent's last move)
        if len >= 2 {
            prev_to.push(stack[len - 2].last_move.map(|m| m.to));
        }
        // 2 plies back (our previous move)
        if len >= 3 {
            prev_to.push(stack[len - 3].last_move.map(|m| m.to));
        }

        prev_to
    }

    #[inline(always)]
    pub fn get_bonus(&self, remaining_depth: u8) -> i32 {
        self.bonus_multiplier * remaining_depth as i32
    }

    #[inline(always)]
    pub fn get_malus(&self, remaining_depth: u8) -> i32 {
        -self.malus_multiplier * remaining_depth as i32
    }
}
