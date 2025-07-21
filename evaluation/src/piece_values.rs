use chess::ALL_PIECES;

pub const PAWN_VALUE: i16 = 100;
pub const KNIGHT_VALUE: i16 = 320;
pub const BISHOP_VALUE: i16 = 330;
pub const ROOK_VALUE: i16 = 500;
pub const QUEEN_VALUE: i16 = 900;
pub const KING_VALUE: i16 = 0;

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

#[inline(always)]
pub fn total_material(board: &chess::Board) -> i16 {
    let mut material = 0;
    for piece in ALL_PIECES {
        material += piece_value(piece) * (board.pieces(piece).popcnt() as i16);
    }
    material
}
