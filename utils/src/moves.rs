use cozy_chess::{Board, Color, Move, Piece, Rank, Square};

pub fn en_passant_square(board: &Board) -> Option<Square> {
    let file = board.en_passant()?;
    let rank = if board.side_to_move() == Color::White {
        Rank::Sixth
    } else {
        Rank::Third
    };
    Some(Square::new(file, rank))
}

pub fn is_en_passant(board: &Board, mv: Move) -> bool {
    let Some(ep_sq) = en_passant_square(board) else {
        return false;
    };
    mv.to == ep_sq && board.piece_on(mv.from) == Some(Piece::Pawn)
}

pub fn is_capture(board: &Board, mv: Move) -> bool {
    if board.colors(!board.side_to_move()).has(mv.to) {
        return true;
    }

    if is_en_passant(board, mv) {
        return true;
    }

    false
}

pub fn captured_piece(board: &Board, mv: Move) -> Option<Piece> {
    if let Some(piece) = board.piece_on(mv.to) {
        return Some(piece);
    }

    // No piece on destination so it is EP if file matches.
    // Assumes is_capture(board, mv) is true.
    if board.en_passant().is_some_and(|ep| mv.to.file() == ep) {
        return Some(Piece::Pawn);
    }

    None
}

/// Make a move and return a new board.
pub fn make_move(board: &Board, mv: Move) -> Board {
    let mut new_board = board.clone();
    new_board.play_unchecked(mv);
    new_board
}

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
    fn capture_detection() {
        let start = Board::default();
        assert!(!is_capture(&start, mv("e2", "e4")));

        let after_d5: Board = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2"
            .parse()
            .unwrap();
        assert!(is_capture(&after_d5, mv("e4", "d5")));

        // cozy-chess encodes castling as king-takes-own-rook
        let castling: Board = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1"
            .parse()
            .unwrap();
        assert!(!is_capture(&castling, mv("e1", "h1")));
    }

    #[test]
    fn legal_move_helpers() {
        let start = Board::default();
        assert!(has_legal_moves(&start));
        assert_eq!(collect_legal_moves(&start).len(), 20);

        let stalemate: Board = "k7/2Q5/1K6/8/8/8/8/8 b - - 0 1".parse().unwrap();
        assert!(!has_legal_moves(&stalemate));

        let one_move: Board = "k7/8/8/8/8/8/1r6/K7 w - - 0 1".parse().unwrap();
        assert_eq!(
            only_move(&one_move),
            collect_legal_moves(&one_move).len() == 1
        );
    }

    #[test]
    fn check_detection() {
        let start = Board::default();
        assert!(!has_check(&start));
        assert!(!gives_check(&start, mv("e2", "e4")));

        let in_check: Board = "rnbqkbnr/ppppp1pp/8/5p1Q/4P3/8/PPPP1PPP/RNB1KBNR b KQkq - 1 2"
            .parse()
            .unwrap();
        assert!(has_check(&in_check));

        let qxf7: Board = "rnbqkbnr/pppp1ppp/8/4p2Q/4P3/8/PPPP1PPP/RNB1KBNR w KQkq - 1 3"
            .parse()
            .unwrap();
        assert!(gives_check(&qxf7, mv("h5", "f7")));
    }
}
