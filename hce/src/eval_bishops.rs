use super::HCEConfig;
use crate::context::EvalContext;
use cozy_chess::{Color, Piece};

pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let board = ctx.position.board;
    let bishops = board.colored_pieces(color, Piece::Bishop);
    if bishops.is_empty() {
        return 0;
    }

    let mut cp = 0i16;

    // Bishop pair bonus
    if bishops.len() >= 2 {
        cp += config.bishop_pair_bonus;
    }

    cp
}
