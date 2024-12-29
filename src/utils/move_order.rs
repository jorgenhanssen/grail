use chess::{Board, ChessMove, MoveGen, Piece};

pub fn get_ordered_moves(board: &Board) -> Vec<(ChessMove, i32)> {
    let mut moves_with_scores: Vec<(ChessMove, i32)> = MoveGen::new_legal(board)
        .map(|m| (m, score(m, board)))
        .collect();

    moves_with_scores.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    moves_with_scores
}

pub const PROMOTION_SCORE: i32 = 10000;
const PROMOTION_SCORE_QUEEN: i32 = PROMOTION_SCORE + 4;
const PROMOTION_SCORE_ROOK: i32 = PROMOTION_SCORE + 3;
const PROMOTION_SCORE_BISHOP: i32 = PROMOTION_SCORE + 2;
const PROMOTION_SCORE_KNIGHT: i32 = PROMOTION_SCORE + 1;

pub const CAPTURE_SCORE: i32 = 1000;
const CAPTURE_SCORE_QUEEN: i32 = CAPTURE_SCORE + 5;
const CAPTURE_SCORE_ROOK: i32 = CAPTURE_SCORE + 4;
const CAPTURE_SCORE_BISHOP: i32 = CAPTURE_SCORE + 3;
const CAPTURE_SCORE_KNIGHT: i32 = CAPTURE_SCORE + 2;
const CAPTURE_SCORE_PAWN: i32 = CAPTURE_SCORE + 1;

pub const CHECK_SCORE: i32 = 100;

pub const PIECE_SCORE: i32 = 10;
const PIECE_SCORE_QUEEN: i32 = PIECE_SCORE + 6;
const PIECE_SCORE_ROOK: i32 = PIECE_SCORE + 5;
const PIECE_SCORE_BISHOP: i32 = PIECE_SCORE + 4;
const PIECE_SCORE_KNIGHT: i32 = PIECE_SCORE + 3;
const PIECE_SCORE_PAWN: i32 = PIECE_SCORE + 2;
const PIECE_SCORE_KING: i32 = PIECE_SCORE + 1;

fn score(move_: ChessMove, board: &Board) -> i32 {
    // Check for promotions first
    if let Some(promotion) = move_.get_promotion() {
        return match promotion {
            Piece::Queen => PROMOTION_SCORE_QUEEN,
            Piece::Rook => PROMOTION_SCORE_ROOK,
            Piece::Bishop => PROMOTION_SCORE_BISHOP,
            Piece::Knight => PROMOTION_SCORE_KNIGHT,
            _ => 0,
        };
    }

    // Next look at captures
    if let Some(victim) = board.piece_on(move_.get_dest()) {
        return match victim {
            Piece::Queen => CAPTURE_SCORE_QUEEN,
            Piece::Rook => CAPTURE_SCORE_ROOK,
            Piece::Bishop => CAPTURE_SCORE_BISHOP,
            Piece::Knight => CAPTURE_SCORE_KNIGHT,
            Piece::Pawn => CAPTURE_SCORE_PAWN,
            _ => 0,
        };
    }

    // Then look for checks
    let resulting_board = board.make_move_new(move_);
    if resulting_board.checkers().popcnt() > 0 {
        return CHECK_SCORE;
    }

    // For non-capture moves, return a small positive value based on the piece type
    // This encourages moving more valuable pieces first in quiet positions
    let piece = board.piece_on(move_.get_source()).unwrap();
    match piece {
        Piece::Queen => PIECE_SCORE_QUEEN,
        Piece::Rook => PIECE_SCORE_ROOK,
        Piece::Bishop => PIECE_SCORE_BISHOP,
        Piece::Knight => PIECE_SCORE_KNIGHT,
        Piece::Pawn => PIECE_SCORE_PAWN,
        Piece::King => PIECE_SCORE_KING,
    }
}
