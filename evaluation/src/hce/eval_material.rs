use chess::{BitBoard, Board, Color, Piece, EMPTY};

use crate::hce::pst::{get_pst, sum_pst};
use crate::piece_values::PieceValues;

#[inline(always)]
pub(super) fn evaluate(
    board: &Board,
    color: Color,
    color_mask: &BitBoard,
    phase: f32,
    inv_phase: f32,
    piece_values: &PieceValues,
) -> i16 {
    let pawn_mask = board.pieces(Piece::Pawn) & color_mask;
    let knight_mask = board.pieces(Piece::Knight) & color_mask;
    let bishop_mask = board.pieces(Piece::Bishop) & color_mask;
    let rook_mask = board.pieces(Piece::Rook) & color_mask;
    let queen_mask = board.pieces(Piece::Queen) & color_mask;
    let king_mask = board.pieces(Piece::King) & color_mask;

    let mut cp = 0i16;

    cp += piece_values.get(Piece::Pawn, phase) * pawn_mask.popcnt() as i16;
    cp += piece_values.get(Piece::Knight, phase) * knight_mask.popcnt() as i16;
    cp += piece_values.get(Piece::Bishop, phase) * bishop_mask.popcnt() as i16;
    cp += piece_values.get(Piece::Rook, phase) * rook_mask.popcnt() as i16;
    cp += piece_values.get(Piece::Queen, phase) * queen_mask.popcnt() as i16;

    let pst = get_pst(color);
    if pawn_mask != EMPTY {
        cp += sum_pst(pawn_mask, pst.pawn, phase, inv_phase);
    }
    if knight_mask != EMPTY {
        cp += sum_pst(knight_mask, pst.knight, phase, inv_phase);
    }
    if bishop_mask != EMPTY {
        cp += sum_pst(bishop_mask, pst.bishop, phase, inv_phase);
    }
    if rook_mask != EMPTY {
        cp += sum_pst(rook_mask, pst.rook, phase, inv_phase);
    }
    if queen_mask != EMPTY {
        cp += sum_pst(queen_mask, pst.queen, phase, inv_phase);
    }
    cp += sum_pst(king_mask, pst.king, phase, inv_phase); // king always present

    cp
}
