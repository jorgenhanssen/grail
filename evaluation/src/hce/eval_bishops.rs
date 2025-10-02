use super::HCEConfig;
use crate::hce::context::EvalContext;
use chess::{get_bishop_moves, Color, EMPTY};

#[inline(always)]
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let bishops = ctx.bishops_for(color);
    if bishops == EMPTY {
        return 0;
    }

    let mut cp = 0i16;

    // Bishop pair bonus
    if bishops.popcnt() >= 2 {
        cp += config.bishop_pair_bonus;
    }

    for sq in bishops {
        let mobility = get_bishop_moves(sq, ctx.all_pieces).popcnt() as i16;
        cp += ((config.bishop_mobility_multiplier * mobility) as f32 * ctx.phase).round() as i16;
    }

    cp
}
