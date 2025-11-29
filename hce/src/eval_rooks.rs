use super::HCEConfig;
use crate::context::EvalContext;
use cozy_chess::{Color, Piece, Rank};

pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let board = ctx.position.board;
    let rooks = board.colored_pieces(color, Piece::Rook);
    if rooks.is_empty() {
        return 0;
    }

    let our_pawns = board.colored_pieces(color, Piece::Pawn);
    let their_pawns = board.colored_pieces(!color, Piece::Pawn);

    let mut cp = 0i16;
    for sq in rooks {
        let file_bb = sq.file().bitboard();

        let our_file_pawns = (our_pawns & file_bb).len();
        let their_file_pawns = (their_pawns & file_bb).len();

        cp += match (our_file_pawns == 0, their_file_pawns == 0) {
            (true, true) => config.rook_open_file_bonus,
            (true, false) => config.rook_semi_open_file_bonus,
            _ => 0,
        };

        // rook on seventh (second for Black)
        let rank = sq.rank();
        if (color == Color::White && rank == Rank::Seventh)
            || (color == Color::Black && rank == Rank::Second)
        {
            cp += config.rook_seventh_rank_bonus;
        }
    }
    cp
}
