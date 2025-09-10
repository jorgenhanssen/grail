use chess::{BitBoard, Board, Color, Piece, EMPTY};

use crate::piece_values::piece_value;
use crate::traditional::pst::{get_pst, sum_pst};

#[inline(always)]
pub(super) fn evaluate(board: &Board, color: Color, color_mask: &BitBoard, phase: f32) -> i16 {
    let pawn_mask = board.pieces(Piece::Pawn) & color_mask;
    let knight_mask = board.pieces(Piece::Knight) & color_mask;
    let bishop_mask = board.pieces(Piece::Bishop) & color_mask;
    let rook_mask = board.pieces(Piece::Rook) & color_mask;
    let queen_mask = board.pieces(Piece::Queen) & color_mask;
    let king_mask = board.pieces(Piece::King) & color_mask;

    let mut cp = 0i16;

    cp += piece_value(Piece::Pawn, phase) * pawn_mask.popcnt() as i16;
    cp += piece_value(Piece::Knight, phase) * knight_mask.popcnt() as i16;
    cp += piece_value(Piece::Bishop, phase) * bishop_mask.popcnt() as i16;
    cp += piece_value(Piece::Rook, phase) * rook_mask.popcnt() as i16;
    cp += piece_value(Piece::Queen, phase) * queen_mask.popcnt() as i16;

    let pst = get_pst(color);
    if pawn_mask != EMPTY {
        cp += sum_pst(pawn_mask, pst.pawn, phase);
    }
    if knight_mask != EMPTY {
        cp += sum_pst(knight_mask, pst.knight, phase);
    }
    if bishop_mask != EMPTY {
        cp += sum_pst(bishop_mask, pst.bishop, phase);
    }
    if rook_mask != EMPTY {
        cp += sum_pst(rook_mask, pst.rook, phase);
    }
    if queen_mask != EMPTY {
        cp += sum_pst(queen_mask, pst.queen, phase);
    }
    cp += sum_pst(king_mask, pst.king, phase); // king always present

    cp
}
