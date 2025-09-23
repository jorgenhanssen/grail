use super::HCEConfig;
use chess::{get_file, get_rook_moves, Board, Color, Piece, Rank, EMPTY};

#[inline(always)]
pub(super) fn evaluate(board: &Board, color: Color, phase: f32, config: &HCEConfig) -> i16 {
    let rooks = board.pieces(Piece::Rook) & board.color_combined(color);
    if rooks == EMPTY {
        return 0;
    }

    let our_pawns = board.pieces(Piece::Pawn) & board.color_combined(color);
    let their_pawns = board.pieces(Piece::Pawn) & board.color_combined(!color);
    let occupied = *board.combined();

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

        let mobility = get_rook_moves(sq, occupied).popcnt() as i16;
        cp += ((config.rook_mobility_multiplier * mobility) as f32 * phase).round() as i16;
    }
    cp
}
