use cozy_chess::{
    BitBoard, Board, Move, Piece, Rank, get_bishop_moves, get_knight_moves, get_pawn_attacks,
    get_rook_moves,
};

use crate::moves::{en_passant_square, make_move};

/// Check if the side to move is in check.
pub fn has_check(board: &Board) -> bool {
    !board.checkers().is_empty()
}

/// Check if a move gives check to the opponent.
pub fn gives_check(board: &Board, mov: Move) -> bool {
    let next_board = make_move(board, mov);
    has_check(&next_board)
}

/// A slightly more efficient way to check if moves give check.
/// Basically just filters out moves that cannot give check before
/// falling back to the expensive gives_check.
pub struct CheckChecker<'a> {
    board: &'a Board,
    /// Starting squares that need a check test regardless of the destination.
    from_squares: BitBoard,
    /// Candidate destinations indexed by piece.
    to_squares: [BitBoard; Piece::NUM],
}

impl<'a> CheckChecker<'a> {
    pub fn new(board: &'a Board) -> Self {
        let us = board.side_to_move();
        let enemy_king = board.king(!us);

        let occupied = board.occupied();
        let bishop_check_rays = get_bishop_moves(enemy_king, occupied);
        let rook_check_rays = get_rook_moves(enemy_king, occupied);
        let knight_check_squares = get_knight_moves(enemy_king);

        let enemy_king_rays = bishop_check_rays | rook_check_rays;

        // if a blocker on the enemy kings slider ray moves, there can be a discovered check.
        // Also, castling starts at our king but can check with the rook so we have to check that too.
        let from_squares = enemy_king_rays | board.colored_pieces(us, Piece::King);

        let back_rank = Rank::Eighth.relative_to(us).bitboard();
        let promotion_checks = back_rank & (enemy_king_rays | knight_check_squares);
        let mut pawn_check_squares = get_pawn_attacks(enemy_king, !us) | promotion_checks;
        if let Some(ep) = en_passant_square(board) {
            // hehe gotcha: an en passant pawn can give check.
            pawn_check_squares |= ep.bitboard();
        }

        Self {
            board,
            from_squares,
            to_squares: Piece::ALL.map(|piece| match piece {
                Piece::Pawn => pawn_check_squares,
                Piece::Knight => knight_check_squares,
                Piece::Bishop => bishop_check_rays,
                Piece::Rook => rook_check_rays,
                Piece::Queen => enemy_king_rays,
                Piece::King => BitBoard::EMPTY,
            }),
        }
    }

    pub fn gives_check(&self, mov: Move, piece: Piece) -> bool {
        if !self.from_squares.has(mov.from) && !self.to_squares[piece as usize].has(mov.to) {
            return false;
        }

        gives_check(self.board, mov)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::collect_legal_moves;

    fn compare_checks(fen: &str) {
        let board: Board = fen.parse().unwrap();
        let checker = CheckChecker::new(&board);

        for mov in collect_legal_moves(&board) {
            let child = make_move(&board, mov);

            assert_eq!(
                checker.gives_check(mov, board.piece_on(mov.from).unwrap()),
                !child.checkers().is_empty(),
                "{fen}, move {mov}"
            );
        }
    }

    #[test]
    fn check_checker_matches_cozy() {
        let fens = [
            "5k2/8/8/8/8/8/8/4K2R w K - 0 1",
            "5k2/8/8/8/8/8/8/6KR w H - 0 1",
            "6k1/8/8/3pP3/8/8/B7/6K1 w - d6 0 2",
            "6k1/b7/8/8/3Pp3/8/5N2/6K1 b - d3 0 2",
            "4k3/8/8/8/4N3/8/8/K3R3 w - - 0 1",
            "4k3/7N/8/8/8/8/8/K7 w - - 0 1",
            "4k3/8/8/8/4B3/8/8/K3R3 w - - 0 1",
            "4k3/8/8/8/8/8/1B5R/K7 w - - 0 1",
            "4k3/5p2/8/7Q/8/8/8/K7 w - - 0 1",
            "4k3/P7/8/8/8/8/8/K7 w - - 0 1",
            "8/4k3/8/3P4/8/8/1P2P3/K7 w - - 0 1",
            "7k/8/8/8/8/5K2/4p3/8 b - - 0 1",
        ];

        for fen in fens {
            compare_checks(fen);
        }
    }
}
