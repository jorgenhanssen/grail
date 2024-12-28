use chess::{Board, ChessMove, MoveGen, Piece};

pub fn get_ordered_moves(board: &Board) -> Vec<ChessMove> {
    let mut moves_with_scores: Vec<(ChessMove, i32)> = MoveGen::new_legal(board)
        .map(|m| (m, score(m, board)))
        .collect();

    moves_with_scores.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    moves_with_scores.into_iter().map(|(m, _)| m).collect()
}

fn score(move_: ChessMove, board: &Board) -> i32 {
    // Check for promotions first
    if let Some(promotion) = move_.get_promotion() {
        return match promotion {
            Piece::Queen => 20000,
            Piece::Rook => 19000,
            Piece::Bishop | Piece::Knight => 18000,
            _ => 0,
        };
    }

    // Then look for checks
    let resulting_board = board.make_move_new(move_);
    if resulting_board.checkers().popcnt() > 0 {
        return 15000;
    }

    // Next look at captures
    if let Some(victim) = board.piece_on(move_.get_dest()) {
        return match victim {
            Piece::Queen => 900,
            Piece::Rook => 500,
            Piece::Bishop => 330, // Slightly higher than knight
            Piece::Knight => 320,
            Piece::Pawn => 100,
            Piece::King => 0, // Shouldn't happen in legal moves
        };
    }

    // For non-capture moves, return a small positive value based on the piece type
    // This encourages moving more valuable pieces first in quiet positions
    let piece = board.piece_on(move_.get_source()).unwrap();
    match piece {
        Piece::Queen => 50,
        Piece::Rook => 40,
        Piece::Bishop => 30,
        Piece::Knight => 30,
        Piece::Pawn => 20,
        Piece::King => 10,
    }
}
