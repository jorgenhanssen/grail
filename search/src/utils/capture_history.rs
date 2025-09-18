use chess::{Board, ChessMove, Piece, Square, NUM_PIECES, NUM_SQUARES};

const MAX_HISTORY: i32 = 512;
const MAX_DEPTH: usize = 100;

#[derive(Clone)]
pub struct CaptureHistory {
    // table[moved_piece][dest_square][captured_piece]
    table: [[[i16; NUM_PIECES]; NUM_SQUARES]; NUM_PIECES],
}

impl CaptureHistory {
    pub fn new() -> Self {
        Self {
            table: [[[0; NUM_PIECES]; NUM_SQUARES]; NUM_PIECES],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.table = [[[0; NUM_PIECES]; NUM_SQUARES]; NUM_PIECES];
    }

    #[inline(always)]
    pub fn get(&self, moved: Piece, dest: Square, captured: Piece) -> i16 {
        self.table[piece_index(moved)][dest.to_index()][piece_index(captured)]
    }

    #[inline(always)]
    pub fn update(&mut self, moved: Piece, dest: Square, captured: Piece, delta: i32) {
        let entry = &mut self.table[piece_index(moved)][dest.to_index()][piece_index(captured)];

        let h = *entry as i32;
        let b = delta.clamp(-MAX_HISTORY, MAX_HISTORY);

        // Gravity update like quiet history
        let new = h + b - ((h * b.abs()) / MAX_HISTORY);

        *entry = new.clamp(-MAX_HISTORY, MAX_HISTORY) as i16;
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
    pub fn get_bonus(&self, remaining_depth: u8) -> i32 {
        BONUS[remaining_depth.min(MAX_DEPTH as u8) as usize]
    }

    #[inline(always)]
    pub fn get_malus(&self, remaining_depth: u8) -> i32 {
        MALUS[remaining_depth.min(MAX_DEPTH as u8) as usize]
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

const BONUS: [i32; MAX_DEPTH + 1] = {
    let mut table = [0; MAX_DEPTH + 1];
    let mut i = 0;
    while i <= MAX_DEPTH {
        let depth = i as i32;
        table[i] = depth * depth;
        i += 1;
    }
    table
};

const MALUS: [i32; MAX_DEPTH + 1] = {
    let mut table = [0; MAX_DEPTH + 1];
    let mut i = 0;
    while i <= MAX_DEPTH {
        let depth = i as i32;
        table[i] = -2 * depth;
        i += 1;
    }
    table
};
