use super::HCEConfig;
use chess::{get_bishop_moves, Board, Color, Piece, EMPTY};

#[inline(always)]
pub(super) fn evaluate(board: &Board, color: Color, phase: f32, config: &HCEConfig) -> i16 {
    let bishops = board.pieces(Piece::Bishop) & board.color_combined(color);
    if bishops == EMPTY {
        return 0;
    }

    let occupied = *board.combined();
    let mut cp = 0i16;

    // Bishop pair bonus
    if bishops.popcnt() >= 2 {
        cp += config.bishop_pair_bonus;
    }

    for sq in bishops {
        let mobility = get_bishop_moves(sq, occupied).popcnt() as i16;
        cp += ((config.bishop_mobility_multiplier * mobility) as f32 * phase).round() as i16;
    }

    cp
}
