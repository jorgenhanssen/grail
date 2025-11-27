use cozy_chess::{Color, Piece};

use crate::hce::context::EvalContext;
use crate::hce::pst::{get_pst, sum_pst};
use crate::piece_values::PieceValues;

pub(super) fn evaluate(ctx: &EvalContext, color: Color, piece_values: &PieceValues) -> i16 {
    let board = ctx.position.board;

    let pawns = board.colored_pieces(color, Piece::Pawn);
    let knights = board.colored_pieces(color, Piece::Knight);
    let bishops = board.colored_pieces(color, Piece::Bishop);
    let rooks = board.colored_pieces(color, Piece::Rook);
    let queens = board.colored_pieces(color, Piece::Queen);
    let king = board.king(color).bitboard();

    let mut cp = 0i16;

    cp += piece_values.get(Piece::Pawn, ctx.phase) * pawns.len() as i16;
    cp += piece_values.get(Piece::Knight, ctx.phase) * knights.len() as i16;
    cp += piece_values.get(Piece::Bishop, ctx.phase) * bishops.len() as i16;
    cp += piece_values.get(Piece::Rook, ctx.phase) * rooks.len() as i16;
    cp += piece_values.get(Piece::Queen, ctx.phase) * queens.len() as i16;

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
