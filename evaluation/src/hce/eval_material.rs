use chess::{BitBoard, Color, Piece, EMPTY};

use crate::hce::context::EvalContext;
use crate::hce::pst::{get_pst, sum_pst};
use crate::piece_values::PieceValues;

#[inline(always)]
pub(super) fn evaluate(ctx: &EvalContext, color: Color, piece_values: &PieceValues) -> i16 {
    let color_mask = if color == Color::White {
        &ctx.white_pieces
    } else {
        &ctx.black_pieces
    };

    // Use pre-fetched piece bitboards from context
    let pawn_mask = ctx.pawns & color_mask;
    let knight_mask = ctx.knights & color_mask;
    let bishop_mask = ctx.bishops & color_mask;
    let rook_mask = ctx.rooks & color_mask;
    let queen_mask = ctx.queens & color_mask;

    // King mask - reconstruct from king square
    let king_sq = if color == Color::White {
        ctx.white_king_sq
    } else {
        ctx.black_king_sq
    };
    let king_mask = BitBoard(1u64 << king_sq.to_int());

    let mut cp = 0i16;

    cp += piece_values.get(Piece::Pawn, ctx.phase) * pawn_mask.popcnt() as i16;
    cp += piece_values.get(Piece::Knight, ctx.phase) * knight_mask.popcnt() as i16;
    cp += piece_values.get(Piece::Bishop, ctx.phase) * bishop_mask.popcnt() as i16;
    cp += piece_values.get(Piece::Rook, ctx.phase) * rook_mask.popcnt() as i16;
    cp += piece_values.get(Piece::Queen, ctx.phase) * queen_mask.popcnt() as i16;

    let pst = get_pst(color);
    if pawn_mask != EMPTY {
        cp += sum_pst(pawn_mask, pst.pawn, ctx.phase, ctx.inv_phase);
    }
    if knight_mask != EMPTY {
        cp += sum_pst(knight_mask, pst.knight, ctx.phase, ctx.inv_phase);
    }
    if bishop_mask != EMPTY {
        cp += sum_pst(bishop_mask, pst.bishop, ctx.phase, ctx.inv_phase);
    }
    if rook_mask != EMPTY {
        cp += sum_pst(rook_mask, pst.rook, ctx.phase, ctx.inv_phase);
    }
    if queen_mask != EMPTY {
        cp += sum_pst(queen_mask, pst.queen, ctx.phase, ctx.inv_phase);
    }
    cp += sum_pst(king_mask, pst.king, ctx.phase, ctx.inv_phase); // king always present

    cp
}
