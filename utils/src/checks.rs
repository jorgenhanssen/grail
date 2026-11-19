use cozy_chess::{BitBoard, Board, Move, Piece, get_knight_moves};

use crate::attacks::get_queen_moves;
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
    /// Destinations that need a check test regardless of the starting square.
    to_squares: BitBoard,
}

impl<'a> CheckChecker<'a> {
    pub fn new(board: &'a Board) -> Self {
        let us = board.side_to_move();
        let enemy_king = board.king(!us);

        let enemy_king_rays = get_queen_moves(enemy_king, board.occupied());

        // if a blocker on the enemy kings slider ray moves, there can be a discovered check.
        // Also, castling starts at our king but can check with the rook so we have to check that too.
        let from_squares = enemy_king_rays | board.colored_pieces(us, Piece::King);

        // A move to one of these squares will give direct check.
        let mut to_squares = enemy_king_rays | get_knight_moves(enemy_king);
        if let Some(ep) = en_passant_square(board) {
            // hehe gotcha: an en passant pawn can give check.
            to_squares |= ep.bitboard();
        }

        Self {
            board,
            from_squares,
            to_squares,
        }
    }

    pub fn gives_check(&self, mov: Move) -> bool {
        if !self.from_squares.has(mov.from) && !self.to_squares.has(mov.to) {
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
                checker.gives_check(mov),
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
            "4k3/5p2/8/7Q/8/8/8/K7 w - - 0 1",
            "4k3/P7/8/8/8/8/8/K7 w - - 0 1",
        ];

        for fen in fens {
            compare_checks(fen);
        }
    }
}
