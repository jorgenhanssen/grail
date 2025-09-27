use chess::{Board, ChessMove, Piece, Square, NUM_PIECES, NUM_SQUARES};

use crate::EngineConfig;

const MAX_DEPTH: usize = 100;

#[derive(Clone)]
pub struct CaptureHistory {
    // Flattened: [moved_piece][dest_square][captured_piece]
    table: Vec<i16>,

    max_history: i32,
    bonus_multiplier: i32,
    malus_multiplier: i32,
}

impl CaptureHistory {
    pub fn new(max_history: i32, bonus_multiplier: i32, malus_multiplier: i32) -> Self {
        const TABLE_SIZE: usize = NUM_PIECES * NUM_SQUARES * NUM_PIECES;
        Self {
            table: vec![0; TABLE_SIZE],
            max_history,
            bonus_multiplier,
            malus_multiplier,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.max_history = config.capture_history_max_value.value;
        self.bonus_multiplier = config.capture_history_bonus_multiplier.value;
        self.malus_multiplier = config.capture_history_malus_multiplier.value;

        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.max_history == config.capture_history_max_value.value
            && self.bonus_multiplier == config.capture_history_bonus_multiplier.value
            && self.malus_multiplier == config.capture_history_malus_multiplier.value
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.table.fill(0);
    }

    #[inline(always)]
    pub fn get(&self, moved: Piece, dest: Square, captured: Piece) -> i16 {
        self.table[Self::index(moved, dest, captured)]
    }

    #[inline(always)]
    pub fn update(&mut self, moved: Piece, dest: Square, captured: Piece, delta: i32) {
        let idx = Self::index(moved, dest, captured);
        let entry = &mut self.table[idx];

        let h = *entry as i32;
        let b = delta.clamp(-self.max_history, self.max_history);

        // Gravity update like quiet history
        let new = h + b - ((h * b.abs()) / self.max_history);

        *entry = new.clamp(-self.max_history, self.max_history) as i16;
    }

    #[inline(always)]
    pub fn update_capture(&mut self, board: &Board, mv: ChessMove, delta: i32) {
        let dest = mv.get_dest();
        let moved = match board.piece_on(mv.get_source()) {
            Some(p) => p,
            None => return,
        };
        let captured = match board.piece_on(dest) {
            Some(p) => p,
            None => return, // not a capture, ignore
        };
        self.update(moved, dest, captured, delta);
    }

    #[inline(always)]
    fn index(moved: Piece, dest: Square, captured: Piece) -> usize {
        let moved_idx = piece_index(moved);
        let dest_idx = dest.to_index();
        let captured_idx = piece_index(captured);

        let moved_stride = NUM_SQUARES * NUM_PIECES;
        let dest_stride = NUM_PIECES;

        moved_idx * moved_stride + dest_idx * dest_stride + captured_idx
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
}

#[inline(always)]
fn piece_index(piece: Piece) -> usize {
    match piece {
        Piece::King => 0,
        Piece::Queen => 1,
        Piece::Rook => 2,
        Piece::Bishop => 3,
        Piece::Knight => 4,
        Piece::Pawn => 5,
    }
}
