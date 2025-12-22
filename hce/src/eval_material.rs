use cozy_chess::{Color, Piece};
use utils::{BISHOP_VALUE, KNIGHT_VALUE, PAWN_VALUE, QUEEN_VALUE, ROOK_VALUE};

use crate::context::EvalContext;
use crate::pst::{get_pst, sum_pst};

pub(super) fn evaluate(ctx: &EvalContext, color: Color) -> i16 {
    let board = ctx.position.board;

    let pawns = board.colored_pieces(color, Piece::Pawn);
    let knights = board.colored_pieces(color, Piece::Knight);
    let bishops = board.colored_pieces(color, Piece::Bishop);
    let rooks = board.colored_pieces(color, Piece::Rook);
    let queens = board.colored_pieces(color, Piece::Queen);
    let king = board.king(color).bitboard();

    let mut cp = 0i16;

    cp += PAWN_VALUE * pawns.len() as i16;
    cp += KNIGHT_VALUE * knights.len() as i16;
    cp += BISHOP_VALUE * bishops.len() as i16;
    cp += ROOK_VALUE * rooks.len() as i16;
    cp += QUEEN_VALUE * queens.len() as i16;

    let pst = get_pst(color);
    if !pawns.is_empty() {
        cp += sum_pst(pawns, pst.pawn, ctx.phase, ctx.inv_phase);
    }
    if !knights.is_empty() {
        cp += sum_pst(knights, pst.knight, ctx.phase, ctx.inv_phase);
    }
    if !bishops.is_empty() {
        cp += sum_pst(bishops, pst.bishop, ctx.phase, ctx.inv_phase);
    }
    if !rooks.is_empty() {
        cp += sum_pst(rooks, pst.rook, ctx.phase, ctx.inv_phase);
    }
    if !queens.is_empty() {
        cp += sum_pst(queens, pst.queen, ctx.phase, ctx.inv_phase);
    }
    cp += sum_pst(king, pst.king, ctx.phase, ctx.inv_phase);

    cp
}
