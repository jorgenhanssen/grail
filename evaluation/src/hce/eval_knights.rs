use super::HCEConfig;
use crate::hce::context::EvalContext;
use chess::{get_knight_moves, Color, EMPTY};

#[inline(always)]
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let knights = ctx.knights_for(color);
    if knights == EMPTY {
        return 0;
    }

    let my_pieces = ctx.color_mask_for(color);
    let mut cp = 0i16;
    for sq in knights {
        let squares = get_knight_moves(sq);
        let mobility = (squares & !my_pieces).popcnt() as i16;
        cp += ((config.knight_mobility_multiplier * mobility) as f32 * ctx.phase).round() as i16;
    }

    cp
}
