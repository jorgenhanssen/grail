use crate::utils::values::{BISHOP_VALUE, KNIGHT_VALUE, PAWN_VALUE, QUEEN_VALUE, ROOK_VALUE};
use chess::{BitBoard, Board, BoardStatus, Color, MoveGen, Piece, EMPTY};

use super::{
    get_pst, piece_value, sum_pst, CHECKMATE_SCORE, KNIGHT_OUTPOST_BONUS, PASSED_PAWN_BONUS,
    ROOK_ON_SEVENTH_BONUS, ROOK_OPEN_FILE_BONUS, ROOK_SEMI_OPEN_FILE_BONUS,
};

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

    let white_mask = board.color_combined(Color::White);
    let black_mask = board.color_combined(Color::Black);

    let mut score = 0.0;
    score += evaluate_material(board, Color::White, &white_mask);
    score -= evaluate_material(board, Color::Black, &black_mask);

    score += evaluate_pawn_structure(board, &white_mask);
    score -= evaluate_pawn_structure(board, &black_mask);

    score += evaluate_mobility(board, Color::White);
    score -= evaluate_mobility(board, Color::Black);

    score += evaluate_rooks(board, Color::White);
    score -= evaluate_rooks(board, Color::Black);

    score += evaluate_knights(board, Color::White);
    score -= evaluate_knights(board, Color::Black);

    score += evaluate_king_safety(board, Color::White);
    score -= evaluate_king_safety(board, Color::Black);

    score
}

fn evaluate_material(board: &Board, color: Color, color_mask: &BitBoard) -> f32 {
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

fn evaluate_pawn_structure(board: &Board, color_mask: &BitBoard) -> f32 {
    let mut score = 0.0;
    let pawns = board.pieces(Piece::Pawn) & color_mask;
    let color = if color_mask == board.color_combined(Color::White) {
        Color::White
    } else {
        Color::Black
    };

    // double, tripled, isolated
    let mut files = [0; 8];
    for sq in pawns {
        let file = sq.get_file() as usize;
        files[file] += 1;
    }

    for file in 0..8 {
        match files[file] {
            0 => continue,
            1 => {
                // Check for isolated pawns
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

    // For each pawn, check if it’s "passed".
    let enemy_pawns = board.pieces(Piece::Pawn) & board.color_combined(!color);

    for sq in pawns {
        if is_passed_pawn(sq, color, enemy_pawns) {
            // Rank from White's perspective is sq.get_rank(), from 0..7 (White=bottom).
            // For Black we might invert it: rank = 7 - sq.get_rank().
            let rank = sq.get_rank() as usize;
            let rank_from_white_persp = if color == Color::White {
                rank
            } else {
                7 - rank
            };
            score += PASSED_PAWN_BONUS[rank_from_white_persp];
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
    let mut zones = [EMPTY; 64];
    let mut i = 0;
    while i < 64 {
        let king_file = (i % 8) as i8;
        let king_rank = (i / 8) as i8;

        let mut zone = EMPTY;
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

fn is_passed_pawn(pawn_square: chess::Square, color: Color, enemy_pawns: BitBoard) -> bool {
    // Identify all squares in front of this pawn (including same file,
    // and adjacent files) up until it reaches the last rank.
    // If no enemy pawns exist in that region, it’s passed.

    let file = pawn_square.get_file() as i8;
    let rank = pawn_square.get_rank() as i8;

    // For White, check ranks above ‘rank’, for Black, ranks below.
    let direction = if color == Color::White { 1 } else { -1 };

    for r in (rank + direction..=7)
        .step_by(1)
        .take_while(|&r| r >= 0 && r < 8)
    {
        for f in (file - 1)..=(file + 1) {
            if f < 0 || f > 7 {
                continue;
            }
            let sq_index = (r * 8 + f) as u8;
            let sq_bb = BitBoard(1 << sq_index);
            if (sq_bb & enemy_pawns).popcnt() > 0 {
                return false;
            }
        }
    }
    true
}

fn evaluate_rooks(board: &Board, color: Color) -> f32 {
    let mut score = 0.0;
    let rooks = board.pieces(Piece::Rook) & board.color_combined(color);

    let all_pawns = board.pieces(Piece::Pawn);
    let our_pawns = all_pawns & board.color_combined(color);
    let their_pawns = all_pawns & board.color_combined(!color);

    for sq in rooks {
        let file = sq.get_file() as usize;

        // Count how many pawns are on this file for each side.
        let mut our_file_pawns = 0;
        let mut their_file_pawns = 0;
        for rank in 0..8 {
            let sq_index = rank * 8 + file;
            let sq_bb = BitBoard(1 << sq_index);
            if (sq_bb & our_pawns).popcnt() > 0 {
                our_file_pawns += 1;
            }
            if (sq_bb & their_pawns).popcnt() > 0 {
                their_file_pawns += 1;
            }
        }

        if our_file_pawns == 0 && their_file_pawns == 0 {
            // fully open file
            score += ROOK_OPEN_FILE_BONUS;
        } else if our_file_pawns == 0 && their_file_pawns > 0 {
            // semi-open file
            score += ROOK_SEMI_OPEN_FILE_BONUS;
        }

        // Rook on seventh (or second for Black) rank
        let rank = sq.get_rank() as u8;
        if (color == Color::White && rank == 6) || (color == Color::Black && rank == 1) {
            score += ROOK_ON_SEVENTH_BONUS;
        }
    }

    score
}

fn evaluate_knights(board: &Board, color: Color) -> f32 {
    let mut score = 0.0;
    let knights = board.pieces(Piece::Knight) & board.color_combined(color);
    let our_pawns = board.pieces(Piece::Pawn) & board.color_combined(color);
    let their_pawns = board.pieces(Piece::Pawn) & board.color_combined(!color);

    for sq in knights {
        // We want to see if the square is protected by our pawn
        // and cannot be attacked by an enemy pawn.

        let file = sq.get_file() as i8;
        let rank = sq.get_rank() as i8;

        // Check squares from which an enemy pawn could attack this knight’s square.
        // For White pawns, those squares are one rank down and file +/- 1.
        // For Black pawns, one rank up, file +/- 1.
        // We’ll see if there's an actual enemy pawn on those squares.

        let (attack_rank, enemy_color) = if color == Color::White {
            (rank - 1, Color::Black)
        } else {
            (rank + 1, Color::White)
        };
        let can_be_attacked_by_pawn = {
            if attack_rank >= 0 && attack_rank < 8 {
                let left_file = file - 1;
                let right_file = file + 1;
                let mut attack_squares = vec![];
                if left_file >= 0 && left_file < 8 {
                    attack_squares.push((attack_rank, left_file));
                }
                if right_file >= 0 && right_file < 8 {
                    attack_squares.push((attack_rank, right_file));
                }
                attack_squares.iter().any(|&(r, f)| {
                    let sq_index = (r as u8) * 8 + (f as u8);
                    let sq_bb = BitBoard(1 << sq_index);
                    (sq_bb & their_pawns).popcnt() > 0
                })
            } else {
                false
            }
        };

        // Check if the knight’s square is defended by your own pawns.
        // For White pawns, those squares are one rank up, file +/- 1.
        // For Black pawns, one rank down, file +/- 1.
        // This is simplistic, but enough for demonstration.

        let defend_rank = if color == Color::White {
            rank + 1
        } else {
            rank - 1
        };
        let is_defended_by_own_pawn = if defend_rank >= 0 && defend_rank < 8 {
            let left_file = file - 1;
            let right_file = file + 1;
            let mut defend_squares = vec![];
            if left_file >= 0 && left_file < 8 {
                defend_squares.push((defend_rank, left_file));
            }
            if right_file >= 0 && right_file < 8 {
                defend_squares.push((defend_rank, right_file));
            }
            defend_squares.iter().any(|&(r, f)| {
                let sq_index = (r as u8) * 8 + (f as u8);
                let sq_bb = BitBoard(1 << sq_index);
                (sq_bb & our_pawns).popcnt() > 0
            })
        } else {
            false
        };

        // If the knight cannot be attacked by a pawn AND is defended by your pawn,
        // give it an outpost bonus.
        if !can_be_attacked_by_pawn && is_defended_by_own_pawn {
            score += KNIGHT_OUTPOST_BONUS;
        }
    }

    score
}
