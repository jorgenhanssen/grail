use cozy_chess::{Board, Move};

/// Check if a move is a real capture (enemy piece on destination).
/// Correctly handles castling, which cozy-chess represents as "king captures rook".
#[inline(always)]
pub fn is_capture(board: &Board, mv: Move) -> bool {
    board.colors(!board.side_to_move()).has(mv.to)
}

/// Make a move and return a new board.
#[inline(always)]
pub fn make_move(board: &Board, mv: Move) -> Board {
    let mut new_board = board.clone();
    new_board.play_unchecked(mv);
    new_board
}

/// Check if there are any legal moves in the position.
#[inline(always)]
pub fn has_legal_moves(board: &Board) -> bool {
    board.generate_moves(|_| true)
}

/// Check if there is exactly one legal move in the position.
#[inline(always)]
pub fn only_move(board: &Board) -> bool {
    let mut count = 0;
    board.generate_moves(|moves| {
        count += moves.len();
        count > 1
    });
    count == 1
}

/// Collect all legal moves into a Vec.
#[inline(always)]
pub fn collect_legal_moves(board: &Board) -> Vec<Move> {
    let mut moves = Vec::new();
    board.generate_moves(|batch| {
        moves.extend(batch);
        false
    });
    moves
}

/// Check if the side to move is in check.
#[inline(always)]
pub fn has_check(board: &Board) -> bool {
    !board.checkers().is_empty()
}

/// Check if a move gives check to the opponent.
#[inline(always)]
pub fn gives_check(board: &Board, mv: Move) -> bool {
    let new_board = make_move(board, mv);
    has_check(&new_board)
}
