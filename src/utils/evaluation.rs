use crate::utils::values::{BISHOP_VALUE, KNIGHT_VALUE, PAWN_VALUE, QUEEN_VALUE, ROOK_VALUE};
use chess::{Board, BoardStatus, Color, Piece};

use super::{get_pst, sum_pst, CHECKMATE_SCORE};

// Return final evaluation (positive = good for White, negative = good for Black)
pub fn evaluate_board(board: &Board) -> f32 {
    match board.status() {
        BoardStatus::Checkmate => {
            // If it’s White to move and board is checkmated => White lost
            if board.side_to_move() == chess::Color::White {
                return -CHECKMATE_SCORE;
            } else {
                return CHECKMATE_SCORE;
            }
        }
        BoardStatus::Stalemate => return 0.0,
        BoardStatus::Ongoing => {}
    }

    let mut score = 0.0;
    score += evaluate_material(board, Color::White);
    score -= evaluate_material(board, Color::Black);

    score += evaluate_pawn_structure(board, Color::White);
    score -= evaluate_pawn_structure(board, Color::Black);

    score
}

fn evaluate_material(board: &Board, color: Color) -> f32 {
    let color_mask = board.color_combined(color);

    let pawn_mask = board.pieces(Piece::Pawn) & color_mask;
    let knight_mask = board.pieces(Piece::Knight) & color_mask;
    let bishop_mask = board.pieces(Piece::Bishop) & color_mask;
    let rook_mask = board.pieces(Piece::Rook) & color_mask;
    let queen_mask = board.pieces(Piece::Queen) & color_mask;
    let king_mask = board.pieces(Piece::King) & color_mask;

    let num_pawns = pawn_mask.popcnt();
    let num_knights = knight_mask.popcnt();
    let num_bishops = bishop_mask.popcnt();
    let num_rooks = rook_mask.popcnt();
    let num_queens = queen_mask.popcnt();

    let piece_value = PAWN_VALUE * num_pawns as f32
        + KNIGHT_VALUE * num_knights as f32
        + BISHOP_VALUE * num_bishops as f32
        + ROOK_VALUE * num_rooks as f32
        + QUEEN_VALUE * num_queens as f32;

    let pst = get_pst(color);
    let mut pst_value = 0.0;
    if num_pawns > 0 {
        pst_value += sum_pst(pawn_mask, pst.pawn);
    }
    if num_knights > 0 {
        pst_value += sum_pst(knight_mask, pst.knight);
    }
    if num_bishops > 0 {
        pst_value += sum_pst(bishop_mask, pst.bishop);
    }
    if num_rooks > 0 {
        pst_value += sum_pst(rook_mask, pst.rook);
    }
    if num_queens > 0 {
        pst_value += sum_pst(queen_mask, pst.queen);
    }
    pst_value += sum_pst(king_mask, pst.king);

    // bonus for bishop pair
    let bishop_pair_bonus = if num_bishops >= 2 { 50.0 } else { 0.0 };

    return piece_value + pst_value + bishop_pair_bonus;
}

fn evaluate_pawn_structure(board: &Board, color: Color) -> f32 {
    let mut score = 0.0;
    let pawns = board.pieces(Piece::Pawn) & board.color_combined(color);

    // File occupancy
    let mut files = [0; 8];
    for sq in pawns {
        let file = sq.get_file() as usize;
        files[file] += 1;
    }

    // check isolated pawns and doubled+ pawns
    for file in 0..8 {
        match files[file] {
            0 => continue,
            1 => {
                // Check for isolated pawns in the same loop
                let isolated = if file == 0 {
                    files[file + 1] == 0
                } else if file == 7 {
                    files[file - 1] == 0
                } else {
                    files[file - 1] == 0 && files[file + 1] == 0
                };
                if isolated {
                    score -= 30.0;
                }
            }
            2 => score -= 20.0, // doubled pawns
            _ => score -= 40.0, // tripled or more
        }
    }

    score
}
