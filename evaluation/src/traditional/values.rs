pub const PAWN_VALUE: i16 = 100;
pub const KNIGHT_VALUE: i16 = 320;
pub const BISHOP_VALUE: i16 = 330;
pub const ROOK_VALUE: i16 = 500;
pub const QUEEN_VALUE: i16 = 900;
pub const KING_VALUE: i16 = 1000;

#[inline(always)]
pub fn piece_value(piece: chess::Piece) -> i16 {
    match piece {
        chess::Piece::Pawn => PAWN_VALUE,
        chess::Piece::Knight => KNIGHT_VALUE,
        chess::Piece::Bishop => BISHOP_VALUE,
        chess::Piece::Rook => ROOK_VALUE,
        chess::Piece::Queen => QUEEN_VALUE,
        chess::Piece::King => KING_VALUE,
    }
}

pub const PASSED_PAWN_BONUS: [i16; 8] = [0, 10, 20, 40, 80, 140, 220, 0];
pub const ROOK_OPEN_FILE_BONUS: i16 = 15;
pub const ROOK_SEMI_OPEN_FILE_BONUS: i16 = 8;
pub const ROOK_ON_SEVENTH_BONUS: i16 = 20;
