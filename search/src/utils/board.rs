use chess::{Board, ChessMove, MoveGen, Piece};

#[inline(always)]
pub fn only_move(board: &Board) -> bool {
    let mut g = MoveGen::new_legal(board);
    matches!((g.next(), g.next()), (Some(_), None))
}

#[inline(always)]
pub fn is_zugzwang(board: &Board) -> bool {
    let side_bits = *board.color_combined(board.side_to_move());
    let pawn_bits = *board.pieces(Piece::Pawn) & side_bits;
    let king_bits = *board.pieces(Piece::King) & side_bits;

    // Only king and pawns (common zugzwang scenario)
    if side_bits == (pawn_bits | king_bits) {
        return true;
    }

    // Positions with no pawns and no major pieces and at most
    // one minor piece are also prone to null-move failures.
    let knight_bits = *board.pieces(Piece::Knight) & side_bits;
    let bishop_bits = *board.pieces(Piece::Bishop) & side_bits;
    let rook_bits = *board.pieces(Piece::Rook) & side_bits;
    let queen_bits = *board.pieces(Piece::Queen) & side_bits;

    let has_pawns = pawn_bits.popcnt() > 0;
    let has_major = (rook_bits | queen_bits).popcnt() > 0;
    let minor_count = (knight_bits | bishop_bits).popcnt();

    !has_pawns && !has_major && minor_count <= 1
}

#[inline(always)]
pub fn game_phase(board: &Board) -> f32 {
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);

    let score = knights.popcnt() + bishops.popcnt() + 2 * rooks.popcnt() + 4 * queens.popcnt();

    (score.min(24) as f32) / 24.0
}

#[inline(always)]
pub fn gives_check(board: &Board, mv: ChessMove) -> bool {
    board.make_move_new(mv).checkers().popcnt() > 0
}

