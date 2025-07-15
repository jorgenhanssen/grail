pub const PAWN_VALUE: i32 = 100;
pub const KNIGHT_VALUE: i32 = 320;
pub const BISHOP_VALUE: i32 = 330;
pub const ROOK_VALUE: i32 = 500;
pub const QUEEN_VALUE: i32 = 900;
pub const KING_VALUE: i32 = 1000;

#[inline(always)]
pub fn piece_value(piece: chess::Piece) -> i32 {
    match piece {
        chess::Piece::Pawn => PAWN_VALUE,
        chess::Piece::Knight => KNIGHT_VALUE,
        chess::Piece::Bishop => BISHOP_VALUE,
        chess::Piece::Rook => ROOK_VALUE,
        chess::Piece::Queen => QUEEN_VALUE,
        chess::Piece::King => KING_VALUE,
    }
}

pub const CHECKMATE_SCORE: i32 = 1_000_000;

pub const PASSED_PAWN_BONUS: [i32; 8] = [0, 5, 10, 20, 40, 70, 120, 200];
pub const ROOK_OPEN_FILE_BONUS: i32 = 15;
pub const ROOK_SEMI_OPEN_FILE_BONUS: i32 = 8;
pub const ROOK_ON_SEVENTH_BONUS: i32 = 20;
