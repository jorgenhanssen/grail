use cozy_chess::{Board, Color, Move, Piece, Rank};

/// Passed pawn extension: returns 1 for pawn pushes to 7th rank, 0 otherwise.
/// Non-captures only - captures are already tactical.
pub fn extension(board: &Board, m: &Move, moved_piece: Piece, is_capture: bool) -> u8 {
    if moved_piece != Piece::Pawn || is_capture || m.promotion.is_some() {
        return 0;
    }
    let is_seventh = match board.side_to_move() {
        Color::White => m.to.rank() == Rank::Seventh,
        Color::Black => m.to.rank() == Rank::Second,
    };
    if is_seventh {
        1
    } else {
        0
    }
}
