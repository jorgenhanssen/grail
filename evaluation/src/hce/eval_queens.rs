use chess::{get_bishop_moves, get_rook_moves, Board, Color, Piece, EMPTY};

#[inline(always)]
pub(super) fn evaluate(board: &Board, color: Color, phase: f32) -> i16 {
    let my_pieces = board.color_combined(color);
    let queens = board.pieces(Piece::Queen) & my_pieces;
    if queens == EMPTY {
        return 0;
    }

    let occupied = *board.combined();

    let mut cp = 0i16;
    for sq in queens {
        let moves = get_bishop_moves(sq, occupied) | get_rook_moves(sq, occupied);
        let mobility = (moves & !my_pieces).popcnt() as i16;
        cp += ((mobility as f32) * phase).round() as i16;
    }

    cp
}
