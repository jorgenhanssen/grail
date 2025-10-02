use super::HCEConfig;
use crate::hce::context::EvalContext;
use chess::{get_bishop_moves, get_rook_moves, Color, EMPTY};

#[inline(always)]
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let queens = ctx.queens_for(color);
    if queens == EMPTY {
        return 0;
    }

    let my_pieces = ctx.color_mask_for(color);
    let mut cp = 0i16;
    for sq in queens {
        let moves = get_bishop_moves(sq, ctx.all_pieces) | get_rook_moves(sq, ctx.all_pieces);
        let mobility = (moves & !my_pieces).popcnt() as i16;
        cp += ((config.queen_mobility_multiplier * mobility) as f32 * ctx.phase).round() as i16;
    }

    cp
}
