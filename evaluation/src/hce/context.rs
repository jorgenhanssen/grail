use chess::{BitBoard, Board, Color, Piece, Square, NUM_COLORS};

// Pre-computed evaluation context to avoid redundant bitboard lookups
pub struct EvalContext {
    pub all_pieces: BitBoard,
    pub pieces_by_color: [BitBoard; NUM_COLORS],

    pub pawns: BitBoard,
    pub knights: BitBoard,
    pub bishops: BitBoard,
    pub rooks: BitBoard,
    pub queens: BitBoard,

    pub king_sqs: [Square; NUM_COLORS],

    pub phase: f32,
    pub inv_phase: f32,
}

impl EvalContext {
    #[inline(always)]
    pub fn new(board: &Board, phase: f32) -> Self {
        Self {
            all_pieces: *board.combined(),
            pawns: *board.pieces(Piece::Pawn),
            knights: *board.pieces(Piece::Knight),
            bishops: *board.pieces(Piece::Bishop),
            rooks: *board.pieces(Piece::Rook),
            queens: *board.pieces(Piece::Queen),
            pieces_by_color: [
                *board.color_combined(Color::White),
                *board.color_combined(Color::Black),
            ],
            king_sqs: [
                board.king_square(Color::White),
                board.king_square(Color::Black),
            ],
            phase,
            inv_phase: 1.0 - phase,
        }
    }

    #[inline(always)]
    pub fn color_mask_for(&self, color: Color) -> &BitBoard {
        &self.pieces_by_color[color.to_index()]
    }

    #[inline(always)]
    pub fn king_sq_for(&self, color: Color) -> Square {
        self.king_sqs[color.to_index()]
    }

    #[inline(always)]
    pub fn pawns_for(&self, color: Color) -> BitBoard {
        self.pawns & self.color_mask_for(color)
    }

    #[inline(always)]
    pub fn knights_for(&self, color: Color) -> BitBoard {
        self.knights & self.color_mask_for(color)
    }

    #[inline(always)]
    pub fn bishops_for(&self, color: Color) -> BitBoard {
        self.bishops & self.color_mask_for(color)
    }

    #[inline(always)]
    pub fn rooks_for(&self, color: Color) -> BitBoard {
        self.rooks & self.color_mask_for(color)
    }

    #[inline(always)]
    pub fn queens_for(&self, color: Color) -> BitBoard {
        self.queens & self.color_mask_for(color)
    }
}
