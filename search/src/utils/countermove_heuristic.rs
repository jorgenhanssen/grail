use chess::{ChessMove, Color, Piece, Square, NUM_COLORS, NUM_SQUARES};

const NUM_PIECES: usize = 6;

#[inline(always)]
fn piece_to_index(piece: Piece) -> usize {
    match piece {
        Piece::King => 0,
        Piece::Queen => 1,
        Piece::Rook => 2,
        Piece::Bishop => 3,
        Piece::Knight => 4,
        Piece::Pawn => 5,
    }
}

#[derive(Clone)]
pub struct CountermoveHeuristic {
    countermove: [[[Option<ChessMove>; NUM_SQUARES]; NUM_PIECES]; NUM_COLORS],
}

impl CountermoveHeuristic {
    pub fn new() -> Self {
        Self {
            countermove: [[[None; NUM_SQUARES]; NUM_PIECES]; NUM_COLORS],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.countermove = [[[None; NUM_SQUARES]; NUM_PIECES]; NUM_COLORS];
    }

    #[inline(always)]
    pub fn get(&self, color: Color, piece: Piece, square: Square) -> Option<ChessMove> {
        self.countermove[color.to_index()][piece_to_index(piece)][square.to_index()]
    }

    #[inline(always)]
    pub fn update(&mut self, color: Color, piece: Piece, square: Square, mov: ChessMove) {
        self.countermove[color.to_index()][piece_to_index(piece)][square.to_index()] = Some(mov);
    }
}
