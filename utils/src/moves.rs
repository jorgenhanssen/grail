use cozy_chess::{Board, Move};

/// Check if a move is a capture (enemy piece on destination).
///
/// Handles castling correctly: cozy-chess represents castling as
/// "king captures rook", but this function only returns true for
/// actual captures of enemy pieces.
pub fn is_capture(board: &Board, mv: Move) -> bool {
    board.colors(!board.side_to_move()).has(mv.to)
}

/// Make a move and return a new board.
pub fn make_move(board: &Board, mv: Move) -> Board {
    let mut new_board = board.clone();
    new_board.play_unchecked(mv);
    new_board
}

/// Check if there are any legal moves in the position.
pub fn has_legal_moves(board: &Board) -> bool {
    board.generate_moves(|_| true)
}

/// Check if there is exactly one legal move in the position.
pub fn only_move(board: &Board) -> bool {
    let mut count = 0;
    board.generate_moves(|moves| {
        count += moves.len();
        count > 1
    });
    count == 1
}

/// Collect all legal moves into a Vec.
pub fn collect_legal_moves(board: &Board) -> Vec<Move> {
    let mut moves = Vec::new();
    board.generate_moves(|batch| {
        moves.extend(batch);
        false
    });
    moves
}

/// Check if the side to move is in check.
pub fn has_check(board: &Board) -> bool {
    !board.checkers().is_empty()
}

/// Check if a move gives check to the opponent.
pub fn gives_check(board: &Board, mv: Move) -> bool {
    let new_board = make_move(board, mv);
    has_check(&new_board)
}

#[cfg(test)]
mod tests {
    use cozy_chess::Square;

    use super::*;

    fn mv(from: &str, to: &str) -> Move {
        Move {
            from: from.parse::<Square>().unwrap(),
            to: to.parse::<Square>().unwrap(),
            promotion: None,
        }
    }

    #[test]
    fn test_is_capture() {
        // Starting position - e2e4 is not a capture
        let board = Board::default();
        assert!(!is_capture(&board, mv("e2", "e4")));

        // Position with capture available
        let board: Board = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2"
            .parse()
            .unwrap();
        assert!(is_capture(&board, mv("e4", "d5")));
    }

    #[test]
    fn test_castling_not_capture() {
        // Position where white can castle kingside
        let board: Board = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1"
            .parse()
            .unwrap();
        // Kingside castle in cozy-chess is e1h1 (king captures rook notation)
        assert!(!is_capture(&board, mv("e1", "h1")));
    }

    #[test]
    fn test_has_legal_moves() {
        // Starting position has moves
        assert!(has_legal_moves(&Board::default()));

        // Stalemate position - black king in corner, no legal moves
        let board: Board = "k7/2Q5/1K6/8/8/8/8/8 b - - 0 1".parse().unwrap();
        assert!(!has_legal_moves(&board));
    }

    #[test]
    fn test_only_move() {
        // King in corner with only one escape square
        let board: Board = "k7/8/8/8/8/8/1r6/K7 w - - 0 1".parse().unwrap();
        let moves = collect_legal_moves(&board);
        assert_eq!(only_move(&board), moves.len() == 1);
    }

    #[test]
    fn test_has_check() {
        // Starting position - not in check
        assert!(!has_check(&Board::default()));

        // Position with check
        let board: Board = "rnbqkbnr/ppppp1pp/8/5p1Q/4P3/8/PPPP1PPP/RNB1KBNR b KQkq - 1 2"
            .parse()
            .unwrap();
        assert!(has_check(&board));
    }

    #[test]
    fn test_gives_check() {
        // Position where Qxf7 gives check
        let board: Board = "rnbqkbnr/pppp1ppp/8/4p2Q/4P3/8/PPPP1PPP/RNB1KBNR w KQkq - 1 3"
            .parse()
            .unwrap();
        assert!(gives_check(&board, mv("h5", "f7")));

        // e4 from starting position doesn't give check
        assert!(!gives_check(&Board::default(), mv("e2", "e4")));
    }

    #[test]
    fn test_collect_legal_moves_starting_position() {
        // Starting position has 20 legal moves
        assert_eq!(collect_legal_moves(&Board::default()).len(), 20);
    }
}
