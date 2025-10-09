use super::HCEConfig;
use crate::hce::context::EvalContext;
use chess::{Color, Piece, EMPTY};

#[inline(always)]
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let board = ctx.position.board;
    let bishops = board.pieces(Piece::Bishop) & board.color_combined(color);
    if bishops == EMPTY {
        return 0;
    }

    let mut cp = 0i16;

    // Bishop pair bonus
    if bishops.popcnt() >= 2 {
        cp += config.bishop_pair_bonus;
    }

    cp
}
