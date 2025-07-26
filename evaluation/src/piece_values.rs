use chess::{Piece, ALL_PIECES};

pub const PAWN_MG: i16 = 98;
pub const PAWN_EG: i16 = 113;
pub const KNIGHT_MG: i16 = 325;
pub const KNIGHT_EG: i16 = 340;
pub const BISHOP_MG: i16 = 335;
pub const BISHOP_EG: i16 = 350;
pub const ROOK_MG: i16 = 510;
pub const ROOK_EG: i16 = 560;
pub const QUEEN_MG: i16 = 975;
pub const QUEEN_EG: i16 = 1020;
pub const KING_MG: i16 = 0;
pub const KING_EG: i16 = 0;

#[inline(always)]
pub fn piece_value(piece: Piece, phase: f32) -> i16 {
    let (mg, eg) = match piece {
        Piece::Pawn => (PAWN_MG as f32, PAWN_EG as f32),
        Piece::Knight => (KNIGHT_MG as f32, KNIGHT_EG as f32),
        Piece::Bishop => (BISHOP_MG as f32, BISHOP_EG as f32),
        Piece::Rook => (ROOK_MG as f32, ROOK_EG as f32),
        Piece::Queen => (QUEEN_MG as f32, QUEEN_EG as f32),
        Piece::King => (KING_MG as f32, KING_EG as f32),
    };
    ((mg * phase) + (eg * (1.0 - phase))).round() as i16
}

#[inline(always)]
pub fn total_material(board: &chess::Board, phase: f32) -> i16 {
    let mut material = 0;
    for piece in ALL_PIECES {
        material += piece_value(piece, phase) * (board.pieces(piece).popcnt() as i16);
    }
    material
}
