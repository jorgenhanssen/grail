use chess::{Board, ChessMove, Color, Piece};

#[derive(Clone, Copy, Default)]
pub struct Castle(u8);

impl Castle {
    const WHITE: u8 = 1;
    const BLACK: u8 = 2;

    pub fn new() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub fn white_has_castled(&self) -> bool {
        self.0 & Self::WHITE != 0
    }

    #[inline(always)]
    pub fn black_has_castled(&self) -> bool {
        self.0 & Self::BLACK != 0
    }

    #[inline(always)]
    pub fn update(mut self, board: &Board, mv: ChessMove) -> Self {
        if is_castle(board, &mv) {
            let bit = match board.side_to_move() {
                Color::White => Self::WHITE,
                Color::Black => Self::BLACK,
            };
            self.0 |= bit;
        }
        self
    }
}

#[inline(always)]
pub fn is_castle(board: &Board, mv: &ChessMove) -> bool {
    if board.piece_on(mv.get_source()) != Some(Piece::King) {
        return false;
    }

    let f = mv.get_source().to_index() % 8;
    let t = mv.get_dest().to_index() % 8;
    (f as i8 - t as i8).abs() == 2
}
