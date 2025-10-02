use super::HCEConfig;
use crate::hce::context::EvalContext;
use chess::{get_file, get_rook_moves, Color, Rank, EMPTY};

#[inline(always)]
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let rooks = ctx.rooks_for(color);
    if rooks == EMPTY {
        return 0;
    }

    let our_pawns = ctx.pawns_for(color);
    let their_pawns = ctx.pawns_for(!color);

    let mut cp = 0i16;
    for sq in rooks {
        let file_mask = get_file(sq.get_file());

        let our_file_pawns = (our_pawns & file_mask).popcnt();
        let their_file_pawns = (their_pawns & file_mask).popcnt();

        cp += match (our_file_pawns == 0, their_file_pawns == 0) {
            (true, true) => config.rook_open_file_bonus,
            (true, false) => config.rook_semi_open_file_bonus,
            _ => 0,
        };

        // rook on seventh (second for Black)
        let rank = sq.get_rank();
        if (color == Color::White && rank == Rank::Seventh)
            || (color == Color::Black && rank == Rank::Second)
        {
            cp += config.rook_seventh_rank_bonus;
        }

        let mobility = get_rook_moves(sq, ctx.all_pieces).popcnt() as i16;
        cp += ((config.rook_mobility_multiplier * mobility) as f32 * ctx.phase).round() as i16;
    }
    cp
}
