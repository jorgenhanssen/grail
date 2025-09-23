use super::HCEConfig;
use chess::{get_knight_moves, Board, Color, Piece, EMPTY};

#[inline(always)]
pub(super) fn evaluate(board: &Board, color: Color, phase: f32, config: &HCEConfig) -> i16 {
    let my_pieces = board.color_combined(color);
    let knights = board.pieces(Piece::Knight) & my_pieces;
    if knights == EMPTY {
        return 0;
    }

    let mut cp = 0i16;
    for sq in knights {
        let squares = get_knight_moves(sq);
        let mobility = (squares & !my_pieces).popcnt() as i16;
        cp += ((config.knight_mobility_multiplier * mobility) as f32 * phase).round() as i16;
    }

    cp
}
