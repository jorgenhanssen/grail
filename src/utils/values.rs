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

pub const PASSED_PAWN_BONUS: [f32; 8] = [0.0, 5.0, 10.0, 20.0, 40.0, 70.0, 120.0, 200.0];
pub const ROOK_OPEN_FILE_BONUS: f32 = 15.0;
pub const ROOK_SEMI_OPEN_FILE_BONUS: f32 = 8.0;
pub const ROOK_ON_SEVENTH_BONUS: f32 = 20.0;
