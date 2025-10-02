use chess::{BitBoard, Board, Color, Piece, Square};

// Pre-computed evaluation context to avoid redundant bitboard lookups
pub struct EvalContext {
    pub all_pieces: BitBoard,
    pub white_pieces: BitBoard,
    pub black_pieces: BitBoard,

    pub pawns: BitBoard,
    pub knights: BitBoard,
    pub bishops: BitBoard,
    pub rooks: BitBoard,
    pub queens: BitBoard,

    pub white_king_sq: Square,
    pub black_king_sq: Square,

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
            white_pieces: *board.color_combined(Color::White),
            black_pieces: *board.color_combined(Color::Black),
            white_king_sq: board.king_square(Color::White),
            black_king_sq: board.king_square(Color::Black),
            phase,
            inv_phase: 1.0 - phase,
        }
    }
}
