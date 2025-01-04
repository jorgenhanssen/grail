use crate::utils::values::{BISHOP_VALUE, KNIGHT_VALUE, PAWN_VALUE, QUEEN_VALUE, ROOK_VALUE};
use chess::{BitBoard, Board, BoardStatus, Color, File, MoveGen, Piece, Rank, Square, EMPTY};

use super::{get_pst, piece_value, sum_pst, CHECKMATE_SCORE};

// Return final evaluation (positive = good for White, negative = good for Black)
pub fn evaluate_board(board: &Board) -> f32 {
    let is_white = board.side_to_move() == Color::White;

    match board.status() {
        BoardStatus::Checkmate => {
            // If it’s White to move and board is checkmated => White lost
            if is_white {
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

    score += evaluate_mobility(board, Color::White);
    score -= evaluate_mobility(board, Color::Black);

    score += evaluate_king_safety(board, Color::White);
    score -= evaluate_king_safety(board, Color::Black);

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

#[inline]
fn evaluate_mobility(board: &Board, color: Color) -> f32 {
    if board.side_to_move() == color {
        return MoveGen::new_legal(&board).count() as f32;
    }

    // Movegen will return None if the current board has a check.
    // However, the quiescence search implementation will never allow evaluation
    // of a board with a check, so Some(board) will always be returned.
    if let Some(board) = board.null_move() {
        return MoveGen::new_legal(&board).count() as f32;
    }

    0.0
}

fn evaluate_king_safety(board: &Board, color: Color) -> f32 {
    let mut score = 0.0;
    let king_square = board.king_square(color);
    let king_zone = KING_ZONES[king_square.to_index()];
    let enemy_color = !color;
    let enemy_pieces = board.color_combined(enemy_color);

    // Weight threats by piece type
    let enemy_queens = enemy_pieces & board.pieces(Piece::Queen) & king_zone;
    let enemy_rooks = enemy_pieces & board.pieces(Piece::Rook) & king_zone;
    let enemy_bishops = enemy_pieces & board.pieces(Piece::Bishop) & king_zone;
    let enemy_knights = enemy_pieces & board.pieces(Piece::Knight) & king_zone;
    let enemy_pawns = enemy_pieces & board.pieces(Piece::Pawn) & king_zone;

    // Apply different penalties based on piece type
    score -= (enemy_queens.popcnt() as f32) * piece_value(Piece::Queen);
    score -= (enemy_rooks.popcnt() as f32) * piece_value(Piece::Rook);
    score -= (enemy_bishops.popcnt() as f32) * piece_value(Piece::Bishop);
    score -= (enemy_knights.popcnt() as f32) * piece_value(Piece::Knight);
    score -= (enemy_pawns.popcnt() as f32) * piece_value(Piece::Pawn);

    score * 0.3
}

const KING_ZONE_RADIUS: i8 = 2;
const KING_ZONES: [BitBoard; 64] = {
    let mut zones = [BitBoard(0); 64];
    let mut i = 0;
    while i < 64 {
        let king_file = (i % 8) as i8;
        let king_rank = (i / 8) as i8;

        let mut zone = BitBoard(0);
        let mut rank_offset = -KING_ZONE_RADIUS;
        while rank_offset <= KING_ZONE_RADIUS {
            let mut file_offset = -KING_ZONE_RADIUS;
            while file_offset <= KING_ZONE_RADIUS {
                let new_file = king_file + file_offset;
                let new_rank = king_rank + rank_offset;

                if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                    zone = BitBoard(zone.0 | (1u64 << (new_rank * 8 + new_file) as u64));
                }
                file_offset += 1;
            }
            rank_offset += 1;
        }
        zones[i] = zone;
        i += 1;
    }
    zones
};
