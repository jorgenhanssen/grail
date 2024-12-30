pub const PAWN_VALUE: f32 = 100.0;
pub const KNIGHT_VALUE: f32 = 320.0;
pub const BISHOP_VALUE: f32 = 330.0;
pub const ROOK_VALUE: f32 = 500.0;
pub const QUEEN_VALUE: f32 = 900.0;
pub const KING_VALUE: f32 = 1000.0;

#[inline(always)]
pub fn piece_value(piece: chess::Piece) -> f32 {
    match piece {
        chess::Piece::Pawn => PAWN_VALUE,
        chess::Piece::Knight => KNIGHT_VALUE,
        chess::Piece::Bishop => BISHOP_VALUE,
        chess::Piece::Rook => ROOK_VALUE,
        chess::Piece::Queen => QUEEN_VALUE,
        chess::Piece::King => KING_VALUE,
    }
}

pub const CHECKMATE_SCORE: f32 = 1_000_000.0;
