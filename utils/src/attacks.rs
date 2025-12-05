use cozy_chess::{
    get_bishop_moves, get_king_moves, get_knight_moves, get_pawn_attacks, get_rook_moves, BitBoard,
    Board, Color, Piece, Square,
};

/// Returns a bitboard of all pieces attacking the given square.
///
/// Uses custom occupancy to properly handle X-ray attacks through pieces
/// that have already captured.
#[inline]
pub fn get_attackers_to(board: &Board, sq: Square, occupied: BitBoard) -> BitBoard {
    let knights = board.pieces(Piece::Knight);
    let kings = board.pieces(Piece::King);
    let bishops_queens = board.pieces(Piece::Bishop) | board.pieces(Piece::Queen);
    let rooks_queens = board.pieces(Piece::Rook) | board.pieces(Piece::Queen);

    // Pawns: reverse pawn attacks to find pawns that can attack this square
    let white_pawn_attackers =
        get_pawn_attacks(sq, Color::Black) & board.colored_pieces(Color::White, Piece::Pawn);
    let black_pawn_attackers =
        get_pawn_attacks(sq, Color::White) & board.colored_pieces(Color::Black, Piece::Pawn);

    white_pawn_attackers
        | black_pawn_attackers
        | (get_knight_moves(sq) & knights)
        | (get_king_moves(sq) & kings)
        | (get_bishop_moves(sq, occupied) & bishops_queens)
        | (get_rook_moves(sq, occupied) & rooks_queens)
}

/// Returns newly discovered attackers after a piece is removed.
///
/// When a piece captures and is removed from the board, it may reveal hidden
/// attackers behind it. This function returns those newly visible attackers.
#[inline]
pub fn get_discovered_attacks(
    piece_moved: Piece,
    target: Square,
    occupied: BitBoard,
    bishops_queens: BitBoard,
    rooks_queens: BitBoard,
) -> BitBoard {
    let mut xrays = BitBoard::EMPTY;

    // Diagonal sliders revealed when pawn/bishop/queen moves away
    if matches!(piece_moved, Piece::Pawn | Piece::Bishop | Piece::Queen) {
        xrays |= get_bishop_moves(target, occupied) & bishops_queens;
    }

    // Orthogonal sliders revealed when rook/queen moves away
    if matches!(piece_moved, Piece::Rook | Piece::Queen) {
        xrays |= get_rook_moves(target, occupied) & rooks_queens;
    }

    xrays
}
